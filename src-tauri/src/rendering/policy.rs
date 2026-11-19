//! The explicit HTML subset accepted at the rendering boundary.

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use ammonia::{Builder, UrlRelative};
use base64::Engine as _;

use super::styles;

pub(crate) const MAX_PLAIN_TEXT_BYTES: usize = 512 * 1024;

const STRUCTURE_TAGS: &[&str] = &[
    "blockquote",
    "br",
    "caption",
    "dd",
    "div",
    "dl",
    "dt",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "hr",
    "li",
    "ol",
    "p",
    "pre",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "tr",
    "ul",
];
const INLINE_TAGS: &[&str] = &[
    "a", "b", "code", "del", "em", "i", "q", "s", "samp", "span", "strike", "strong", "sub", "sup",
    "u",
];
const ACTIVE_CONTENT_TAGS: &[&str] = &[
    "audio",
    "canvas",
    "iframe",
    "noembed",
    "noscript",
    "object",
    "option",
    "plaintext",
    "script",
    "select",
    "style",
    "svg",
    "template",
    "textarea",
    "title",
    "video",
    "xmp",
];
const REMOVED_ELEMENT_TAGS: &[&str] = &[
    "audio",
    "body",
    "button",
    "embed",
    "form",
    "head",
    "html",
    "input",
    "link",
    "meta",
    "option",
    "noembed",
    "plaintext",
    "select",
    "source",
    "svg",
    "textarea",
    "track",
    "video",
    "xmp",
];

pub(crate) fn is_known_element(name: &str) -> bool {
    STRUCTURE_TAGS
        .iter()
        .chain(INLINE_TAGS)
        .chain(ACTIVE_CONTENT_TAGS)
        .chain(REMOVED_ELEMENT_TAGS)
        .chain(["img"].iter())
        .any(|tag| tag.eq_ignore_ascii_case(name))
}

pub(crate) fn sanitizer() -> Builder<'static> {
    sanitizer_builder(false, false)
}

/// Preserve only the split pipeline's app-owned references and safe presentation.
pub(super) fn reference_sanitizer() -> Builder<'static> {
    sanitizer_builder(false, true)
}

/// Sanitizer for history previews. It keeps only app-generated PNG data URLs.
pub(crate) fn preview_sanitizer() -> Builder<'static> {
    sanitizer_builder(true, false)
}

fn sanitizer_builder(
    preserve_generated_images: bool,
    preserve_references: bool,
) -> Builder<'static> {
    let allowed_tags = STRUCTURE_TAGS
        .iter()
        .chain(INLINE_TAGS)
        .chain(["img"].iter())
        .copied()
        .collect::<HashSet<_>>();
    let image_attributes = if preserve_references {
        HashSet::from(["data-isoboard-image-ref", "alt", "title", "width", "height"])
    } else if preserve_generated_images {
        HashSet::from(["data-x1", "data-y1", "data-x2", "data-y2", "src"])
    } else {
        HashSet::from(["data-x1", "data-y1", "data-x2", "data-y2"])
    };
    let tag_attributes = HashMap::from([
        ("a", HashSet::from(["href"])),
        ("img", image_attributes),
        ("td", HashSet::from(["colspan", "rowspan"])),
        ("th", HashSet::from(["colspan", "rowspan"])),
        ("table", HashSet::from(["border"])),
    ]);
    let mut builder = Builder::default();
    builder
        .tags(allowed_tags)
        .tag_attributes(tag_attributes)
        .generic_attributes(HashSet::from(["style"]))
        .clean_content_tags(ACTIVE_CONTENT_TAGS.iter().copied().collect())
        .url_schemes(if preserve_generated_images {
            HashSet::from(["http", "https", "mailto", "data"])
        } else {
            HashSet::from(["http", "https", "mailto"])
        })
        .url_relative(UrlRelative::Deny)
        .link_rel(None)
        .strip_comments(true)
        .attribute_filter(
            move |element, attribute, value| match (element, attribute) {
                ("img", "style") if !preserve_references => None,
                ("img", "width" | "height") if preserve_references => {
                    valid_positive_integer(value, 10_000).map(Cow::Owned)
                }
                ("img", "src") if preserve_generated_images => {
                    is_generated_png_url(value).then_some(Cow::Borrowed(value))
                }
                ("img", "src") => None,
                ("a", "href") => {
                    let safe = value.starts_with("http://")
                        || value.starts_with("https://")
                        || value.starts_with("mailto:");
                    safe.then_some(Cow::Borrowed(value))
                }
                (_, "style") => styles::sanitize(value),
                ("td" | "th", "colspan" | "rowspan") => {
                    valid_positive_integer(value, 1000).map(Cow::Owned)
                }
                ("table", "border") => valid_positive_integer(value, 16).map(Cow::Owned),
                _ => Some(Cow::Borrowed(value)),
            },
        );
    builder
}

