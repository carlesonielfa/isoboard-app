//! Detection of what kinds of content a sanitized result contains.
//!
//! History stores the detected tags as a bitmask so a list can be filtered
//! by content. The input is sanitized HTML, so only allowlisted tags and
//! style properties can appear, and the tokenizer sees real tags rather than
//! text that merely looks like markup.

use std::cell::Cell;

use html5ever::tokenizer::{
    BufferQueue, Tag, TagKind, Token, TokenSink, TokenSinkResult, Tokenizer, TokenizerOpts,
};
use serde::{Deserialize, Serialize};

/// A kind of content a result can contain.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ContentTag {
    Tables,
    Images,
    /// Emphasis or inline color, background, font, or text-decoration styles.
    /// Layout styles such as alignment, spacing, and borders do not count.
    Styling,
    Code,
}

impl ContentTag {
    pub(crate) const ALL: [Self; 4] = [Self::Tables, Self::Images, Self::Styling, Self::Code];

    /// The tag's bit in a stored mask. Stored rows keep these values, so a
    /// bit is never renumbered or reused; a new tag takes the next free bit.
    pub(crate) const fn bit(self) -> i64 {
        match self {
            Self::Tables => 1 << 0,
            Self::Images => 1 << 1,
            Self::Styling => 1 << 2,
            Self::Code => 1 << 3,
        }
    }

    pub(crate) fn mask(tags: &[Self]) -> i64 {
        tags.iter().fold(0, |mask, tag| mask | tag.bit())
    }

    pub(crate) fn from_mask(mask: i64) -> Vec<Self> {
        Self::ALL
            .into_iter()
            .filter(|tag| mask & tag.bit() != 0)
            .collect()
    }
}

const EMPHASIS_TAGS: &[&str] = &[
    "b", "del", "em", "i", "s", "strike", "strong", "sub", "sup", "u",
];
const CODE_TAGS: &[&str] = &["code", "pre", "samp"];
const STYLING_PROPERTIES: &[&str] = &[
    "background-color",
    "color",
    "font-size",
    "font-style",
    "font-weight",
    "text-decoration",
];

/// Returns the mask of content tags present in sanitized body HTML.
pub(crate) fn content_tags(body_html: &str) -> i64 {
    let tokenizer = Tokenizer::new(ContentSink::default(), TokenizerOpts::default());
    let queue = BufferQueue::default();
    queue.push_back(body_html.into());
    let _ = tokenizer.feed(&queue);
    tokenizer.end();
    tokenizer.sink.mask.get()
}

#[derive(Default)]
struct ContentSink {
    mask: Cell<i64>,
}

impl ContentSink {
    fn add(&self, tag: ContentTag) {
        self.mask.set(self.mask.get() | tag.bit());
    }

    fn inspect(&self, tag: &Tag) {
        let name = tag.name.as_ref();
        match name {
            "table" => self.add(ContentTag::Tables),
            "img" => self.add(ContentTag::Images),
            _ if CODE_TAGS.contains(&name) => self.add(ContentTag::Code),
            _ if EMPHASIS_TAGS.contains(&name) => self.add(ContentTag::Styling),
            _ => {}
        }
        if tag.attrs.iter().any(|attribute| {
            attribute.name.local.as_ref() == "style" && styles_text(&attribute.value)
        }) {
            self.add(ContentTag::Styling);
        }
    }
}

/// Whether a style attribute sets a property that changes how text looks.
fn styles_text(style: &str) -> bool {
    style.split(';').any(|declaration| {
        declaration.split_once(':').is_some_and(|(property, _)| {
            STYLING_PROPERTIES.contains(&property.trim().to_ascii_lowercase().as_str())
        })
    })
}

impl TokenSink for ContentSink {
    type Handle = ();

    fn process_token(&self, token: Token, _line_number: u64) -> TokenSinkResult<()> {
        if let Token::TagToken(tag) = token {
            if tag.kind == TagKind::StartTag {
                self.inspect(&tag);
            }
        }
        TokenSinkResult::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tags(html: &str) -> Vec<ContentTag> {
        ContentTag::from_mask(content_tags(html))
    }

    #[test]
    fn detects_each_kind_of_content() {
        assert_eq!(tags("<p>Plain text</p>"), []);
        assert_eq!(
            tags("<table><tbody><tr><td>1</td></tr></tbody></table>"),
            [ContentTag::Tables]
        );
        assert_eq!(
            tags(r#"<p><img src="data:image/png;base64,AA"></p>"#),
            [ContentTag::Images]
        );
        assert_eq!(tags("<pre>let x = 1;</pre>"), [ContentTag::Code]);
        assert_eq!(tags("<p>Use <code>ls</code></p>"), [ContentTag::Code]);
        assert_eq!(tags("<p><strong>Bold</strong></p>"), [ContentTag::Styling]);
        assert_eq!(
            tags(r#"<p><span style="color: navy">Blue</span></p>"#),
            [ContentTag::Styling]
        );
    }

    #[test]
    fn ignores_layout_styles_and_text_that_looks_like_markup() {
        assert_eq!(
            tags(r#"<p style="text-align: center; margin: 0">Centered</p>"#),
            []
        );
        assert_eq!(tags(r#"<td style="border: 1px solid">x</td>"#), []);
        assert_eq!(tags("<p>&lt;table&gt; and &lt;code&gt;</p>"), []);
    }

    #[test]
    fn combines_tags_and_keeps_stable_bits() {
        let html = concat!(
            r#"<table><tbody><tr><td style="Background-Color: yellow">"#,
            "<code>x</code></td></tr></tbody></table>",
        );
        assert_eq!(
            tags(html),
            [ContentTag::Tables, ContentTag::Styling, ContentTag::Code]
        );
        assert_eq!(ContentTag::mask(&ContentTag::ALL), 0b1111);
        assert_eq!(
            ContentTag::from_mask(ContentTag::Images.bit()),
            [ContentTag::Images]
        );
    }
}
