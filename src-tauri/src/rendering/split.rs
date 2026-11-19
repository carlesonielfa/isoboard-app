//! A private crop registry bridges the two untrusted model responses.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use crate::profiles::{ConversionSettings, ConversionSettingsApplicationMode};
use html2text::{Element, RcDom};
use html5ever::tendril::TendrilSink;
use html5ever::tokenizer::{
    states::RawKind, BufferQueue, TagKind, Token, TokenSink, TokenSinkResult, Tokenizer,
    TokenizerOpts,
};
use html5ever::{local_name, ns, parse_fragment, QualName};

use super::{conversion, images, plain_text, policy, NormalizedHtml, RenderingError, SourceImage};

const IMAGE_REFERENCE: &str = "data-isoboard-image-ref";
const REFERENCE_ATTRIBUTES: &[&str] =
    &[IMAGE_REFERENCE, "style", "alt", "title", "width", "height"];

/// Neither raw provider HTML nor the original capture is retained here. Crop
/// payloads stay private and cannot enter the second model's request.
pub(crate) struct PreparedConversion {
    intermediate_html: String,
    crops: HashMap<String, String>,
}

impl PreparedConversion {
    pub(crate) fn intermediate_html(&self) -> &str {
        &self.intermediate_html
    }
}

pub(crate) fn prepare_conversion(
    input: &str,
    source: SourceImage<'_>,
) -> Result<PreparedConversion, RenderingError> {
    let input = stage_input(input)?;
    let tags = images::visible_image_tags(input)?;
    let mut decoded = None;
    let mut crops = HashMap::new();
    let mut allocated_bytes = 0usize;
    let mut replaced = String::with_capacity(input.len());
    let mut last = 0;
    let tag_count = tags.len();
    for (index, (start, end)) in tags.into_iter().enumerate() {
        let coordinates = images::coordinates(&image_attributes(&input[start..end])?)?;
        if decoded.is_none() {
            decoded = Some(images::decode_source(source)?);
        }
        let crop = images::crop_data_url(
            decoded.as_ref().expect("source decoded for crop"),
            coordinates,
            images::crop_budget(allocated_bytes, tag_count - index),
        )?;
        replaced.push_str(&input[last..start]);
        last = end;
        let Some(crop) = crop else {
            continue;
        };
        allocated_bytes = allocated_bytes
            .checked_add(crop.len())
            .ok_or(RenderingError::EmbeddedImagesTooLarge)?;
        // References are intentionally short and only meaningful within this
        // prepared conversion's private crop registry.
        let reference = format!("i{}", crops.len() + 1);
        replaced.push_str(&format!("<img {IMAGE_REFERENCE}=\"{reference}\">"));
        crops.insert(reference, crop);
    }
    replaced.push_str(&input[last..]);
    let body = sanitize_references(&replaced)?;
    validate_references(&body, &crops)?;
    if body.is_empty() {
        return Err(RenderingError::EmptyOutput);
    }
    let intermediate_html = document(&body)?;
    Ok(PreparedConversion {
        intermediate_html,
        crops,
    })
}

pub(crate) fn finalize_postprocessing(
    prepared: PreparedConversion,
    input: &str,
    settings: ConversionSettings,
    mode: ConversionSettingsApplicationMode,
) -> Result<NormalizedHtml, RenderingError> {
    let input = stage_input(input)?;
    // Validate before either sanitizer or checkbox cleanup can erase evidence.
    validate_raw_references(input, &prepared.crops)?;
    // Subtree removal is part of the safety boundary. Check its exact output
    // too: HTML5 raw-text repair can disagree with textual subtree endpoints.
    let stripped = images::strip_removed_subtrees(input);
    validate_raw_references(&stripped, &prepared.crops)?;
    validate_dom_references(&stripped, &prepared.crops)?;
    let body = sanitize_references(input)?;
    validate_references(&body, &prepared.crops)?;
    let cleanup_images = !settings.images
        && matches!(
            mode,
            ConversionSettingsApplicationMode::HtmlCleanup
                | ConversionSettingsApplicationMode::PromptAndHtmlCleanup
        );
    let formatting_body_html = embed_references(&body, &prepared.crops)?;
    let body = conversion::apply(&formatting_body_html, settings, mode)?;
    if body.trim().is_empty() {
        return Err(RenderingError::EmptyOutput);
    }
    let document_html = document(&body)?;
    let plain_text = plain_text::from_html(&body)?;
    if cleanup_images && plain_text.trim().is_empty() {
        return Err(RenderingError::EmptyOutput);
    }
    Ok(NormalizedHtml {
        formatting_body_html: Some(formatting_body_html),
        document_html,
        body_html: body,
        plain_text,
    })
}

