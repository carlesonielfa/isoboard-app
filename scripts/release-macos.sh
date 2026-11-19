#!/bin/sh
# Builds, signs, notarizes and verifies the macOS release app and DMG, and
# writes the signed update archive and its latest.json manifest.
#
# Requires a Developer ID Application identity in a keychain and notarization
# credentials in the environment, using either an App Store Connect API key
# (APPLE_API_KEY, APPLE_API_ISSUER, APPLE_API_KEY_PATH) or an Apple ID
# (APPLE_ID, APPLE_PASSWORD, APPLE_TEAM_ID). Tauri reads the same variables to
# notarize and staple the app; this script then signs, notarizes and staples
# the DMG that contains it.
#
# TAURI_SIGNING_PRIVATE_KEY (and TAURI_SIGNING_PRIVATE_KEY_PASSWORD when the
# key has one) signs the update archive for the public key in tauri.conf.json.
set -eu

cd "$(dirname "$0")/.."

fail() {
	printf 'release: %s\n' "$1" >&2
	exit 1
}

package_version=$(bun -e 'console.log(require("./package.json").version)')
tauri_version=$(bun -e 'console.log(require("./src-tauri/tauri.conf.json").version)')
cargo_version=$(sed -n 's/^version = "\(.*\)"$/\1/p' src-tauri/Cargo.toml | head -n 1)
[ "$package_version" = "$tauri_version" ] && [ "$package_version" = "$cargo_version" ] ||
	fail "versions differ: package.json $package_version, tauri.conf.json $tauri_version, Cargo.toml $cargo_version"
version=$package_version

if [ -n "${RELEASE_TAG:-}" ] && [ "$RELEASE_TAG" != "v$version" ]; then
	fail "tag $RELEASE_TAG does not match app version $version"
fi

[ -n "${APPLE_SIGNING_IDENTITY:-}" ] && [ "$APPLE_SIGNING_IDENTITY" != "-" ] ||
	fail 'set APPLE_SIGNING_IDENTITY to a Developer ID Application certificate'
security find-identity -v -p codesigning | grep -F "$APPLE_SIGNING_IDENTITY" | grep -q 'Developer ID Application' ||
	fail "no Developer ID Application identity matches APPLE_SIGNING_IDENTITY"

if [ -n "${APPLE_API_KEY:-}" ] && [ -n "${APPLE_API_ISSUER:-}" ] && [ -n "${APPLE_API_KEY_PATH:-}" ]; then
	[ -f "$APPLE_API_KEY_PATH" ] || fail "APPLE_API_KEY_PATH does not point to a file"
	set -- --key "$APPLE_API_KEY_PATH" --key-id "$APPLE_API_KEY" --issuer "$APPLE_API_ISSUER"
elif [ -n "${APPLE_ID:-}" ] && [ -n "${APPLE_PASSWORD:-}" ] && [ -n "${APPLE_TEAM_ID:-}" ]; then
	set -- --apple-id "$APPLE_ID" --password "$APPLE_PASSWORD" --team-id "$APPLE_TEAM_ID"
else
	fail 'set APPLE_API_KEY, APPLE_API_ISSUER and APPLE_API_KEY_PATH, or APPLE_ID, APPLE_PASSWORD and APPLE_TEAM_ID'
fi

[ -n "${TAURI_SIGNING_PRIVATE_KEY:-}" ] || fail 'set TAURI_SIGNING_PRIVATE_KEY to the updater signing key'

grep -Eq "^## $version( |\$)" CHANGELOG.md || fail "CHANGELOG.md has no section for $version"

bun run tauri build --bundles app,dmg --config '{"bundle":{"createUpdaterArtifacts":true}}'

bundle_dir=src-tauri/target/release/bundle
app="$bundle_dir/macos/isoboard.app"
dmg="$bundle_dir/dmg/isoboard_${version}_aarch64.dmg"
[ -d "$app" ] || fail "missing $app"
[ -f "$dmg" ] || fail "missing $dmg"
archive="$bundle_dir/macos/isoboard.app.tar.gz"
[ -f "$archive" ] && [ -f "$archive.sig" ] || fail "missing $archive or its signature"

codesign --force --sign "$APPLE_SIGNING_IDENTITY" --timestamp "$dmg"
xcrun notarytool submit "$dmg" "$@" --wait
xcrun stapler staple "$dmg"

codesign --verify --deep --strict --verbose=2 "$app"
# Screen Recording and Keychain grants follow the designated requirement, so
# it must name the bundle and the team rather than one build's code hash.
requirement=$(codesign --display --requirements - "$app" 2>&1)
printf '%s\n' "$requirement" | grep -q 'identifier "app.isoboard"' &&
	printf '%s\n' "$requirement" | grep -q 'certificate leaf\[subject.OU\]' ||
	fail "designated requirement is not stable across releases: $requirement"
xcrun stapler validate "$app"
spctl --assess --type execute --verbose=2 "$app"
xcrun stapler validate "$dmg"
spctl --assess --type open --context context:primary-signature --verbose=2 "$dmg"

# The archive is published as a release asset named after the version.
release_dir="$bundle_dir/release"
rm -rf "$release_dir"
mkdir -p "$release_dir"
asset="isoboard_${version}_aarch64.app.tar.gz"
cp "$dmg" "$release_dir/"
cp "$archive" "$release_dir/$asset"
# Installed apps read the manifest URL from services.jsonc; the archive is
# published next to it in the same repository's release.
manifest_url=$(sed -n 's/^ *"manifestUrl": "\(.*\)",$/\1/p' config/services.jsonc)
repository=${manifest_url%/releases/latest/download/latest.json}
[ -n "$manifest_url" ] && [ "$repository" != "$manifest_url" ] ||
	fail "services.jsonc manifestUrl must end in /releases/latest/download/latest.json"
# The full changelog lets installed apps show every newer version's notes
# before they download the update. build.rs has already validated its format.
CHANGELOG="$(cat CHANGELOG.md)" \
	REPOSITORY="$repository" VERSION="$version" ASSET="$asset" SIGNATURE="$(cat "$archive.sig")" \
	NOTES="$(awk -v version="$version" '
		$0 ~ "^## " version "( |$)" { found = 1; next }
		found && /^## / { exit }
		found { print }
	' CHANGELOG.md)" \
	bun -e '
		const { CHANGELOG, REPOSITORY, VERSION, ASSET, SIGNATURE, NOTES } = process.env;
		const manifest = {
			version: VERSION,
			notes: NOTES.trim(),
			changelog: CHANGELOG + "\n",
			pub_date: new Date().toISOString(),
			platforms: {
				"darwin-aarch64": {
					signature: SIGNATURE,
					url: `${REPOSITORY}/releases/download/v${VERSION}/${ASSET}`,
				},
			},
		};
		await Bun.write(process.argv[1], JSON.stringify(manifest, null, 2) + "\n");
	' "$release_dir/latest.json"

printf 'release: notarized %s; assets in %s\n' "$dmg" "$release_dir"
