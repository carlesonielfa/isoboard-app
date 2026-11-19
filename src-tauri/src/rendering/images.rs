//! Validation and embedding for the prompt's normalized image markers.

use std::io::{self, Cursor, Seek, SeekFrom, Write};

use base64::Engine as _;
use image::imageops::FilterType;
use image::{DynamicImage, ImageFormat, ImageReader, Limits};

use super::{RenderingError, SourceImage};

pub(crate) const MAX_MARKERS: usize = 32;
// Small valid regions are omitted, using normalized area so the cutoff scales with resolution.
const MIN_CROP_AREA_FRACTION: f64 = 0.001;
pub(crate) const MAX_EMBEDDED_IMAGE_BYTES: usize = 6 * 1024 * 1024;
const MAX_PER_IMAGE_BYTES: usize = 2 * 1024 * 1024;
const MAX_SOURCE_BYTES: usize = 20 * 1024 * 1024;
const MAX_IMAGE_PIXELS: u64 = 100_000_000;
const MAX_IMAGE_DIMENSION: u32 = 10_000;
const MAX_DECODE_BYTES: u64 = 100 * 1024 * 1024;
const ACTIVE_CONTENT: &[&str] = &[
    "canvas",
    "iframe",
    "noembed",
    "noscript",
    "object",
    "plaintext",
    "script",
    "style",
    "template",
    "title",
    "xmp",
];
const REMOVED_SUBTREES: &[&str] = &[
    "audio",
    "head",
    "noembed",
    "option",
    "plaintext",
    "select",
    "svg",
    "textarea",
    "video",
    "xmp",
];

pub(super) fn is_discarded_content(name: &str) -> bool {
    ACTIVE_CONTENT.contains(&name) || REMOVED_SUBTREES.contains(&name)
}

/// Validate marker shape before ammonia drops provider-owned attributes. This
/// keeps duplicate or partial markers from becoming an apparently valid empty
/// image after sanitization.
pub(crate) fn validate_markers(input: &str) -> Result<(), RenderingError> {
    let mut cursor = 0;
    let mut count = 0;
    let mut ignored = Vec::<String>::new();
    while let Some((start, end, name, closing)) = next_tag(input, cursor)? {
        cursor = end;
        if !ignored.is_empty() {
            if closing {
                if ignored.last().is_some_and(|active| active == &name) {
                    ignored.pop();
                }
            } else if ACTIVE_CONTENT.contains(&name.as_str())
                || REMOVED_SUBTREES.contains(&name.as_str())
            {
                ignored.push(name);
            }
            continue;
        }
        if !closing
            && (ACTIVE_CONTENT.contains(&name.as_str())
                || REMOVED_SUBTREES.contains(&name.as_str()))
        {
            ignored.push(name);
            if ignored.last().is_some_and(|active| active == "plaintext") {
                break;
            }
            continue;
        }
        if name == "img" && !closing {
            let attrs =
                parse_attributes(&input[start..end]).ok_or(RenderingError::InvalidImageMarker)?;
            coordinates(&attrs)?;
            count += 1;
            if count > MAX_MARKERS {
                return Err(RenderingError::ImageCountExceeded);
            }
        }
    }
    Ok(())
}

/// Image start tags in the same visible-content boundary as first-pass markers.
/// Raw slices retain duplicate attributes until the HTML tokenizer checks them.
pub(super) fn visible_image_tags(input: &str) -> Result<Vec<(usize, usize)>, RenderingError> {
    let mut cursor = 0;
    let mut ignored = Vec::<String>::new();
    let mut tags = Vec::new();
    while let Some((start, end, name, closing)) = next_tag(input, cursor)? {
        cursor = end;
        let removed =
            ACTIVE_CONTENT.contains(&name.as_str()) || REMOVED_SUBTREES.contains(&name.as_str());
        if !ignored.is_empty() {
            if closing {
                if ignored.last().is_some_and(|active| active == &name) {
                    ignored.pop();
                }
            } else if removed {
                ignored.push(name);
            }
            continue;
        }
        if !closing && removed {
            if name == "plaintext" {
                break;
            }
            ignored.push(name);
        } else if name == "img" && !closing {
            tags.push((start, end));
            if tags.len() > MAX_MARKERS {
                return Err(RenderingError::ImageCountExceeded);
            }
        }
    }
    Ok(tags)
}