fn stage_input(input: &str) -> Result<&str, RenderingError> {
    if input.len() > super::MAX_PROVIDER_OUTPUT_BYTES {
        return Err(RenderingError::ProviderOutputTooLarge);
    }
    let input = super::strip_outer_html_fence(input).trim();
    if input.is_empty() {
        return Err(RenderingError::EmptyInput);
    }
    // Check even the subtrees the sanitizer will discard, before serialization.
    let dom = parse_body(input);
    conversion::checked_nodes(&dom.document)?;
    Ok(input)
}

fn parse_body(body: &str) -> RcDom {
    parse_fragment(
        RcDom::default(),
        Default::default(),
        QualName::new(None, ns!(html), local_name!(body)),
        Vec::new(),
        false,
    )
    .one(body.to_owned())
}

fn sanitize_references(input: &str) -> Result<String, RenderingError> {
    let input = images::strip_removed_subtrees(input);
    let body = if super::contains_markup(&input) {
        policy::reference_sanitizer().clean(&input).to_string()
    } else {
        format!("<p>{}</p>", super::escape_text(&input))
    };
    let dom = parse_body(&body);
    conversion::checked_nodes(&dom.document)?;
    Ok(body.trim().to_owned())
}

fn validate_references(input: &str, crops: &HashMap<String, String>) -> Result<(), RenderingError> {
    let attributes = images::visible_image_tags(input)?
        .into_iter()
        .map(|(start, end)| image_attributes(&input[start..end]))
        .collect::<Result<Vec<_>, _>>()?;
    validate_reference_attributes(attributes, crops)
}

fn validate_reference_attributes(
    images: Vec<Vec<(String, String)>>,
    crops: &HashMap<String, String>,
) -> Result<(), RenderingError> {
    let mut embedded_bytes = 0usize;
    for attributes in images {
        if attributes
            .iter()
            .any(|(name, _)| !REFERENCE_ATTRIBUTES.contains(&name.as_str()))
        {
            return Err(RenderingError::InvalidImageMarker);
        }
        let reference = attributes
            .iter()
            .find(|(name, _)| name == IMAGE_REFERENCE)
            .ok_or(RenderingError::InvalidImageMarker)?;
        let crop = crops
            .get(&reference.1)
            .ok_or(RenderingError::InvalidImageMarker)?;
        embedded_bytes = embedded_bytes
            .checked_add(crop.len())
            .ok_or(RenderingError::EmbeddedImagesTooLarge)?;
        if embedded_bytes > images::MAX_EMBEDDED_IMAGE_BYTES {
            return Err(RenderingError::EmbeddedImagesTooLarge);
        }
    }
    Ok(())
}

/// Inspect the parser's actual visible nodes before attribute filtering. The
/// tokenizer is responsible for duplicate evidence; this DOM check covers
/// aliases, namespace repair, and raw-text/tree-construction interactions.
fn validate_dom_references(
    input: &str,
    crops: &HashMap<String, String>,
) -> Result<(), RenderingError> {
    let dom = parse_body(input);
    conversion::checked_nodes(&dom.document)?;
    let mut pending = vec![dom.document.clone()];
    let mut images = Vec::new();
    while let Some(node) = pending.pop() {
        let name = node.element_name();
        if name.as_deref().is_some_and(images::is_discarded_content) {
            continue;
        }
        if name.as_deref() == Some("img") {
            let Element { ref attrs, .. } = node.data else {
                continue;
            };
            images.push(
                attrs
                    .borrow()
                    .iter()
                    .map(|attribute| {
                        (
                            attribute.name.local.to_string(),
                            attribute.value.to_string(),
                        )
                    })
                    .collect(),
            );
            if images.len() > images::MAX_MARKERS {
                return Err(RenderingError::ImageCountExceeded);
            }
        }
        pending.extend(node.children.borrow().iter().rev().cloned());
    }
    validate_reference_attributes(images, crops)
}

