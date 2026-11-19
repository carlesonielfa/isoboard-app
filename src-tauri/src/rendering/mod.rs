//! The provider-output rendering boundary.
//!
//! The inference prompt is the product contract. This module accepts the
//! document, body, or fragment that prompt requests, applies the explicit
//! safety policy, embeds only app-generated image crops, and produces both
//! representations needed by the app:
//!
//! * `document_html` is the canonical full document stored in app state.
//! * `body_html` is the sanitized body fragment passed to clipboard adapters.
//! * `plain_text` is derived from the final body after image embedding.

mod content;
mod conversion;
mod images;
mod plain_text;
pub(crate) mod policy;
mod saved_images;
mod split;
mod styles;

pub(crate) use content::{content_tags, ContentTag};
pub(crate) use plain_text::from_html as plain_text_from_html;
pub(crate) use split::{finalize_postprocessing, prepare_conversion, PreparedConversion};

use crate::profiles::{ConversionSettings, ConversionSettingsApplicationMode};

use std::fmt;

pub(crate) const MAX_PROVIDER_OUTPUT_BYTES: usize = 1024 * 1024;
pub(crate) const MAX_DOCUMENT_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Copy, Debug)]
pub(crate) struct SourceImage<'a> {
    pub(crate) bytes: &'a [u8],
    pub(crate) width: u32,
    pub(crate) height: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NormalizedHtml {
    pub(crate) document_html: String,
    pub(crate) formatting_body_html: Option<String>,
    pub(crate) body_html: String,
    pub(crate) plain_text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RenderingError {
    EmptyInput,
    EmptyOutput,
    ProviderOutputTooLarge,
    HtmlTooLarge,
    PlainTextTooLarge,
    PlainTextExtraction,
    InvalidImageMarker,
    ImageSource,
    ImageCountExceeded,
    EmbeddedImagesTooLarge,
    HtmlNestingTooDeep,
}

impl fmt::Display for RenderingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptyInput => "the provider returned no content",
            Self::EmptyOutput => "the provider content contains no usable output",
            Self::ProviderOutputTooLarge => "the provider output exceeds the size limit",
            Self::HtmlTooLarge => "the normalized HTML exceeds the size limit",
            Self::PlainTextTooLarge => "the derived plain text exceeds the size limit",
            Self::PlainTextExtraction => "plain text could not be derived from the normalized HTML",
            Self::InvalidImageMarker => "an image marker is invalid",
            Self::ImageSource => "the captured image could not be decoded for embedding",
            Self::ImageCountExceeded => "the provider returned too many image markers",
            Self::EmbeddedImagesTooLarge => "embedded image data exceeds the size limit",
            Self::HtmlNestingTooDeep => "the HTML nesting depth exceeds the safety limit",
        };
        formatter.write_str(message)
    }
}

pub(crate) fn reformat_saved(
    body: &str,
    settings: ConversionSettings,
) -> Result<NormalizedHtml, RenderingError> {
    if body.len() > MAX_DOCUMENT_BYTES {
        return Err(RenderingError::HtmlTooLarge);
    }
    let images = saved_images::CompactImages::new(body);
    let safe = policy::preview_sanitizer().clean(images.html()).to_string();
    let body_html = conversion::apply(
        &safe,
        settings,
        ConversionSettingsApplicationMode::HtmlCleanup,
    )?;
    let plain_text = plain_text::from_html(&body_html)?;
    if plain_text.trim().is_empty() && !body_html.contains("<img ") {
        return Err(RenderingError::EmptyOutput);
    }
    let body_html = images.restore(body_html);
    let document_html = format!("<html><body>{body_html}</body></html>");
    if document_html.len() > MAX_DOCUMENT_BYTES {
        return Err(RenderingError::HtmlTooLarge);
    }
    Ok(NormalizedHtml {
        formatting_body_html: None,
        document_html,
        body_html,
        plain_text,
    })
}

#[cfg(test)]
pub(crate) fn normalize(
    input: &str,
    source: SourceImage<'_>,
) -> Result<NormalizedHtml, RenderingError> {
    normalize_internal(input, source, ConversionSettings::default(), None)
}

/// Normalize provider HTML and apply deterministic conversion settings when
/// the selected application mode includes HTML cleanup. Prompt-only mode
/// intentionally delegates to the compatibility path and preserves all safe
/// model output, including image markers.
pub(crate) fn normalize_with_conversion(
    input: &str,
    source: SourceImage<'_>,
    settings: ConversionSettings,
    mode: ConversionSettingsApplicationMode,
) -> Result<NormalizedHtml, RenderingError> {
    normalize_internal(input, source, settings, Some(mode))
}

