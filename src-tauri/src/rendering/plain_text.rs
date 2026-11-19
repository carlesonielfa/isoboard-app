//! Plain-text derivation from canonical sanitized HTML.

use crate::profiles::{ConversionSettings, ConversionSettingsApplicationMode};

use super::{conversion, saved_images::CompactImages, RenderingError};

pub(crate) fn from_html(html: &str) -> Result<String, RenderingError> {
    let images = CompactImages::new(html);
    // Flatten tables before text rendering so cells need neither borders nor padding.
    let body = conversion::apply(
        images.html(),
        ConversionSettings {
            tables: false,
            ..ConversionSettings::default()
        },
        ConversionSettingsApplicationMode::HtmlCleanup,
    )?;
    let text = html2text::config::with_decorator(html2text::render::TrivialDecorator::new())
        .unicode_strikeout(false)
        .string_from_read(body.as_bytes(), usize::MAX)
        .map_err(|_| RenderingError::PlainTextExtraction)?
        .trim_matches('\n')
        .to_owned();

    if text.len() > super::policy::MAX_PLAIN_TEXT_BYTES {
        return Err(RenderingError::PlainTextTooLarge);
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforces_the_plain_text_byte_limit_independently() {
        let html = format!(
            "<p>{}</p>",
            "x".repeat(super::super::policy::MAX_PLAIN_TEXT_BYTES + 1)
        );
        assert_eq!(from_html(&html), Err(RenderingError::PlainTextTooLarge));
    }

    #[test]
    fn preserves_content_and_whitespace_without_generated_decoration() {
        let html = concat!(
            "<p>See <a href=\"https://example.com\">the link</a>.</p>",
            "<ul><li>Outer<ul><li>Inner</li></ul></li></ul>",
            "<pre>  first\n  second</pre>",
            "<table><tr><th>Item</th><th>Price</th></tr><tr><td>Apple</td><td>2</td></tr><tr><td>Pear</td><td>3</td></tr></table>",
            "<p><s>old</s> &amp; <strong>new</strong>: A | B │ C ─ D</p>",
        );
        assert_eq!(
            from_html(html).unwrap(),
            "See the link.\nOuter\nInner\n\n  first\n  second\n\nItem Price\nApple 2\nPear 3\n\nold & new: A | B │ C ─ D"
        );
    }

    #[test]
    fn preserves_preformatted_whitespace_at_document_edges() {
        assert_eq!(
            from_html("<pre>  first\n  second  </pre>").unwrap(),
            "  first\n  second  "
        );
    }
}