#[derive(Default)]
struct RawReferenceSink {
    images: RefCell<Vec<Vec<(String, String)>>>,
    ignored: RefCell<Vec<String>>,
    attribute_error: Cell<bool>,
    error: Cell<Option<RenderingError>>,
}

impl TokenSink for RawReferenceSink {
    type Handle = ();

    fn process_token(&self, token: Token, _line: u64) -> TokenSinkResult<()> {
        match token {
            Token::ParseError(_) | Token::NullCharacterToken => self.attribute_error.set(true),
            Token::TagToken(tag) => {
                let name = tag.name.as_ref();
                let closing = tag.kind == TagKind::EndTag;
                let mut ignored = self.ignored.borrow_mut();
                if !ignored.is_empty() {
                    if closing {
                        if ignored.last().is_some_and(|active| active == name) {
                            ignored.pop();
                        }
                    } else if images::is_discarded_content(name) {
                        ignored.push(name.to_owned());
                    }
                } else if !closing && images::is_discarded_content(name) {
                    ignored.push(name.to_owned());
                } else if !closing && matches!(name, "img" | "image") {
                    if self.attribute_error.get() {
                        self.error.set(Some(RenderingError::InvalidImageMarker));
                    }
                    let mut images = self.images.borrow_mut();
                    if images.len() == images::MAX_MARKERS {
                        self.error.set(Some(RenderingError::ImageCountExceeded));
                    } else {
                        images.push(
                            tag.attrs
                                .into_iter()
                                .map(|attr| (attr.name.local.to_string(), attr.value.to_string()))
                                .collect(),
                        );
                    }
                }
                if ignored.len() > 1024 {
                    self.error.set(Some(RenderingError::HtmlNestingTooDeep));
                }
                self.attribute_error.set(false);
                if !closing {
                    return match name {
                        "plaintext" => TokenSinkResult::Plaintext,
                        "script" => TokenSinkResult::RawData(RawKind::ScriptData),
                        "textarea" | "title" => TokenSinkResult::RawData(RawKind::Rcdata),
                        "iframe" | "noembed" | "noscript" | "style" | "xmp" => {
                            TokenSinkResult::RawData(RawKind::Rawtext)
                        }
                        _ => TokenSinkResult::Continue,
                    };
                }
            }
            Token::CommentToken(_) | Token::CharacterTokens(_) | Token::DoctypeToken(_) => {
                self.attribute_error.set(false);
            }
            _ => {}
        }
        TokenSinkResult::Continue
    }
}

fn validate_raw_references(
    input: &str,
    crops: &HashMap<String, String>,
) -> Result<(), RenderingError> {
    let tokenizer = Tokenizer::new(
        RawReferenceSink::default(),
        TokenizerOpts {
            exact_errors: true,
            ..Default::default()
        },
    );
    let queue = BufferQueue::default();
    queue.push_back(input.into());
    let _ = tokenizer.feed(&queue);
    tokenizer.end();
    if let Some(error) = tokenizer.sink.error.get() {
        return Err(error);
    }
    validate_reference_attributes(tokenizer.sink.images.into_inner(), crops)
}

fn embed_references(body: &str, crops: &HashMap<String, String>) -> Result<String, RenderingError> {
    validate_references(body, crops)?;
    let dom = parse_body(body);
    for node in conversion::checked_nodes(&dom.document)? {
        if node.element_name().as_deref() != Some("img") {
            continue;
        }
        let Element { ref attrs, .. } = node.data else {
            continue;
        };
        let mut attrs = attrs.borrow_mut();
        let reference = attrs
            .iter_mut()
            .find(|attribute| attribute.name.local.as_ref() == IMAGE_REFERENCE)
            .ok_or(RenderingError::InvalidImageMarker)?;
        let crop = crops
            .get(reference.value.as_ref())
            .ok_or(RenderingError::InvalidImageMarker)?;
        let src = crop
            .strip_prefix("<img src=\"")
            .and_then(|crop| crop.strip_suffix("\">"))
            .ok_or(RenderingError::ImageSource)?;
        reference.name = QualName::new(None, ns!(), local_name!(src));
        reference.value = src.into();
    }
    conversion::serialize_document(&dom)
}