/// Remove subtrees that the sanitizer discards as elements before it can
/// flatten their wrappers and accidentally promote child image markers into
/// document content.
pub(crate) fn strip_removed_subtrees(input: &str) -> String {
    let mut cursor = 0;
    let mut stack: Vec<String> = Vec::new();
    let mut start = None;
    let mut removals = Vec::new();
    while let Ok(Some((tag_start, tag_end, name, closing))) = next_tag(input, cursor) {
        cursor = tag_end;
        if !REMOVED_SUBTREES.contains(&name.as_str()) {
            continue;
        }
        if closing {
            if stack.last().is_some_and(|current| current == &name) {
                stack.pop();
                if stack.is_empty() {
                    removals.push((start.take().expect("removed subtree start exists"), tag_end));
                }
            }
        } else if stack.is_empty() {
            start = Some(tag_start);
            stack.push(name);
        } else {
            stack.push(name);
        }
    }
    if let Some(start) = start {
        removals.push((start, input.len()));
    }
    if removals.is_empty() {
        return input.to_owned();
    }
    let mut output = String::with_capacity(input.len());
    let mut last = 0;
    for (start, end) in removals {
        output.push_str(&input[last..start]);
        last = end;
    }
    output.push_str(&input[last..]);
    output
}

pub(crate) fn embed_markers(body: &str, source: SourceImage<'_>) -> Result<String, RenderingError> {
    let mut markers = 0usize;
    let mut cursor = 0;
    while let Some((_, end, name, closing)) = next_tag(body, cursor)? {
        cursor = end;
        if name == "img" && !closing {
            markers += 1;
        }
    }
    let mut decoded = None;
    let mut output = String::with_capacity(body.len());
    let mut embedded_bytes = 0usize;
    let mut last = 0;
    let mut cursor = 0;
    let mut count = 0;
    while let Some((start, end, name, closing)) = next_tag(body, cursor)? {
        cursor = end;
        if name != "img" || closing {
            continue;
        }
        let attrs =
            parse_attributes(&body[start..end]).ok_or(RenderingError::InvalidImageMarker)?;
        let coordinates = coordinates(&attrs)?;
        count += 1;
        if count > MAX_MARKERS {
            return Err(RenderingError::ImageCountExceeded);
        }
        if decoded.is_none() {
            decoded = Some(decode_source(source)?);
        }
        output.push_str(&body[last..start]);
        let replacement = crop_data_url(
            decoded.as_ref().expect("decoded source initialized above"),
            coordinates,
            crop_budget(embedded_bytes, markers - (count - 1)),
        )?;
        let Some(replacement) = replacement else {
            last = end;
            continue;
        };
        embedded_bytes = embedded_bytes
            .checked_add(replacement.len())
            .ok_or(RenderingError::EmbeddedImagesTooLarge)?;
        if embedded_bytes > MAX_EMBEDDED_IMAGE_BYTES {
            return Err(RenderingError::EmbeddedImagesTooLarge);
        }
        output.push_str(&replacement);
        last = end;
    }
    if count == 0 {
        return Ok(body.to_owned());
    }
    output.push_str(&body[last..]);
    Ok(output)
}

pub(super) fn next_tag(
    input: &str,
    mut cursor: usize,
) -> Result<Option<(usize, usize, String, bool)>, RenderingError> {
    while let Some(relative) = input[cursor..].find('<') {
        let start = cursor + relative;
        if input[start..].starts_with("<!--") {
            let Some(close) = input[start + 4..].find("-->") else {
                return Ok(None);
            };
            cursor = start + 4 + close + 3;
            continue;
        }
        let Some(end) = tag_end(input, start) else {
            return Ok(None);
        };
        let mut index = start + 1;
        let closing = input.as_bytes().get(index) == Some(&b'/');
        if closing {
            index += 1;
        }
        while input
            .get(index..)
            .and_then(|value| value.chars().next())
            .is_some_and(is_html_tag_space)
        {
            index += 1;
        }
        let name_start = index;
        let remainder = input.get(index..).unwrap_or_default();
        let name_length = remainder
            .char_indices()
            .find(|(_, character)| is_html_tag_space(*character) || matches!(character, '/' | '>'))
            .map_or(remainder.len(), |(offset, _)| offset);
        index += name_length;
        if name_start == index {
            cursor = end;
            continue;
        }
        return Ok(Some((
            start,
            end,
            input[name_start..index].to_ascii_lowercase(),
            closing,
        )));
    }
    Ok(None)
}

