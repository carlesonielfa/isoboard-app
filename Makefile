SHELL := /bin/sh

BUN ?= bun
CARGO ?= cargo
MANIFEST := src-tauri/Cargo.toml
BINDINGS := src/lib/generated/bindings.ts

.PHONY: help install dev dev-reset-permissions frontend-dev desktop-dev frontend-check desktop-check check frontend-test desktop-test test frontend-build build desktop-build bundle beta release frontend-format desktop-format format frontend-format-check desktop-format-check format-check frontend-lint desktop-lint lint bindings bindings-check api-key-set api-key-status api-key-logout api-probe

help:
	@printf '%s\n' \
		'Frontend and desktop commands:' \
		'  make install                 Install Bun dependencies' \
		'  make dev                    Run the Tauri desktop app with frontend reloads' \
		'  make dev-reset-permissions  Reset dev privacy permissions and open notification settings' \
		'  make frontend-dev           Run the Vite frontend only' \
		'  make check                  Run frontend, Rust, and binding checks' \
		'  make test                   Run frontend and Rust tests' \
		'  make build                  Build the frontend' \
		'  make bundle                 Build the Tauri desktop bundle' \
		'  make beta APPLE_SIGNING_IDENTITY="<certificate>"  Build a signed test app and DMG' \
		'  make release                Build, notarize and verify the release app and DMG' \
		'  make format                Format frontend and Rust sources' \
		'  make format-check          Check frontend and Rust formatting' \
		'  make lint                  Run frontend lint and Rust clippy with warnings denied' \
		'  make bindings              Regenerate TypeScript bindings from Rust contracts' \
		'  make bindings-check        Regenerate bindings and fail when the committed file changes' \
		'  make api-key-set PROVIDER=<id>  Read one API key from stdin into Keychain' \
		'  make api-key-status PROVIDER=<id>  Check one API-key probe slot' \
		'  make api-key-logout PROVIDER=<id>  Remove one API-key probe slot' \
		'  make api-probe PROVIDER=<id> MODEL=<id> [ENDPOINT=<url>]  Probe one API provider'

install:
	$(BUN) install --frozen-lockfile

dev:
	$(MAKE) desktop-dev

frontend-dev:
	$(BUN) run dev

desktop-dev:
	ISOBOARD_CONFIG_PATH="$(CURDIR)/.dev/config.toml" \
	CARGO_TARGET_AARCH64_APPLE_DARWIN_RUNNER='$(BUN) $(CURDIR)/scripts/run-macos-dev.mjs' \
	CARGO_TARGET_X86_64_APPLE_DARWIN_RUNNER='$(BUN) $(CURDIR)/scripts/run-macos-dev.mjs' \
	$(BUN) run tauri dev --config "$$($(BUN) scripts/run-macos-dev.mjs --config)"

dev-reset-permissions:
	$(BUN) scripts/run-macos-dev.mjs --reset-permissions

frontend-check:
	$(BUN) run check

desktop-check:
	$(CARGO) check --manifest-path $(MANIFEST)

check: frontend-check desktop-check bindings-check

frontend-test:
	$(BUN) run test

desktop-test:
	$(CARGO) test --manifest-path $(MANIFEST)

test: frontend-test desktop-test

frontend-build:
	$(BUN) run build

build: frontend-build

desktop-build:
	$(BUN) run tauri build

bundle: desktop-build

beta: export APPLE_SIGNING_IDENTITY := $(APPLE_SIGNING_IDENTITY)
beta:
	@test -n "$$APPLE_SIGNING_IDENTITY" && test "$$APPLE_SIGNING_IDENTITY" != "-" || { \
		printf '%s\n' 'Set APPLE_SIGNING_IDENTITY to a stable signing certificate name; ad-hoc signing is not supported by make beta.' >&2; \
		exit 1; \
	}
	$(BUN) run tauri build --bundles app,dmg

release:
	./scripts/release-macos.sh

frontend-format:
	$(BUN) run format

desktop-format:
	$(CARGO) fmt --manifest-path $(MANIFEST)

format: frontend-format desktop-format

frontend-format-check:
	$(BUN) run format:check

desktop-format-check:
	$(CARGO) fmt --manifest-path $(MANIFEST) -- --check

format-check: frontend-format-check desktop-format-check

frontend-lint:
	$(BUN) run lint

desktop-lint:
	$(CARGO) clippy --manifest-path $(MANIFEST) --all-targets --all-features -- -D warnings

lint: frontend-lint desktop-lint

bindings:
	$(CARGO) run --manifest-path $(MANIFEST) --example export_bindings -- $(BINDINGS)

bindings-check:
	@temporary=$$(mktemp "$${TMPDIR:-/tmp}/isoboard-bindings.XXXXXX"); \
	trap 'rm -f "$$temporary"' EXIT INT TERM; \
	if ! $(CARGO) run --manifest-path $(MANIFEST) --example export_bindings -- "$$temporary"; then \
		exit 1; \
	fi; \
	if test ! -f "$(BINDINGS)"; then \
		echo "missing $(BINDINGS); run 'make bindings'" >&2; \
		exit 1; \
	fi; \
	if ! cmp -s "$(BINDINGS)" "$$temporary"; then \
		echo "generated bindings are stale; run 'make bindings'" >&2; \
	diff -u "$(BINDINGS)" "$$temporary" || true; \
	exit 1; \
	fi

api-key-set:
	@test -n "$(PROVIDER)" || (printf '%s\n' 'PROVIDER is required' >&2; exit 2)
	$(CARGO) run --manifest-path $(MANIFEST) --features api-probe --example api_credentials -- set "$(PROVIDER)"

api-key-status:
	@test -n "$(PROVIDER)" || (printf '%s\n' 'PROVIDER is required' >&2; exit 2)
	$(CARGO) run --manifest-path $(MANIFEST) --features api-probe --example api_credentials -- status "$(PROVIDER)"

api-key-logout:
	@test -n "$(PROVIDER)" || (printf '%s\n' 'PROVIDER is required' >&2; exit 2)
	$(CARGO) run --manifest-path $(MANIFEST) --features api-probe --example api_credentials -- logout "$(PROVIDER)"

api-probe:
	@test -n "$(PROVIDER)" || (printf '%s\n' 'PROVIDER is required' >&2; exit 2)
	@test -n "$(MODEL)" || (printf '%s\n' 'MODEL is required; no provider model default is selected' >&2; exit 2)
	$(CARGO) run --manifest-path $(MANIFEST) --features api-probe --example api_probe -- probe "$(PROVIDER)" "$(MODEL)" "$(ENDPOINT)"
