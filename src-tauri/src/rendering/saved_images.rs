//! Short-lived image references keep saved PNG payloads out of HTML parsing.

use base64::Engine as _;

pub(super) struct CompactImages<'a> {
    html: String,
    resources: Vec<(String, &'a str)>,
}

impl<'a> CompactImages<'a> {
    pub(super) fn new(body: &'a str) -> Self {
        let mut html = String::with_capacity(body.len());
        let mut resources = Vec::new();
        let mut cursor = 0;
        let mut copied = 0;
        let nonce = uuid::Uuid::new_v4();
        while let Some(offset) = body[cursor..].find("<img") {
            let start = cursor + offset;
            cursor = start + 4;
            if !body
                .as_bytes()
                .get(cursor)
                .is_some_and(u8::is_ascii_whitespace)
            {
                continue;
            }
            let Some(end) = tag_end(body, cursor) else {
                break;
            };
            cursor = end;
            let tag = &body[start..end];
            // App-generated and DOM-serialized image attributes use double quotes.
            let Some(attribute) = tag.find(" src=\"data:image/png;base64,") else {
                continue;
            };
            let url_start = start + attribute + " src=\"".len();
            let Some(length) = body[url_start..end].find('"') else {
                continue;
            };
            let url_end = url_start + length;
            let url = &body[url_start..url_end];
            if !super::policy::is_generated_png_url(url) {
                continue;
            }
            let encoded = base64::engine::general_purpose::STANDARD
                .encode(format!("isoboard-{nonce}-{}", resources.len()));
            let reference = format!("data:image/png;base64,{encoded}");
            html.push_str(&body[copied..url_start]);
            html.push_str(&reference);
            copied = url_end;
            resources.push((reference, url));
        }
        html.push_str(&body[copied..]);
        Self { html, resources }
    }

    pub(super) fn html(&self) -> &str {
        &self.html
    }

    pub(super) fn restore(&self, mut html: String) -> String {
        for (reference, url) in &self.resources {
            html = html.replace(&format!("src=\"{reference}\""), &format!("src=\"{url}\""));
        }
        html
    }
}

fn tag_end(body: &str, mut cursor: usize) -> Option<usize> {
    while let Some(byte) = body.as_bytes().get(cursor) {
        match byte {
            b'>' => return Some(cursor + 1),
            b'"' | b'\'' => {
                let quote = *byte as char;
                cursor += 1;
                cursor += body[cursor..].find(quote)? + 1;
            }
            _ => cursor += 1,
        }
    }
    None
}