fn normalize_internal(
    input: &str,
    source: SourceImage<'_>,
    settings: ConversionSettings,
    mode: Option<ConversionSettingsApplicationMode>,
) -> Result<NormalizedHtml, RenderingError> {
    if input.len() > MAX_PROVIDER_OUTPUT_BYTES {
        return Err(RenderingError::ProviderOutputTooLarge);
    }
    let input = strip_outer_html_fence(input).trim();
    if input.is_empty() {
        return Err(RenderingError::EmptyInput);
    }
    let cleanup_images = mode.is_some_and(|mode| {
        matches!(
            mode,
            ConversionSettingsApplicationMode::HtmlCleanup
                | ConversionSettingsApplicationMode::PromptAndHtmlCleanup
        ) && !settings.images
    });
    if !cleanup_images {
        images::validate_markers(input)?;
    }

    let input = images::strip_removed_subtrees(input);
    let body = if contains_markup(&input) {
        policy::sanitizer().clean(&input).to_string()
    } else {
        format!(
            "<p>{}</p>",
            escape_text(&input)
                .replace("\r\n", "\n")
                .replace('\r', "\n")
                .replace('\n', "<br>")
        )
    };
    // Keep the safe model output with generated crops before feature cleanup.
    // Invalid markers ignored by images-off capture do not become a baseline.
    let formatting_body_html = images::embed_markers(body.trim(), source).ok();
    let body = if let Some(baseline) = &formatting_body_html {
        conversion::apply(
            baseline,
            settings.clone(),
            mode.unwrap_or(ConversionSettingsApplicationMode::Prompt),
        )?
    } else {
        let cleaned = conversion::apply(
            body.trim(),
            settings.clone(),
            mode.unwrap_or(ConversionSettingsApplicationMode::Prompt),
        )?;
        if cleanup_images {
            cleaned
        } else {
            images::embed_markers(cleaned.trim(), source)?
        }
    }
    .trim()
    .to_owned();
    if body.is_empty() {
        return Err(RenderingError::EmptyOutput);
    }

    let document_html = format!("<html><body>{body}</body></html>");
    if document_html.len() > MAX_DOCUMENT_BYTES {
        return Err(RenderingError::HtmlTooLarge);
    }

    let plain_text = plain_text::from_html(&body)?;
    if cleanup_images && plain_text.trim().is_empty() {
        return Err(RenderingError::EmptyOutput);
    }
    Ok(NormalizedHtml {
        formatting_body_html,
        document_html,
        body_html: body,
        plain_text,
    })
}

fn strip_outer_html_fence(input: &str) -> &str {
    let trimmed = input.trim();
    let Some(after_ticks) = trimmed.strip_prefix("```") else {
        return trimmed;
    };
    let Some(newline) = after_ticks.find('\n') else {
        return trimmed;
    };
    let language = after_ticks[..newline].trim();
    if !language.is_empty() && !language.eq_ignore_ascii_case("html") {
        return trimmed;
    }
    let body = &after_ticks[newline + 1..];
    let Some(body_without_close) = body.strip_suffix("```") else {
        return trimmed;
    };
    body_without_close.trim()
}

fn contains_markup(input: &str) -> bool {
    let bytes = input.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let Some(relative) = bytes[index..].iter().position(|byte| *byte == b'<') else {
            return false;
        };
        index += relative + 1;
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index < bytes.len() && matches!(bytes[index], b'!' | b'?') {
            return true;
        }
        if index < bytes.len() && bytes[index] == b'/' {
            index += 1;
        }
        let name_start = index;
        while index < bytes.len() && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'-')
        {
            index += 1;
        }
        if name_start == index {
            continue;
        }
        let name = &input[name_start..index];
        if policy::is_known_element(name) || has_closing_tag(input, name, index) {
            return true;
        }
    }
    false
}

fn has_closing_tag(input: &str, name: &str, start: usize) -> bool {
    let suffix = input.get(start..).unwrap_or_default().to_ascii_lowercase();
    let needle = format!("</{}", name.to_ascii_lowercase());
    let Some(close_start) = suffix.find(&needle) else {
        return false;
    };
    suffix[close_start + needle.len()..]
        .trim_start_matches(|character: char| character.is_ascii_whitespace())
        .starts_with('>')
}