fn document(body: &str) -> Result<String, RenderingError> {
    if body
        .len()
        .saturating_add("<html><body></body></html>".len())
        > super::MAX_DOCUMENT_BYTES
    {
        return Err(RenderingError::HtmlTooLarge);
    }
    Ok(format!("<html><body>{body}</body></html>"))
}

#[derive(Default)]
struct ImageTokenSink {
    attributes: RefCell<Option<Vec<(String, String)>>>,
    invalid: Cell<bool>,
}

impl TokenSink for ImageTokenSink {
    type Handle = ();

    fn process_token(&self, token: Token, _line_number: u64) -> TokenSinkResult<()> {
        match token {
            Token::TagToken(tag) if tag.name.as_ref() == "img" => {
                if self.attributes.borrow().is_some() {
                    self.invalid.set(true);
                }
                *self.attributes.borrow_mut() = Some(
                    tag.attrs
                        .into_iter()
                        .map(|attribute| {
                            (
                                attribute.name.local.to_string(),
                                attribute.value.to_string(),
                            )
                        })
                        .collect(),
                );
            }
            // In particular, duplicate attributes are tokenizer errors, before
            // the DOM parser can discard the second value and lose evidence.
            Token::ParseError(_) | Token::NullCharacterToken => self.invalid.set(true),
            _ => {}
        }
        TokenSinkResult::Continue
    }
}