fn is_html_tag_space(character: char) -> bool {
    matches!(character, '\t' | '\n' | '\x0c' | '\r' | ' ')
}

fn tag_end(input: &str, start: usize) -> Option<usize> {
    let mut quote = None;
    for (offset, character) in input[start..].char_indices() {
        match (quote, character) {
            (None, '"' | '\'') => quote = Some(character),
            (Some(current), c) if current == c => quote = None,
            (None, '>') => return Some(start + offset + 1),
            _ => {}
        }
    }
    None
}

fn parse_attributes(tag: &str) -> Option<Vec<(String, String)>> {
    let tag_without_close = tag.strip_suffix('>')?;
    let prefix = tag_without_close.get(..4)?;
    if !prefix.eq_ignore_ascii_case("<img") {
        return None;
    }
    let inner = tag_without_close
        .get(4..)?
        .trim()
        .trim_end_matches('/')
        .trim();
    let mut attrs = Vec::new();
    let mut chars = inner.chars().peekable();
    while chars.peek().is_some() {
        while chars.peek().is_some_and(|c| c.is_ascii_whitespace()) {
            chars.next();
        }
        if chars.peek().is_none() {
            break;
        }
        let mut name = String::new();
        while chars
            .peek()
            .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        {
            name.push(chars.next()?);
        }
        if name.is_empty() {
            return None;
        }
        while chars.peek().is_some_and(|c| c.is_ascii_whitespace()) {
            chars.next();
        }
        if chars.peek().is_none() {
            attrs.push((name.to_ascii_lowercase(), String::new()));
            break;
        }
        if chars.next()? != '=' {
            attrs.push((name.to_ascii_lowercase(), String::new()));
            continue;
        }
        while chars.peek().is_some_and(|c| c.is_ascii_whitespace()) {
            chars.next();
        }
        let mut value = String::new();
        let first = chars.next()?;
        if first == '"' || first == '\'' {
            let quote = first;
            let mut closed = false;
            for c in chars.by_ref() {
                if c == quote {
                    closed = true;
                    break;
                }
                value.push(c);
            }
            if !closed {
                return None;
            }
        } else {
            value.push(first);
            while chars.peek().is_some_and(|c| !c.is_ascii_whitespace()) {
                value.push(chars.next()?);
            }
        }
        attrs.push((name.to_ascii_lowercase(), value));
    }
    Some(attrs)
}

pub(super) fn coordinates(attrs: &[(String, String)]) -> Result<[f64; 4], RenderingError> {
    let names = ["data-x1", "data-y1", "data-x2", "data-y2"];
    let mut values = [0.0; 4];
    for (index, name) in names.iter().enumerate() {
        let matches = attrs
            .iter()
            .filter(|(key, _)| key == name)
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err(RenderingError::InvalidImageMarker);
        }
        let value = matches[0]
            .1
            .parse::<f64>()
            .map_err(|_| RenderingError::InvalidImageMarker)?;
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(RenderingError::InvalidImageMarker);
        }
        values[index] = value;
    }
    if values[0] >= values[2] || values[1] >= values[3] {
        return Err(RenderingError::InvalidImageMarker);
    }
    Ok(values)
}

pub(super) fn decode_source(source: SourceImage<'_>) -> Result<DynamicImage, RenderingError> {
    if source.bytes.len() > MAX_SOURCE_BYTES {
        return Err(RenderingError::ImageSource);
    }
    let mut reader = ImageReader::new(Cursor::new(source.bytes));
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_DIMENSION);
    limits.max_image_height = Some(MAX_IMAGE_DIMENSION);
    limits.max_alloc = Some(MAX_DECODE_BYTES);
    reader.limits(limits);
    let decoded = reader
        .with_guessed_format()
        .map_err(|_| RenderingError::ImageSource)?
        .decode()
        .map_err(|_| RenderingError::ImageSource)?;
    let (width, height) = decoded.dimensions();
    if width != source.width || height != source.height {
        return Err(RenderingError::ImageSource);
    }
    if u64::from(width) * u64::from(height) > MAX_IMAGE_PIXELS {
        return Err(RenderingError::ImageSource);
    }
    Ok(decoded)
}

