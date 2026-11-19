//! Release notes for Settings → About.
//!
//! The installed version and older ones come from the bundled `CHANGELOG.md`,
//! which `build.rs` validates. Newer versions come from the `changelog` field
//! of the update manifest, so they are known as soon as a check finds an
//! update, before its archive downloads.

pub(crate) mod parse;

use parse::{Release, Style};
use semver::Version;
use serde::Serialize;
use std::sync::LazyLock;

static BUNDLED: LazyLock<Vec<Release>> = LazyLock::new(|| {
    parse::parse(include_str!("../../CHANGELOG.md")).expect("build.rs validates CHANGELOG.md")
});

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ChangelogStyle {
    Plain,
    Code,
    Strong,
}

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChangelogSpan {
    pub(crate) text: String,
    pub(crate) style: ChangelogStyle,
}

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChangelogEntry {
    pub(crate) version: String,
    pub(crate) label: Option<String>,
    pub(crate) items: Vec<Vec<ChangelogSpan>>,
    /// The installed version.
    pub(crate) current: bool,
    /// The newest version the last update check found.
    pub(crate) latest: bool,
}

/// Newest first: bundled releases up to `current`, then manifest releases
/// after `current` up to `latest`.
pub(crate) fn entries(
    current: &Version,
    latest: Option<&Version>,
    remote: &[Release],
) -> Vec<ChangelogEntry> {
    entries_from(&BUNDLED, current, latest, remote)
}

fn entries_from(
    bundled: &[Release],
    current: &Version,
    latest: Option<&Version>,
    remote: &[Release],
) -> Vec<ChangelogEntry> {
    let mut selected: Vec<(Version, &Release)> = bundled
        .iter()
        .filter_map(|release| Some((Version::parse(&release.version).ok()?, release)))
        .filter(|(version, _)| version <= current)
        .collect();
    if let Some(latest) = latest {
        for release in remote {
            let Ok(version) = Version::parse(&release.version) else {
                continue;
            };
            if &version > current
                && &version <= latest
                && !selected.iter().any(|(known, _)| known == &version)
            {
                selected.push((version, release));
            }
        }
    }
    selected.sort_by(|(left, _), (right, _)| right.cmp(left));
    selected
        .into_iter()
        .map(|(version, release)| ChangelogEntry {
            version: release.version.clone(),
            label: release.label.clone(),
            items: release
                .items
                .iter()
                .map(|item| {
                    item.iter()
                        .map(|span| ChangelogSpan {
                            text: span.text.clone(),
                            style: match span.style {
                                Style::Plain => ChangelogStyle::Plain,
                                Style::Code => ChangelogStyle::Code,
                                Style::Strong => ChangelogStyle::Strong,
                            },
                        })
                        .collect()
                })
                .collect(),
            current: &version == current,
            latest: latest.is_some_and(|latest| &version == latest),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::parse::{parse, Span, Style};
    use super::*;

    #[test]
    fn parses_the_documented_format_and_rejects_anything_else() {
        let releases = parse(
            "# Changelog\n\n## 1.1.0-rc.2\n\n- Use `make release` for **signed** builds.\n\n## 1.0.0 — First\n\n- One.\n- Two.\n",
        )
        .unwrap();
        assert_eq!(releases.len(), 2);
        assert_eq!(releases[1].label.as_deref(), Some("First"));
        assert_eq!(
            releases[0].items[0],
            vec![
                Span {
                    text: "Use ".into(),
                    style: Style::Plain
                },
                Span {
                    text: "make release".into(),
                    style: Style::Code
                },
                Span {
                    text: " for ".into(),
                    style: Style::Plain
                },
                Span {
                    text: "signed".into(),
                    style: Style::Strong
                },
                Span {
                    text: " builds.".into(),
                    style: Style::Plain
                },
            ]
        );

        for (source, error) in [
            ("## 1.0.0\n\n- One.\n", "line 1"),
            ("# Changelog\n\n## v1.0\n\n- One.\n", "not a version"),
            ("# Changelog\n\n## 1.0.0\n\nProse.\n", "line 5"),
            ("# Changelog\n\n## 1.0.0\n\n- Open `code\n", "unclosed `code`"),
            ("# Changelog\n\n## 1.0.0\n\n- **Open\n", "unclosed **bold**"),
            ("# Changelog\n\n## 1.0.0\n\n## 0.9.0\n\n- One.\n", "1.0.0 has no items"),
            ("# Changelog\n\n## 1.0.0\n\n- One.\n\n## 1.1.0\n\n- Two.\n", "must be older"),
            ("# Changelog\n\n## 1.0.0\n\n- One.\n\n## 1.0.0-rc.10\n\n- Two.\n\n## 1.0.0-rc.9\n\n- Three.\n", ""),
        ] {
            let result = parse(source);
            if error.is_empty() {
                assert!(result.is_ok(), "{source:?}: {result:?}");
            } else {
                let message = result.expect_err(source);
                assert!(message.contains(error), "{source:?}: {message}");
            }
        }
    }

    #[test]
    fn merges_installed_history_with_newer_manifest_releases() {
        let bundled =
            parse("# Changelog\n\n## 1.1.0\n\n- Unreleased.\n\n## 1.0.0\n\n- Installed.\n")
                .unwrap();
        let remote = parse(
            "# Changelog\n\n## 2.0.0\n\n- Next major.\n\n## 1.2.0\n\n- Latest.\n\n## 1.1.0\n\n- Skipped.\n\n## 1.0.0\n\n- Installed.\n",
        )
        .unwrap();
        let current = Version::new(1, 0, 0);
        let latest = Version::new(1, 2, 0);

        let entries = entries_from(&bundled, &current, Some(&latest), &remote);
        let summary: Vec<_> = entries
            .iter()
            .map(|entry| {
                (
                    entry.version.as_str(),
                    entry.current,
                    entry.latest,
                    entry.items[0][0].text.as_str(),
                )
            })
            .collect();
        assert_eq!(
            summary,
            [
                ("1.2.0", false, true, "Latest."),
                ("1.1.0", false, false, "Skipped."),
                ("1.0.0", true, false, "Installed."),
            ]
        );

        let offline = entries_from(&bundled, &current, None, &remote);
        assert_eq!(offline.len(), 1);
        assert!(offline[0].current && !offline[0].latest);
    }
}