fn image_attributes(tag: &str) -> Result<Vec<(String, String)>, RenderingError> {
    let tokenizer = Tokenizer::new(
        ImageTokenSink::default(),
        TokenizerOpts {
            exact_errors: true,
            ..Default::default()
        },
    );
    let queue = BufferQueue::default();
    queue.push_back(tag.into());
    let _ = tokenizer.feed(&queue);
    tokenizer.end();
    if tokenizer.sink.invalid.get() {
        return Err(RenderingError::InvalidImageMarker);
    }
    tokenizer
        .sink
        .attributes
        .into_inner()
        .ok_or(RenderingError::InvalidImageMarker)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba};

    use super::*;

    const MARKER: &str = r#"<img data-x1="0" data-y1="0" data-x2="1" data-y2="1">"#;

    fn png(width: u32, height: u32, noisy: bool) -> Vec<u8> {
        let mut seed = 17u32;
        let image = ImageBuffer::from_fn(width, height, |_, _| {
            if noisy {
                let mut channels = [0; 4];
                for channel in &mut channels {
                    seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    *channel = (seed >> 24) as u8;
                }
                Rgba(channels)
            } else {
                Rgba([255, 0, 0, 255])
            }
        });
        let mut bytes = Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(image)
            .write_to(&mut bytes, ImageFormat::Png)
            .unwrap();
        bytes.into_inner()
    }

    fn prepare(input: &str) -> PreparedConversion {
        let bytes = png(32, 32, false);
        prepare_conversion(
            input,
            SourceImage {
                bytes: &bytes,
                width: 32,
                height: 32,
            },
        )
        .unwrap()
    }

    fn reference_tags(prepared: &PreparedConversion) -> Vec<String> {
        let html = prepared.intermediate_html();
        images::visible_image_tags(html)
            .unwrap()
            .into_iter()
            .map(|(start, end)| html[start..end].to_owned())
            .collect()
    }

    fn finalize(prepared: PreparedConversion, raw: &str) -> Result<NormalizedHtml, RenderingError> {
        finalize_postprocessing(
            prepared,
            raw,
            ConversionSettings::default(),
            ConversionSettingsApplicationMode::Prompt,
        )
    }

    #[test]
    fn intermediate_exposes_only_app_references_and_safe_document_content() {
        let prepared = prepare(&format!(
            r#"<p style="color:red">before</p>{}<p>caption</p><script>bad</script>"#,
            MARKER.replace("<img", r#"<img src="https://bad" alt="bad" onclick="bad""#)
        ));
        let html = prepared.intermediate_html();
        assert!(html.contains(r#"style="color: red""#));
        assert!(html.contains("before") && html.contains("caption"));
        for forbidden in [
            "src=", "data-x", "data-y", "base64", "onclick", "alt=", "bad",
        ] {
            assert!(!html.contains(forbidden), "{forbidden}");
        }
        let tags = reference_tags(&prepared);
        let attributes = image_attributes(&tags[0]).unwrap();
        assert_eq!(attributes.len(), 1);
        assert_eq!(attributes[0].1, "i1");
        let result = finalize(prepared, &format!("<p>changed caption</p>{}", tags[0])).unwrap();
        assert!(result.body_html.contains("data:image/png;base64,"));
        assert!(!result.body_html.contains(IMAGE_REFERENCE));
        assert_eq!(result.plain_text, "changed caption");
    }

    #[test]
    fn final_known_references_can_reorder_drop_repeat_and_keep_safe_presentation() {
        let prepared = prepare(&format!("<p>first</p>{MARKER}<p>second</p>{MARKER}"));
        let tags = reference_tags(&prepared);
        let styled = tags[1].replace(
            ">",
            r#" style="color:navy;position:absolute" alt="A &amp; B" width="64">"#,
        );
        let result = finalize(
            prepared,
            &format!(
                "<p>reordered</p>{styled}<p>caption</p>{}<p>end</p>{styled}",
                tags[0]
            ),
        )
        .unwrap();
        assert_eq!(
            result.body_html.matches("data:image/png;base64,").count(),
            3
        );
        assert!(result.body_html.contains(r#"style="color: navy""#));
        assert!(result.body_html.contains(r#"alt="A &amp; B""#));
        assert!(result.body_html.contains(r#"width="64""#));
        assert!(!result.body_html.contains("position"));
        let prepared = prepare(&format!("<p>first</p>{MARKER}{MARKER}"));
        assert_eq!(
            finalize(prepared, "<p>all image refs dropped</p>")
                .unwrap()
                .plain_text,
            "all image refs dropped"
        );
    }

    #[test]
    fn raw_final_image_violations_fail_before_images_false_cleanup() {
        for extra in [
            r#" src="https://bad""#,
            r#" src="data:image/png;base64,AAAA""#,
            r#" data-x1="0""#,
            r#" data-y2="1""#,
            r#" onclick="bad""#,
            r#" srcset="https://bad""#,
            r#" class="bad""#,
            r#" DATA-ISOBOARD-IMAGE-REF="unknown""#,
        ] {
            let prepared = prepare(MARKER);
            let tag = reference_tags(&prepared)
                .remove(0)
                .replace(">", &format!("{extra}>"));
            let result = finalize_postprocessing(
                prepared,
                &format!("<p>text</p>{tag}"),
                ConversionSettings {
                    images: false,
                    ..Default::default()
                },
                ConversionSettingsApplicationMode::HtmlCleanup,
            );
            assert_eq!(result, Err(RenderingError::InvalidImageMarker), "{extra}");
        }
        for raw in [
            "<p>text</p><img>",
            r#"<p>text</p><img data-isoboard-image-ref="unknown">"#,
        ] {
            assert_eq!(
                finalize(prepare(MARKER), raw),
                Err(RenderingError::InvalidImageMarker)
            );
        }
        let first = prepare(MARKER);
        let first_tag = reference_tags(&first).remove(0);
        let second = prepare_conversion(
            MARKER,
            SourceImage {
                bytes: &png(32, 32, true),
                width: 32,
                height: 32,
            },
        )
        .unwrap();
        let second_crop = second.crops["i1"].clone();
        let result = finalize(second, &first_tag).unwrap();
        assert!(result.body_html.contains(&second_crop));
    }

    #[test]
    fn raw_visibility_and_entity_decoding_match_safe_reference_output() {
        let prepared = prepare(MARKER);
        let tag = reference_tags(&prepared).remove(0);
        let (prefix, value) = tag.split_once("=\"").unwrap();
        let encoded = format!("{prefix}=\"{}", value.replace("i1", "&#105;1"));
        let raw = format!(
            r#"<!-- <img src="bad"> --><div title='<img src="bad">'>text</div><script><img src="bad"></script><svg><img src="bad"></svg><textarea><img src="bad"></textarea>{encoded}"#
        );
        let result = finalize(prepared, &raw).unwrap();
        assert_eq!(
            result.body_html.matches("data:image/png;base64,").count(),
            1
        );
        assert!(!result.body_html.contains("bad"));
        assert_eq!(result.plain_text, "text");
        assert!(prepare_conversion(
            r#"<p>safe</p><!-- <img> --><script><img></script><head><img></head>"#,
            SourceImage {
                bytes: &[],
                width: 0,
                height: 0
            },
        )
        .is_ok());
        let prepared = prepare(MARKER);
        let tag = reference_tags(&prepared)
            .remove(0)
            .replace(">", r#" data-isoboard-image-ref="&#117;nknown">"#);
        assert_eq!(
            finalize(prepared, &tag),
            Err(RenderingError::InvalidImageMarker)
        );
    }

    #[test]
    fn html5_repair_cannot_hide_forbidden_attributes_before_sanitization() {
        for prefix in [
            "<!-- junk --!>",
            "<script><style></script>",
            "<head><script></head></script>",
            "<head><title></head></title>",
            "",
        ] {
            let prepared = prepare(MARKER);
            let mut tag = reference_tags(&prepared).remove(0);
            if prefix.is_empty() {
                tag = tag.replace("<img", "<image");
            }
            tag = tag.replace(">", r#" src="https://bad">"#);
            assert_eq!(
                finalize(prepared, &format!("<p>text</p>{prefix}{tag}")),
                Err(RenderingError::InvalidImageMarker)
            );
        }
        let prepared = prepare(MARKER);
        let encoded_value = reference_tags(&prepared).remove(0).replace("i1", "&#105;1");
        assert!(finalize(prepared, &encoded_value).is_ok());
        let prepared = prepare(MARKER);
        let tag = reference_tags(&prepared).remove(0);
        assert!(finalize(prepared, &format!("<!-- repaired --!>{tag}")).is_ok());
        let prepared = prepare(MARKER);
        let tag = reference_tags(&prepared).remove(0);
        assert_eq!(
            finalize_postprocessing(
                prepared,
                &format!("<div>{tag}</div>"),
                ConversionSettings {
                    images: false,
                    ..Default::default()
                },
                ConversionSettingsApplicationMode::HtmlCleanup
            ),
            Err(RenderingError::EmptyOutput)
        );
    }

    #[test]
    fn checkbox_cleanup_happens_only_after_second_call_and_prompt_mode_preserves_images() {
        let raw = format!(
            r#"<table border="1"><caption>caption</caption><tr><td style="color:red">cell</td><td>{MARKER}</td></tr></table>"#
        );
        for mode in [
            ConversionSettingsApplicationMode::Prompt,
            ConversionSettingsApplicationMode::HtmlCleanup,
        ] {
            let prepared = prepare(&raw);
            let second = format!("{}<p>Second-pass content</p>", prepared.intermediate_html());
            assert!(
                second.contains("<table")
                    && second.contains("color: red")
                    && second.contains(IMAGE_REFERENCE)
            );
            let result = finalize_postprocessing(
                prepared,
                &second,
                ConversionSettings {
                    tables: false,
                    colors: false,
                    images: false,
                },
                mode,
            )
            .unwrap();
            let baseline = result.formatting_body_html.as_deref().unwrap();
            assert!(baseline.contains("Second-pass content"));
            let restored =
                super::super::reformat_saved(baseline, ConversionSettings::default()).unwrap();
            assert!(restored.body_html.contains("<table"));
            assert!(restored.body_html.contains("color: red"));
            assert!(restored.body_html.contains("data:image/png;base64,"));
            if mode == ConversionSettingsApplicationMode::Prompt {
                assert!(
                    result.body_html.contains("<table")
                        && result.body_html.contains("color: red")
                        && result.body_html.contains("base64")
                );
            } else {
                assert!(
                    !result.body_html.contains("<table")
                        && !result.body_html.contains("color:")
                        && !result.body_html.contains("<img")
                );
                assert!(
                    result.plain_text.contains("caption") && result.plain_text.contains("cell")
                );
            }
        }
    }

    #[test]
    fn split_preparation_validates_actual_source_dimensions_and_invalid_coordinates() {
        let bytes = png(32, 32, false);
        for (raw, width, expected) in [
            (MARKER, 64, RenderingError::ImageSource),
            (
                r#"<img data-x1="0.5" data-y1="0" data-x2="0" data-y2="1">"#,
                32,
                RenderingError::InvalidImageMarker,
            ),
            (
                r#"<img data-x1="0" data-x1="0" data-y1="0" data-x2="1" data-y2="1">"#,
                32,
                RenderingError::InvalidImageMarker,
            ),
        ] {
            assert!(
                matches!(prepare_conversion(raw, SourceImage { bytes: &bytes, width, height: 32 }), Err(error) if error == expected)
            );
        }
    }

    #[test]
    fn both_stages_and_repeated_images_enforce_count_and_nesting_bounds() {
        let oversized = "x".repeat(super::super::MAX_PROVIDER_OUTPUT_BYTES + 1);
        assert!(matches!(
            prepare_conversion(
                &oversized,
                SourceImage {
                    bytes: &[],
                    width: 0,
                    height: 0
                }
            ),
            Err(RenderingError::ProviderOutputTooLarge)
        ));
        assert_eq!(
            finalize(prepare("<p>x</p>"), &oversized),
            Err(RenderingError::ProviderOutputTooLarge)
        );
        let deep = format!("{}text{}", "<div>".repeat(1100), "</div>".repeat(1100));
        assert!(matches!(
            prepare_conversion(
                &deep,
                SourceImage {
                    bytes: &[],
                    width: 0,
                    height: 0
                }
            ),
            Err(RenderingError::HtmlNestingTooDeep)
        ));
        assert_eq!(
            finalize(prepare("<p>x</p>"), &deep),
            Err(RenderingError::HtmlNestingTooDeep)
        );
        assert!(matches!(
            prepare_conversion(
                &MARKER.repeat(33),
                SourceImage {
                    bytes: &[],
                    width: 0,
                    height: 0
                }
            ),
            Err(RenderingError::ImageCountExceeded)
        ));
        let prepared = prepare(MARKER);
        let tag = reference_tags(&prepared).remove(0);
        assert_eq!(
            finalize(prepared, &tag.repeat(33)),
            Err(RenderingError::ImageCountExceeded)
        );
    }

    #[test]
    fn generated_crops_share_the_budget_and_repeated_embedding_stays_bounded() {
        let bytes = png(256, 256, true);
        let source = SourceImage {
            bytes: &bytes,
            width: 256,
            height: 256,
        };
        // Crops that would exceed the aggregate at full size are downscaled
        // so every marker still gets an image.
        let prepared = prepare_conversion(&MARKER.repeat(32), source).unwrap();
        assert_eq!(prepared.crops.len(), 32);
        let total: usize = prepared.crops.values().map(String::len).sum();
        assert!(total <= images::MAX_EMBEDDED_IMAGE_BYTES);
        let prepared = prepare_conversion(MARKER, source).unwrap();
        let tag = reference_tags(&prepared).remove(0);
        assert_eq!(
            finalize(prepared, &tag.repeat(32)),
            Err(RenderingError::EmbeddedImagesTooLarge)
        );
        let prepared = prepare_conversion(MARKER, source).unwrap();
        let tag = reference_tags(&prepared).remove(0);
        let large_document = format!("<p>{}</p>{}", "&".repeat(600_000), tag.repeat(17));
        assert_eq!(
            finalize(prepared, &large_document),
            Err(RenderingError::HtmlTooLarge)
        );
    }
}