fn escape_text(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    use image::{ImageBuffer, ImageFormat, Rgba};

    const PIXEL: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 248, 207, 192, 240,
        31, 0, 5, 0, 1, 255, 137, 153, 61, 29, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];

    fn source() -> SourceImage<'static> {
        SourceImage {
            bytes: PIXEL,
            width: 1,
            height: 1,
        }
    }

    fn source_32() -> (Vec<u8>, u32, u32) {
        source_sized(32, 32)
    }

    fn source_sized(width: u32, height: u32) -> (Vec<u8>, u32, u32) {
        let image = ImageBuffer::from_pixel(width, height, Rgba([255, 0, 0, 255]));
        let mut bytes = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut bytes, ImageFormat::Png)
            .unwrap();
        (bytes.into_inner(), width, height)
    }

    #[test]
    fn reasonable_small_images_survive_and_incidental_regions_are_omitted_in_both_pipelines() {
        let input = include_str!("../../fixtures/small-image-markers.html").replace(
            "</body>",
            r#"<p>Before<img data-x1="0.1" data-y1="0.1" data-x2="0.11" data-y2="0.11">After</p></body>"#,
        );
        for (width, height) in [(560, 584), (1120, 1168)] {
            let (bytes, _, _) = source_sized(width, height);
            let source = SourceImage {
                bytes: &bytes,
                width,
                height,
            };
            let direct = normalize(&input, source).unwrap();
            let prepared = prepare_conversion(&input, source).unwrap();
            assert_eq!(
                prepared
                    .intermediate_html()
                    .matches("data-isoboard-image")
                    .count(),
                3
            );
            let intermediate = prepared.intermediate_html().to_owned();
            let split = finalize_postprocessing(
                prepared,
                &intermediate,
                ConversionSettings::default(),
                ConversionSettingsApplicationMode::PromptAndHtmlCleanup,
            )
            .unwrap();
            for result in [direct, split] {
                assert_eq!(
                    result.body_html.matches("data:image/png;base64,").count(),
                    3
                );
                assert!(result.plain_text.contains("HIX.AI"));
                assert!(result.plain_text.contains("Before"));
                assert!(result.plain_text.contains("After"));
                assert!(!result.body_html.contains("data-x1"));
            }
        }
    }

    #[test]
    fn canonicalizes_full_document_body_and_fragment_inputs() {
        for input in [
            "<html><body><p>hello</p></body></html>",
            "<body><p>hello</p></body>",
            "<p>hello</p>",
        ] {
            assert_eq!(
                normalize(input, source()).unwrap().document_html,
                "<html><body><p>hello</p></body></html>"
            );
        }
    }

    #[test]
    fn derives_text_from_final_body() {
        let normalized = normalize("<p>hello <strong>world</strong></p>", source()).unwrap();
        assert_eq!(normalized.plain_text, "hello world");
    }

    #[test]
    fn preserves_prompt_wrappers_and_repairs_malformed_html() {
        let normalized = normalize(
            "<unknown><p>one<strong>two</p></unknown><span style=\"color: navy; line-height: 1.5\">three</span><b>four</b><strike>five</strike>",
            source(),
        )
        .unwrap();
        assert!(normalized
            .body_html
            .contains("<p>one<strong>two</strong></p>"));
        assert!(normalized
            .body_html
            .contains("<span style=\"color: navy; line-height: 1.5\">three</span>"));
        assert!(normalized
            .body_html
            .contains("<b>four</b><strike>five</strike>"));
    }

    #[test]
    fn handles_fences_links_resources_and_active_subtrees() {
        assert_eq!(
            normalize("```HTML\n<p>hello</p>\n```", source())
                .unwrap()
                .body_html,
            "<p>hello</p>"
        );
        assert_eq!(
            normalize("```text\nhello\n```", source())
                .unwrap()
                .body_html,
            "<p>```text<br>hello<br>```</p>"
        );
        let normalized = normalize(
            r#"<unknown><p>keep</p></unknown><script>active</script><iframe src="https://bad"></iframe><a href="javascript:bad">bad</a><a href="https://example.com">good</a>"#,
            source(),
        )
        .unwrap();
        assert!(normalized.body_html.contains("<p>keep</p>"));
        assert!(
            !normalized.body_html.contains("script")
                && !normalized.body_html.contains("javascript:")
        );
        assert!(normalized
            .body_html
            .contains(r#"href="https://example.com""#));
    }

    #[test]
    fn escapes_text_preserves_pre_whitespace_and_rejects_empty_output() {
        assert_eq!(
            normalize("unsafe & text", source()).unwrap().body_html,
            "<p>unsafe &amp; text</p>"
        );
        assert_eq!(
            normalize("<pre>  one\n  two</pre>", source())
                .unwrap()
                .body_html,
            "<pre>  one\n  two</pre>"
        );
        assert_eq!(
            normalize("<script>removed</script>", source()),
            Err(RenderingError::EmptyOutput)
        );
        assert_eq!(
            normalize("<status > pending & review", source())
                .unwrap()
                .body_html,
            "<p>&lt;status &gt; pending &amp; review</p>"
        );
        assert_eq!(normalize(" ", source()), Err(RenderingError::EmptyInput));
    }

    #[test]
    fn ignores_marker_like_text_inside_comments_and_attribute_values() {
        let normalized = normalize(
            r#"<p title="<img data-x1='0' data-y1='0' data-x2='1' data-y2='1'>">text</p><!-- <img data-x1="0" data-y1="0" data-x2="1" data-y2="1"> --><svg><img data-x1="0" data-y1="0" data-x2="1" data-y2="1"></svg>"#,
            source(),
        )
        .unwrap();
        assert_eq!(normalized.body_html, r#"<p>text</p>"#);
    }

    #[test]
    fn ignores_raw_text_and_nested_removed_elements() {
        let normalized = normalize(
            r#"<p>before</p><xmp><img data-x1="0" data-y1="0" data-x2="1" data-y2="1"></xmp><noembed><img data-x1="0" data-y1="0" data-x2="1" data-y2="1"></noembed><plaintext><img data-x1="0" data-y1="0" data-x2="1" data-y2="1">"#,
            source(),
        )
        .unwrap();
        assert_eq!(normalized.body_html, r#"<p>before</p>"#);
    }

    #[test]
    fn nested_templates_keep_the_outer_ignored_context() {
        let normalized = normalize(
            r#"<template><template><img data-x1="0" data-y1="0" data-x2="1" data-y2="1"></template><img data-x1="0" data-y1="0" data-x2="1" data-y2="1"></template><p>after</p>"#,
            source(),
        )
        .unwrap();
        assert_eq!(normalized.body_html, r#"<p>after</p>"#);
    }

    #[test]
    fn nested_removed_subtrees_keep_the_outer_ignored_context() {
        let normalized = normalize(
            r#"<svg><svg><img data-x1="0" data-y1="0" data-x2="1" data-y2="1"></svg><img data-x1="0" data-y1="0" data-x2="1" data-y2="1"></svg><p>after</p>"#,
            source(),
        )
        .unwrap();
        assert_eq!(normalized.body_html, r#"<p>after</p>"#);
    }

    #[test]
    fn namespace_like_unknown_wrappers_are_flattened_and_keep_marker_order() {
        let (bytes, width, height) = source_32();
        for wrapper in [
            "svg:wrapper",
            "svg?wrapper",
            "svg&wrapper",
            "svg\u{000b}wrapper",
        ] {
            let input = format!(
                r#"<{wrapper}><p>before</p><img data-x1="0" data-y1="0" data-x2="1" data-y2="1"><p>after</p></{wrapper}>"#
            );
            let normalized = normalize(
                &input,
                SourceImage {
                    bytes: &bytes,
                    width,
                    height,
                },
            )
            .unwrap();
            assert!(normalized
                .body_html
                .starts_with("<p>before</p><img src=\"data:image/png;base64,"));
            assert!(normalized.body_html.ends_with("<p>after</p>"));
        }
    }

    #[test]
    fn html_cleanup_flattens_tables_in_reading_order_without_duplicating_merged_cells() {
        let settings = ConversionSettings {
            tables: false,
            ..ConversionSettings::default()
        };
        let normalized = normalize_with_conversion(
            "<p>before</p><table><caption>Prices</caption><tbody><tr><th colspan='2'><p>Region</p></th></tr><tr><td><strong>North</strong></td><td><ul><li>65</li></ul></td></tr></tbody></table><p>after</p>",
            source(),
            settings,
            ConversionSettingsApplicationMode::HtmlCleanup,
        )
        .unwrap();
        assert!(!normalized.body_html.contains("<table"));
        assert!(normalized.body_html.contains("<p>Prices</p>"));
        assert!(normalized.body_html.contains("<div><p>Region</p></div>"));
        assert!(normalized
            .body_html
            .contains("<div><strong>North</strong> <ul><li>65</li></ul></div>"));
        assert_eq!(
            normalized.plain_text,
            "before\n\nPrices\n\nRegion\n\nNorth\n65\n\nafter"
        );
    }

    #[test]
    fn saved_formatting_preserves_pngs_and_literal_text_without_restoring_unsafe_content() {
        let body = concat!(
            r#"<p>Literal &lt;img src=&quot;data:image/png;base64,AA==&quot;&gt;</p>"#,
            r#"<img src="data:image/png;base64,AA=="><img src="data:image/png;base64,AA==">"#,
            r#"<script><img src="data:image/png;base64,AA=="></script>"#,
            r#"<img src="https://example.com/remote.png"><img src="data:image/svg+xml;base64,AA==">"#,
        );
        let result = reformat_saved(body, ConversionSettings::default()).unwrap();
        assert_eq!(
            result
                .body_html
                .matches("<img src=\"data:image/png;base64,AA==\"")
                .count(),
            2
        );
        assert!(result
            .plain_text
            .contains(r#"Literal <img src="data:image/png;base64,AA==">"#));
        assert!(!result.body_html.contains("script"));
        assert!(!result.body_html.contains("https://example.com"));
        assert!(!result.body_html.contains("image/svg+xml"));
        let removed = reformat_saved(
            body,
            ConversionSettings {
                images: false,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!removed.body_html.contains("<img"));
        assert_eq!(removed.plain_text, result.plain_text);
    }

    #[test]
    fn retained_baseline_restores_features_and_embedded_crops_without_source_image() {
        let (bytes, width, height) = source_32();
        let result = normalize_with_conversion(
            r#"<table><tr><th style="color: navy">Header</th></tr></table><img data-x1="0" data-y1="0" data-x2="1" data-y2="1"><p>Keep</p>"#,
            SourceImage { bytes: &bytes, width, height },
            ConversionSettings { tables: false, colors: false, images: false },
            ConversionSettingsApplicationMode::HtmlCleanup,
        ).unwrap();
        assert!(!result.body_html.contains("<table"));
        assert!(!result.body_html.contains("<img"));
        assert!(!result.body_html.contains("color:"));
        let restored = reformat_saved(
            result.formatting_body_html.as_deref().unwrap(),
            ConversionSettings::default(),
        )
        .unwrap();
        assert!(restored.body_html.contains("<table"));
        assert!(restored.body_html.contains("color: navy"));
        assert!(restored.body_html.contains("data:image/png;base64,"));
        assert!(!restored.body_html.contains("data-x1"));
        assert!(restored.plain_text.contains("Header"));
    }

    #[test]
    fn html_cleanup_with_all_features_enabled_matches_the_normalizer() {
        let input = r#"<p style="color: navy; padding: 4px">hello <strong>world</strong></p>"#;
        let source_image = source();
        assert_eq!(
            normalize_with_conversion(
                input,
                source_image,
                ConversionSettings::default(),
                ConversionSettingsApplicationMode::PromptAndHtmlCleanup,
            )
            .unwrap(),
            normalize(input, source_image).unwrap()
        );
    }

    #[test]
    fn disabled_tables_preserve_row_lines_from_prompts_and_cleanup() {
        let settings = ConversionSettings {
            tables: false,
            ..ConversionSettings::default()
        };
        for input in [
            "<p>Item Price<br>Apple 2<br>Pear 3</p>",
            "Item Price\nApple 2\r\nPear 3",
            "<table><thead><tr><th>Item</th><th>Price</th></tr></thead><tbody><tr><td>Apple</td><td>2</td></tr><tr><td>Pear</td><td>3</td></tr></tbody></table>",
        ] {
            for mode in [ConversionSettingsApplicationMode::HtmlCleanup, ConversionSettingsApplicationMode::PromptAndHtmlCleanup] {
                let result = normalize_with_conversion(input, source(), settings.clone(), mode).unwrap();
                assert_eq!(result.plain_text, "Item Price\nApple 2\nPear 3");
                assert!(!result.body_html.contains("<table"));
            }
        }
        let result = normalize_with_conversion(
            "<table><tr><td><p>A</p><p>B</p>C<br>D &amp; E</td><td><ul><li>F</li><li>G</li></ul></td></tr></table>",
            source(), settings.clone(), ConversionSettingsApplicationMode::HtmlCleanup,
        ).unwrap();
        assert!(result.body_html.contains("<p>A</p><p>B</p>C<br>D &amp; E"));
        assert!(result.body_html.contains("<ul><li>F</li><li>G</li></ul>"));
        let result = normalize_with_conversion(
            "<table><tr><td>Before<img data-x1=\"0\" data-y1=\"0\" data-x2=\"1\" data-y2=\"1\">After</td><td>Next</td></tr></table>",
            source(), settings.clone(), ConversionSettingsApplicationMode::HtmlCleanup,
        ).unwrap();
        assert!(result
            .body_html
            .contains("Before<img src=\"data:image/png;base64,"));
        assert!(result.body_html.contains("After Next</div>"));
    }

    #[test]
    fn html_cleanup_flattens_nested_tables_once_and_keeps_inner_content_in_place() {
        for colors in [true, false] {
            let normalized = normalize_with_conversion(
                r#"<table><caption style="color: navy; font-weight: bold">Outer</caption><tr><td>Left <table><caption>Inner</caption><tr><td>Nested</td></tr></table></td><td>Right</td></tr></table>"#,
                source(),
                ConversionSettings {
                    tables: false,
                    colors,
                    ..ConversionSettings::default()
                },
                ConversionSettingsApplicationMode::HtmlCleanup,
            )
            .unwrap();
            let style = if colors {
                "color: navy; font-weight: bold"
            } else {
                "font-weight: bold"
            };
            assert_eq!(
                normalized.body_html,
                format!(
                    "<div><p style=\"{style}\">Outer</p><div>Left <div><p>Inner</p><div>Nested</div></div> Right</div></div>"
                )
            );
        }
    }

    #[test]
    fn html_cleanup_removes_colors_but_keeps_border_shape_and_other_styles() {
        let settings = ConversionSettings {
            colors: false,
            ..ConversionSettings::default()
        };
        let normalized = normalize_with_conversion(
            r#"<p style="color: red; border: 2px solid currentColor; padding: 4px; text-align: center; background-color: #fff">text</p>"#,
            source(),
            settings,
            ConversionSettingsApplicationMode::PromptAndHtmlCleanup,
        )
        .unwrap();
        assert_eq!(
            normalized.body_html,
            r#"<p style="border: 2px solid; padding: 4px; text-align: center">text</p>"#
        );
    }

    #[test]
    fn html_cleanup_drops_images_without_validating_or_decoding_ignored_markers() {
        let settings = ConversionSettings {
            images: false,
            ..ConversionSettings::default()
        };
        let normalized = normalize_with_conversion(
            r#"<p>before</p><img data-x1="NaN" data-y1="-999999999999999999" data-x2="2" data-y2="Infinity"><p>after</p>"#,
            source(),
            settings,
            ConversionSettingsApplicationMode::HtmlCleanup,
        )
        .unwrap();
        assert_eq!(normalized.body_html, "<p>before</p><p>after</p>");
        assert!(!normalized.body_html.contains("data-x"));
        assert_eq!(
            normalize_with_conversion(
                r#"<img data-x1="NaN" data-y1="0" data-x2="1" data-y2="1">"#,
                source(),
                ConversionSettings {
                    images: false,
                    ..ConversionSettings::default()
                },
                ConversionSettingsApplicationMode::HtmlCleanup,
            ),
            Err(RenderingError::EmptyOutput)
        );
    }

    #[test]
    fn prompt_only_preserves_the_normalizer_even_when_features_are_disabled() {
        let (bytes, width, height) = source_32();
        let input = r#"<table><tr><td style="color: navy">before</td></tr></table><img data-x1="0" data-y1="0" data-x2="1" data-y2="1"><p>after</p>"#;
        let normalized = normalize_with_conversion(
            input,
            SourceImage {
                bytes: &bytes,
                width,
                height,
            },
            ConversionSettings {
                tables: false,
                colors: false,
                images: false,
            },
            ConversionSettingsApplicationMode::Prompt,
        )
        .unwrap();
        assert_eq!(
            normalized,
            normalize(
                input,
                SourceImage {
                    bytes: &bytes,
                    width,
                    height,
                }
            )
            .unwrap()
        );
        assert!(normalized.body_html.contains("data:image/png;base64,"));
    }
}