/// Splits the remaining aggregate budget evenly across the crops still to be
/// generated, so early large crops cannot starve later ones. Unused share
/// carries over to later crops.
pub(super) fn crop_budget(used_bytes: usize, crops_left: usize) -> usize {
    MAX_EMBEDDED_IMAGE_BYTES.saturating_sub(used_bytes) / crops_left.max(1)
}

pub(super) fn crop_data_url(
    decoded: &DynamicImage,
    coordinates: [f64; 4],
    remaining_budget: usize,
) -> Result<Option<String>, RenderingError> {
    let area_fraction = (coordinates[2] - coordinates[0]) * (coordinates[3] - coordinates[1]);
    if area_fraction < MIN_CROP_AREA_FRACTION {
        return Ok(None);
    }
    let (width, height) = decoded.dimensions();
    let x1 = ((coordinates[0] * f64::from(width)).floor() as u32).min(width);
    let y1 = ((coordinates[1] * f64::from(height)).floor() as u32).min(height);
    let x2 = ((coordinates[2] * f64::from(width)).ceil() as u32).min(width);
    let y2 = ((coordinates[3] * f64::from(height)).ceil() as u32).min(height);
    let crop_width = x2.saturating_sub(x1);
    let crop_height = y2.saturating_sub(y1);
    if crop_width == 0 || crop_height == 0 {
        return Ok(None);
    }
    let crop = decoded.crop_imm(x1, y1, crop_width, crop_height);
    let prefix = r#"<img src="data:image/png;base64,"#;
    let suffix = "\">";
    let max_raw = remaining_budget
        .saturating_sub(prefix.len() + suffix.len())
        .saturating_mul(3)
        / 4;
    if max_raw == 0 {
        return Err(RenderingError::EmbeddedImagesTooLarge);
    }
    let encoded = base64::engine::general_purpose::STANDARD
        .encode(encode_png_within(&crop, MAX_PER_IMAGE_BYTES.min(max_raw))?);
    let result = format!("{prefix}{encoded}{suffix}");
    if result.len() > remaining_budget {
        return Err(RenderingError::EmbeddedImagesTooLarge);
    }
    Ok(Some(result))
}

/// Photographic crops can exceed the per-crop or remaining aggregate budget at
/// full size, so they are downscaled until they fit rather than failing the
/// whole capture.
fn encode_png_within(crop: &DynamicImage, limit: usize) -> Result<Vec<u8>, RenderingError> {
    const SCALE_STEP: f64 = 0.75;
    const MAX_ATTEMPTS: u32 = 12;
    let (width, height) = crop.dimensions();
    let mut scale = 1.0;
    for _ in 0..MAX_ATTEMPTS {
        let mut bytes = LimitedWriter::new(limit);
        let written = if scale < 1.0 {
            let scaled_width = ((f64::from(width) * scale).round() as u32).max(1);
            let scaled_height = ((f64::from(height) * scale).round() as u32).max(1);
            crop.resize_exact(scaled_width, scaled_height, FilterType::Triangle)
                .write_to(&mut bytes, ImageFormat::Png)
        } else {
            crop.write_to(&mut bytes, ImageFormat::Png)
        };
        if written.is_ok() {
            return Ok(bytes.into_inner());
        }
        scale *= SCALE_STEP;
    }
    Err(RenderingError::EmbeddedImagesTooLarge)
}

struct LimitedWriter {
    bytes: Vec<u8>,
    limit: usize,
    position: usize,
}

impl LimitedWriter {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
            position: 0,
        }
    }
    fn into_inner(self) -> Vec<u8> {
        self.bytes
    }
}

impl Write for LimitedWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let end = self
            .position
            .checked_add(bytes.len())
            .ok_or_else(|| io::Error::other("image budget exceeded"))?;
        let remaining = self.limit.saturating_sub(self.position);
        if bytes.len() > remaining {
            return Err(io::Error::other("image budget exceeded"));
        }
        if self.position == self.bytes.len() {
            self.bytes.extend_from_slice(bytes);
        } else {
            self.bytes[self.position..end].copy_from_slice(bytes);
        }
        self.position = end;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Seek for LimitedWriter {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let base = match position {
            SeekFrom::Start(_) => 0_i128,
            SeekFrom::Current(_) => self.position as i128,
            SeekFrom::End(_) => self.bytes.len() as i128,
        };
        let offset = match position {
            SeekFrom::Start(value) => value as i128,
            SeekFrom::Current(value) | SeekFrom::End(value) => value as i128,
        };
        let next = base
            .checked_add(offset)
            .filter(|value| *value >= 0)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| io::Error::other("invalid image seek"))?;
        if next > self.bytes.len() {
            return Err(io::Error::other("invalid image seek"));
        }
        self.position = next;
        Ok(next as u64)
    }
}

