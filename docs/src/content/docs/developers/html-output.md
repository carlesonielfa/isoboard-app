---
title: HTML output contract
description: The HTML isoboard accepts from a model, and how the renderer sanitizes it, embeds image crops, cleans it up, derives plain text and bounds its size.
sidebar:
  label: HTML output
  order: 7
---

The conversion prompt defines the HTML a model must produce, and the renderer implements that contract after inference. The renderer accepts a full document, a body, or a recoverable fragment. It sanitizes with an allowlist, embeds validated image crops, applies the Profile's cleanup flags, and returns a canonical document, a clipboard body, and derived plain text.

The renderer must not remove a requested wrapper, style, table attribute, or image marker to simplify clipboard handling. A prompt change is a deliberate product-contract change. For the prompt itself see [Inference and prompts](../inference/#conversion-prompt). For publication to the clipboard and storage in History see [Clipboard and history](../clipboard-and-history/).

The code lives in [`src-tauri/src/rendering/`](https://github.com/carlesonielfa/isoboard-app/tree/main/src-tauri/src/rendering):

| File | Responsibility |
|---|---|
| `mod.rs` | Normalization entry points and size limits |
| `policy.rs` | Element, attribute, and URL allowlist (ammonia) |
| `styles.rs` | Inline CSS allowlist and value validation |
| `images.rs` | Marker validation, cropping, embedding |
| `conversion.rs` | Cleanup for the Tables, Colors, and Images flags |
| `split.rs` | Intermediate HTML and image references for the split pass |
| `plain_text.rs` | Plain-text derivation |
| `saved_images.rs` | Compaction of embedded crops while cleaning saved entries |

## Input and output

The renderer accepts the prompt's full `<html><body>...</body></html>` output, a `<body>` element, or a recoverable HTML fragment. It trims one complete HTML Markdown fence. Markup-free output is escaped and wrapped in a paragraph, with newlines turned into `<br>` so row breaks survive.

The result has two HTML forms:

| Form | Content | Used by |
|---|---|---|
| `document_html` | One canonical `<html><body>...</body></html>` document | Capture state, preview, History |
| `body_html` | The sanitized body fragment | The clipboard adapter, so native APIs get no nested document wrapper |

The result also carries the derived plain text and, for successful captures, a sanitized formatting baseline (see [Clipboard and history](../clipboard-and-history/#formatting-baseline)).

Malformed or misnested markup is repaired by ammonia's HTML5 parser. Unknown wrappers are flattened and active-content subtrees are removed.

## Limits

| Limit | Value | Where |
|---|---|---|
| Provider response before parsing | 1 MiB | `MAX_PROVIDER_OUTPUT_BYTES` |
| Canonical document after image embedding | 8 MiB | `MAX_DOCUMENT_BYTES` |
| Derived plain text | 512 KiB | `MAX_PLAIN_TEXT_BYTES` |
| Image markers per document | 32 | `MAX_MARKERS` |
| Embedded image data, total | 6 MiB | `MAX_EMBEDDED_IMAGE_BYTES` |
| One generated crop | 2 MiB | `MAX_PER_IMAGE_BYTES` |
| Source image | 20 MiB, 100 million pixels, 10,000 px per side | `images.rs` |

## Allowed structure and attributes

The sanitizer keeps the semantic elements the prompt names: headings, paragraphs, emphasis, links, lists, blockquotes, code, captions, and tables. It also accepts the common inline wrappers `span`, `b`, and `strike` when their content is safe.

| Item | Rule |
|---|---|
| Link `href` | Only `http`, `https`, and `mailto`. Relative, protocol-relative, and unsafe URLs lose their `href` |
| Table `colspan`, `rowspan` | Positive integers up to 1,000. Other values are dropped |
| Table `border` | Integers 1 through 16, because they express visible structure. Other table attributes are removed |
| Removed entirely | Scripts, forms, frames, external resources, SVG, raw-text containers (`plaintext`, `xmp`, `noembed`), classes, IDs, event handlers, metadata |

Image markers inside removed or nested active subtrees are ignored as content. They are not treated as provider image markers.

### Inline styles

`styles.rs` allows only the visual properties the prompt requests, and emits declarations in a stable order. The allowlist covers colors and background colors, alignment, font size, indentation, borders, margins, padding, spacing, border collapse, and table layout, plus bounded typography properties.

- Unknown properties are discarded.
- Values containing `url()`, `var()`, `calc()`, `attr()`, `expression()`, imports, comments, braces, or unsafe characters are discarded.
- Named colors use the CSS Color keyword list. `rgb()`, `rgba()`, `hsl()`, and `hsla()` accept bounded numeric components.
- Font size accepts bounded CSS lengths and the standard size keywords.
- Line height accepts bounded lengths or a unitless number from 0 through 10.

## Images

A model emits meaningful images as markers in reading order:

```html
<img data-x1="X1" data-y1="Y1" data-x2="X2" data-y2="Y2">
```

Only those four attributes survive the first sanitization. Provider `src`, `alt`, class, ID, style, event, and other attributes never survive.

### Marker validation

Each coordinate must occur exactly once, be finite and within `[0, 1]`, and satisfy `x1 < x2` and `y1 < y2`. Missing, duplicate, nonfinite, out-of-range, or reversed coordinates fail normalization instead of silently deleting content. More than 32 markers fail too. The prompt asks the model for images of at least 32 by 32 pixels, but the renderer enforces a relative cutoff instead: a valid crop that covers less than 0.1% of the screenshot area is omitted without failing the conversion. The cutoff applies to direct and separate-pass conversion.

### Cropping and embedding

The renderer receives the original validated PNG or JPEG bytes and dimensions from capture. It rejects bytes whose decoded size differs from the reported size.

1. Crop with floor for the upper-left corner and ceil for the lower-right corner, clamped to the decoded image.
2. Encode the crop as an app-generated PNG data URL.
3. Replace the marker with an `<img>` that has only that generated `src`.

Each crop's budget is the smaller of the per-crop limit and an even share of
the remaining aggregate budget across the crops still to generate. A crop that
does not fit is downscaled in steps until it does, so large photos and many
crops reduce image resolution instead of failing the capture. Unused share
carries over to later crops.

This preserves image position in reading order. The 6 MiB aggregate and 2 MiB per-crop limits bound overlap amplification. Repeated references to the same crop in second-pass output still fail when they exceed the aggregate limit. Embedded images stay usable after pasting because they do not depend on temporary source files.

## Conversion cleanup policies

The cleanup portion of a run applies the frozen feature flags after the last model call, depending on the application mode (see [Inference and prompts](../inference/#application-mode)).

| Flag off | Result |
|---|---|
| Tables | Table structure is removed. Rows are separated by line breaks and cells by single spaces. Merged-cell text is emitted once. Captions, reading order, and the content and formatting inside each cell are kept |
| Colors | Color styling is removed. Content and other formatting stay |
| Images | Image regions are omitted. Surrounding text and captions stay. Text inside an omitted region is not transcribed as a substitute |

The prompt forbids ASCII or box-drawing borders, Markdown tables, and column-alignment padding. Row breaks survive in both HTML and the derived plain text. The inventory of visual features beyond Tables, Colors, and Images is not settled.

## Split-stage intermediate

The separate Additional Rules pass receives safety-sanitized intermediate HTML, not the provider's original response and not base64 image data.

- During the first stage each valid crop marker is checked once and receives a short private run-local reference such as `i1` in `data-isoboard-image-ref`. Physical crops are prepared once before the second request, even when image conversion is disabled.
- The second stage is text-only. It may preserve, remove, reorder, or repeat known references. It may keep safe `alt`, `title`, `width`, and `height` attributes, with the CSS allowlist applying and dimensions capped at 10,000. It cannot create crop coordinates, external or data image URLs, or new references.
- Unknown or new references, missing or duplicate ref attributes, coordinates, and external or data sources make the second-stage output invalid.
- Known references stay inside the per-crop, aggregate, document, and 32-final-occurrence limits.
- Finalization resolves only references allocated for the prepared conversion, applies the deterministic cleanup after the last model call, embeds the generated crops, and then derives plain text.

Raw stage responses are source text. They are stored for Debug History and never rendered as markup. Clipboard and History publication happen once, after final cleanup and embedding. An invalid, failed, or cancelled second stage publishes neither. See [Inference and prompts](../inference/#split-stage-pass) for execution and configuration rules.

## Plain text

Plain text is derived from the final body after all sanitization and image replacement.

- Link destinations are not appended.
- Tables use spaces between cells and line breaks between rows. Captions and reading order are kept.
- No table borders, column-alignment padding, Markdown decorations, or strikethrough marks are added.
- Literal punctuation, paragraph and line breaks, and preformatted whitespace are kept.
- Image-only HTML is valid and may have an empty plain-text fallback, so the image can still be published.

History's copy actions derive text from the saved sanitized body, so older entries also use the current plain-text rules without changing the stored result. History text previews limit length without stripping punctuation or changing spacing.

## What is not covered

RTF remains later representation work. The shared clipboard path is not runtime-validated on Windows or Linux, and pure Wayland support is deferred. See [Clipboard and history](../clipboard-and-history/).

## Related

- [Inference and prompts](../inference/)
- [Clipboard and history](../clipboard-and-history/)
- [Architecture](../architecture/)
- [Product specification](../spec/#capture-and-clipboard)