pub(super) fn is_generated_png_url(value: &str) -> bool {
    let Some(encoded) = value.strip_prefix("data:image/png;base64,") else {
        return false;
    };
    encoded.len() <= 700 * 1024
        && !encoded.is_empty()
        && base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .is_ok()
}

fn valid_positive_integer(value: &str, max: u32) -> Option<String> {
    let number = value.parse::<u32>().ok()?;
    (number > 0 && number <= max).then_some(number.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_safe_structure_style_and_merged_cells() {
        let html = sanitizer().clean(r#"<table border="1"><tr><td colspan="2" rowspan="1" style="color: #123456; text-align: center">x</td></tr></table>"#).to_string();
        assert_eq!(
            html,
            r#"<table border="1"><tbody><tr><td colspan="2" rowspan="1" style="color: #123456; text-align: center">x</td></tr></tbody></table>"#
        );
    }

    #[test]
    fn preserves_prompt_compliant_inline_wrappers_and_styles() {
        let html = sanitizer()
            .clean(r#"<span style="color: navy; background-color: lightblue; font-size: large">text</span><b>bold</b><strike>old</strike>"#)
            .to_string();
        assert!(html.contains(r#"<span style="background-color: lightblue; color: navy; font-size: large">text</span>"#));
        assert!(html.contains("<b>bold</b>"));
        assert!(html.contains("<strike>old</strike>"));
    }

    #[test]
    fn preserves_bounded_table_border_and_rejects_invalid_values() {
        assert!(sanitizer()
            .clean(r#"<table border="16"><tr><td>x</td></tr></table>"#)
            .to_string()
            .contains(r#"border="16""#));
        assert!(!sanitizer()
            .clean(r#"<table border="17"><tr><td>x</td></tr></table>"#)
            .to_string()
            .contains("border="));
        assert!(!sanitizer()
            .clean(r#"<table border="0"><tr><td>x</td></tr></table>"#)
            .to_string()
            .contains("border="));
    }

    #[test]
    fn removes_provider_image_attributes_but_keeps_markers() {
        let html = sanitizer().clean(r#"<img src="https://bad" alt="bad" class="bad" style="color:red" data-x1="0" data-y1="0" data-x2="1" data-y2="1">"#).to_string();
        for attribute in [
            r#"data-x1="0""#,
            r#"data-y1="0""#,
            r#"data-x2="1""#,
            r#"data-y2="1""#,
        ] {
            assert!(html.contains(attribute));
        }
        assert!(!html.contains("src=") && !html.contains("alt=") && !html.contains("style="));
    }

    #[test]
    fn preview_sanitizer_keeps_only_generated_png_data_urls() {
        let html = preview_sanitizer()
            .clean(
                r#"<p><img src="data:image/png;base64,iVBORw0KGgpmaXh0dXJl"><img src="data:image/svg+xml;base64,unsafe"><a href="data:text/html,unsafe">bad</a><a href="https://example.com">good</a></p>"#,
            )
            .to_string();
        assert!(html.contains("data:image/png;base64,iVBORw0KGgpmaXh0dXJl"));
        assert!(!html.contains("data:image/svg+xml"));
        assert!(!html.contains("data:text/html"));
        assert!(html.contains("https://example.com"));
    }
}