use image::GenericImageView;

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};

    fn source_png(width: u32, height: u32) -> Vec<u8> {
        let image = ImageBuffer::from_fn(width, height, |x, y| {
            if x < width / 2 && y < height / 2 {
                Rgba([255, 0, 0, 255])
            } else {
                Rgba([0, 0, 255, 255])
            }
        });
        let mut bytes = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut bytes, ImageFormat::Png)
            .unwrap();
        bytes.into_inner()
    }

    #[test]
    fn relative_crop_cutoff_is_resolution_independent_and_omission_uses_no_byte_budget() {
        for size in [64, 128] {
            let bytes = source_png(size, size);
            let decoded = image::load_from_memory(&bytes).unwrap();
            assert_eq!(
                crop_data_url(&decoded, [0.0, 0.0, 0.02, 0.04], 0).unwrap(),
                None
            );
            assert!(
                crop_data_url(&decoded, [0.0, 0.0, 0.04, 0.03], MAX_PER_IMAGE_BYTES)
                    .unwrap()
                    .is_some()
            );
        }
    }

    #[test]
    fn rejects_partial_and_nonfinite_markers() {
        assert!(coordinates(&[("data-x1".into(), "0".into())]).is_err());
        assert!(coordinates(&[
            ("data-x1".into(), "NaN".into()),
            ("data-y1".into(), "0".into()),
            ("data-x2".into(), "1".into()),
            ("data-y2".into(), "1".into())
        ])
        .is_err());
    }

    #[test]
    fn crops_over_the_remaining_budget_are_downscaled_to_fit() {
        let noise = image::RgbImage::from_fn(300, 300, |x, y| {
            let value = x
                .wrapping_mul(2_654_435_761)
                .wrapping_add(y.wrapping_mul(40_503));
            image::Rgb([value as u8, (value >> 8) as u8, (value >> 16) as u8])
        });
        let decoded = DynamicImage::ImageRgb8(noise);
        let budget = 64 * 1024;
        let url = crop_data_url(&decoded, [0.0, 0.0, 1.0, 1.0], budget)
            .unwrap()
            .unwrap();
        assert!(url.len() <= budget);
        let encoded = url
            .trim_start_matches(r#"<img src="data:image/png;base64,"#)
            .trim_end_matches("\">");
        let png = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .unwrap();
        let (width, height) = image::load_from_memory(&png).unwrap().dimensions();
        assert!(width < 300 && width == height);
    }

    #[test]
    fn crops_with_floor_and_ceil_and_returns_app_generated_png() {
        let bytes = source_png(64, 64);
        let body = r#"<p>before</p><img data-x1="0.1" data-y1="0.1" data-x2="0.6" data-y2="0.6"><p>after</p>"#;
        let embedded = embed_markers(
            body,
            SourceImage {
                bytes: &bytes,
                width: 64,
                height: 64,
            },
        )
        .unwrap();
        assert!(embedded.starts_with("<p>before</p><img src=\"data:image/png;base64,"));
        assert!(embedded.ends_with("</p>"));
        let encoded = embedded
            .split("base64,")
            .nth(1)
            .unwrap()
            .trim_end_matches("\"><p>after</p>");
        let crop = image::load_from_memory(
            &base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(crop.dimensions(), (33, 33));
    }

    #[test]
    fn preserves_marker_order_and_rejects_duplicate_marker_attributes() {
        let bytes = source_png(64, 64);
        let ordered = r#"<img data-x1="0" data-y1="0" data-x2="0.5" data-y2="0.5"><p>middle</p><img data-x1="0.5" data-y1="0.5" data-x2="1" data-y2="1">"#;
        let embedded = embed_markers(
            ordered,
            SourceImage {
                bytes: &bytes,
                width: 64,
                height: 64,
            },
        )
        .unwrap();
        assert!(embedded.find("middle").unwrap() > embedded.find("data:image/png").unwrap());
        assert!(validate_markers(
            r#"<img data-x1="0" data-x1="0" data-y1="0" data-x2="1" data-y2="1">"#
        )
        .is_err());
    }
}
