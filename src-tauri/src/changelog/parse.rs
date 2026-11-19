//! Parser for `CHANGELOG.md`, shared by the app and `build.rs`.
//!
//! It has no crate dependencies so the build script can include it and fail
//! every build on a malformed changelog. The accepted format is:
//!
//! ```text
//! # Changelog
//!
//! ## 1.1.0
//!
//! - An item with `code` and **bold** text.
//!
//! ## 1.0.0 — Optional label
//!
//! - Another item.
//! ```
//!
//! Versions are `MAJOR.MINOR.PATCH` with an optional prerelease suffix, listed
//! newest first without duplicates. Each version has at least one item. Any
//! other line is an error.

use std::cmp::Ordering;

const TITLE: &str = "# Changelog";
const LABEL_SEPARATOR: &str = " — ";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Style {
    Plain,
    Code,
    Strong,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Span {
    pub(crate) text: String,
    pub(crate) style: Style,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Release {
    pub(crate) version: String,
    pub(crate) label: Option<String>,
    pub(crate) items: Vec<Vec<Span>>,
}

pub(crate) fn parse(source: &str) -> Result<Vec<Release>, String> {
    let mut lines = source
        .lines()
        .enumerate()
        .map(|(index, line)| (index + 1, line));
    match lines.next() {
        Some((_, TITLE)) => {}
        _ => return Err(format!("line 1: expected `{TITLE}`")),
    }
    let mut releases: Vec<Release> = Vec::new();
    for (number, line) in lines {
        if line.trim().is_empty() {
            continue;
        }
        if let Some(heading) = line.strip_prefix("## ") {
            let (version, label) = match heading.split_once(LABEL_SEPARATOR) {
                Some((version, label)) if !label.trim().is_empty() => {
                    (version, Some(label.trim().to_string()))
                }
                Some(_) => return Err(format!("line {number}: empty version label")),
                None => (heading, None),
            };
            let parsed = Version::parse(version)
                .ok_or_else(|| format!("line {number}: `{version}` is not a version"))?;
            if let Some(previous) = releases.last() {
                if previous.items.is_empty() {
                    return Err(format!("{} has no items", previous.version));
                }
                let previous = Version::parse(&previous.version).expect("validated version");
                if parsed.cmp(&previous) != Ordering::Less {
                    return Err(format!(
                        "line {number}: {version} must be older than the version above it"
                    ));
                }
            }
            releases.push(Release {
                version: version.to_string(),
                label,
                items: Vec::new(),
            });
        } else if let Some(item) = line.strip_prefix("- ") {
            let release = releases
                .last_mut()
                .ok_or_else(|| format!("line {number}: item before the first version"))?;
            let spans = parse_inline(item).map_err(|error| format!("line {number}: {error}"))?;
            release.items.push(spans);
        } else {
            return Err(format!(
                "line {number}: expected `## <version>`, `- <item>` or a blank line"
            ));
        }
    }
    match releases.last() {
        None => Err("no versions".into()),
        Some(release) if release.items.is_empty() => {
            Err(format!("{} has no items", release.version))
        }
        Some(_) => Ok(releases),
    }
}

/// Splits item text into plain, `code` and **strong** spans.
fn parse_inline(text: &str) -> Result<Vec<Span>, String> {
    if text.trim().is_empty() {
        return Err("empty item".into());
    }
    let mut spans = Vec::new();
    let mut rest = text;
    let mut strong = false;
    while !rest.is_empty() {
        let next = [rest.find('`'), rest.find("**")]
            .into_iter()
            .flatten()
            .min()
            .unwrap_or(rest.len());
        push(
            &mut spans,
            &rest[..next],
            if strong { Style::Strong } else { Style::Plain },
        );
        rest = &rest[next..];
        if let Some(code) = rest.strip_prefix('`') {
            let end = code.find('`').ok_or("unclosed `code`")?;
            if end == 0 {
                return Err("empty `code`".into());
            }
            push(&mut spans, &code[..end], Style::Code);
            rest = &code[end + 1..];
        } else if let Some(after) = rest.strip_prefix("**") {
            strong = !strong;
            rest = after;
        }
    }
    if strong {
        return Err("unclosed **bold**".into());
    }
    Ok(spans)
}

fn push(spans: &mut Vec<Span>, text: &str, style: Style) {
    if !text.is_empty() {
        spans.push(Span {
            text: text.to_string(),
            style,
        });
    }
}

/// A semantic version, compared by the rules of semver precedence.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Version<'a> {
    core: [u64; 3],
    prerelease: Option<&'a str>,
}

impl<'a> Version<'a> {
    pub(crate) fn parse(text: &'a str) -> Option<Self> {
        let (core, prerelease) = match text.split_once('-') {
            Some((core, prerelease)) => (core, Some(prerelease)),
            None => (text, None),
        };
        let mut parts = core.split('.');
        let mut numbers = [0; 3];
        for number in &mut numbers {
            let part = parts.next()?;
            if part.is_empty()
                || !part.bytes().all(|byte| byte.is_ascii_digit())
                || (part.len() > 1 && part.starts_with('0'))
            {
                return None;
            }
            *number = part.parse().ok()?;
        }
        if parts.next().is_some() {
            return None;
        }
        if let Some(prerelease) = prerelease {
            if prerelease.split('.').any(|identifier| {
                identifier.is_empty()
                    || !identifier
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            }) {
                return None;
            }
        }
        Some(Self {
            core: numbers,
            prerelease,
        })
    }
}

impl Ord for Version<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.core
            .cmp(&other.core)
            .then_with(|| match (self.prerelease, other.prerelease) {
                (None, None) => Ordering::Equal,
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (Some(left), Some(right)) => {
                    let mut left = left.split('.');
                    let mut right = right.split('.');
                    loop {
                        match (left.next(), right.next()) {
                            (None, None) => return Ordering::Equal,
                            (None, Some(_)) => return Ordering::Less,
                            (Some(_), None) => return Ordering::Greater,
                            (Some(left), Some(right)) => {
                                let order = match (left.parse::<u64>(), right.parse::<u64>()) {
                                    (Ok(left), Ok(right)) => left.cmp(&right),
                                    (Ok(_), Err(_)) => Ordering::Less,
                                    (Err(_), Ok(_)) => Ordering::Greater,
                                    (Err(_), Err(_)) => left.cmp(right),
                                };
                                if order != Ordering::Equal {
                                    return order;
                                }
                            }
                        }
                    }
                }
            })
    }
}

impl PartialOrd for Version<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
