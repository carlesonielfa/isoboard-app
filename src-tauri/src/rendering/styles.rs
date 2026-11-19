//! Inline-style policy for provider HTML.
//!
//! Styles are data, not executable markup. The allowlist is deliberately
//! visible here so adding a property is a reviewed product decision.

use std::borrow::Cow;
use std::collections::BTreeMap;

const PROPERTIES: &[&str] = &[
    "background-color",
    "border",
    "border-bottom",
    "border-collapse",
    "border-color",
    "border-left",
    "border-right",
    "border-spacing",
    "border-style",
    "border-top",
    "border-width",
    "color",
    "font-size",
    "font-style",
    "font-weight",
    "line-height",
    "margin",
    "margin-bottom",
    "margin-left",
    "margin-right",
    "margin-top",
    "padding",
    "padding-bottom",
    "padding-left",
    "padding-right",
    "padding-top",
    "table-layout",
    "text-align",
    "text-decoration",
    "text-indent",
    "vertical-align",
    "white-space",
];

// CSS named colors from the maintained CSS Color keyword list. Keeping the
// list explicit prevents arbitrary identifiers from becoming CSS values.
const NAMED_COLORS: &[&str] = &[
    "aliceblue",
    "antiquewhite",
    "aqua",
    "aquamarine",
    "azure",
    "beige",
    "bisque",
    "black",
    "blanchedalmond",
    "blue",
    "blueviolet",
    "brown",
    "burlywood",
    "cadetblue",
    "chartreuse",
    "chocolate",
    "coral",
    "cornflowerblue",
    "cornsilk",
    "crimson",
    "cyan",
    "darkblue",
    "darkcyan",
    "darkgoldenrod",
    "darkgray",
    "darkgreen",
    "darkgrey",
    "darkkhaki",
    "darkmagenta",
    "darkolivegreen",
    "darkorange",
    "darkorchid",
    "darkred",
    "darksalmon",
    "darkseagreen",
    "darkslateblue",
    "darkslategray",
    "darkslategrey",
    "darkturquoise",
    "darkviolet",
    "deeppink",
    "deepskyblue",
    "dimgray",
    "dimgrey",
    "dodgerblue",
    "firebrick",
    "floralwhite",
    "forestgreen",
    "fuchsia",
    "gainsboro",
    "ghostwhite",
    "gold",
    "goldenrod",
    "gray",
    "green",
    "greenyellow",
    "grey",
    "honeydew",
    "hotpink",
    "indianred",
    "indigo",
    "ivory",
    "khaki",
    "lavender",
    "lavenderblush",
    "lawngreen",
    "lemonchiffon",
    "lightblue",
    "lightcoral",
    "lightcyan",
    "lightgoldenrodyellow",
    "lightgray",
    "lightgreen",
    "lightgrey",
    "lightpink",
    "lightsalmon",
    "lightseagreen",
    "lightskyblue",
    "lightslategray",
    "lightslategrey",
    "lightsteelblue",
    "lightyellow",
    "lime",
    "limegreen",
    "linen",
    "magenta",
    "maroon",
    "mediumaquamarine",
    "mediumblue",
    "mediumorchid",
    "mediumpurple",
    "mediumseagreen",
    "mediumslateblue",
    "mediumspringgreen",
    "mediumturquoise",
    "mediumvioletred",
    "midnightblue",
    "mintcream",
    "mistyrose",
    "moccasin",
    "navajowhite",
    "navy",
    "oldlace",
    "olive",
    "olivedrab",
    "orange",
    "orangered",
    "orchid",
    "palegoldenrod",
    "palegreen",
    "paleturquoise",
    "palevioletred",
    "papayawhip",
    "peachpuff",
    "peru",
    "pink",
    "plum",
    "powderblue",
    "purple",
    "rebeccapurple",
    "red",
    "rosybrown",
    "royalblue",
    "saddlebrown",
    "salmon",
    "sandybrown",
    "seagreen",
    "seashell",
    "sienna",
    "silver",
    "skyblue",
    "slateblue",
    "slategray",
    "slategrey",
    "snow",
    "springgreen",
    "steelblue",
    "tan",
    "teal",
    "thistle",
    "tomato",
    "turquoise",
    "violet",
    "wheat",
    "white",
    "whitesmoke",
    "yellow",
    "yellowgreen",
    "transparent",
    "currentcolor",
];

/// Return a deterministic, safe inline style string. Unsafe or unknown
/// declarations are discarded rather than interpreted by a browser.
pub(crate) fn sanitize(value: &str) -> Option<Cow<'_, str>> {
    if value.contains("/*") || value.contains("*/") || value.contains('{') || value.contains('}') {
        return None;
    }
    let mut declarations = BTreeMap::new();
    for declaration in value.split(';') {
        let Some((property, raw_value)) = declaration.split_once(':') else {
            continue;
        };
        let property = property.trim().to_ascii_lowercase();
        let value = raw_value.trim();
        if !PROPERTIES.contains(&property.as_str()) || !safe_value(value) || value.is_empty() {
            continue;
        }
        if !valid_for_property(&property, value) {
            continue;
        }
        declarations.insert(property, value.to_owned());
    }
    if declarations.is_empty() {
        return None;
    }
    Some(Cow::Owned(
        declarations
            .into_iter()
            .map(|(key, value)| format!("{key}: {value}"))
            .collect::<Vec<_>>()
            .join("; "),
    ))
}

/// Remove provider-authored color declarations while retaining safe layout and
/// typography declarations. Border shorthands need token-level filtering so
/// their width and style continue to apply without a color (including
/// `currentColor`).
pub(crate) fn remove_colors(value: &str) -> Option<String> {
    let mut declarations = BTreeMap::new();
    for declaration in value.split(';') {
        let Some((property, raw_value)) = declaration.split_once(':') else {
            continue;
        };
        let property = property.trim().to_ascii_lowercase();
        let value = raw_value.trim();
        if value.is_empty()
            || !PROPERTIES.contains(&property.as_str())
            || !safe_value(value)
            || !valid_for_property(&property, value)
        {
            continue;
        }
        if matches!(
            property.as_str(),
            "color" | "background-color" | "border-color"
        ) {
            continue;
        }
        let value = if matches!(
            property.as_str(),
            "border" | "border-top" | "border-right" | "border-bottom" | "border-left"
        ) {
            css_tokens(value)
                .into_iter()
                .filter(|part| !valid_color(part))
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            value.to_owned()
        };
        if !value.is_empty() {
            declarations.insert(property, value);
        }
    }
    (!declarations.is_empty()).then(|| {
        declarations
            .into_iter()
            .map(|(key, value)| format!("{key}: {value}"))
            .collect::<Vec<_>>()
            .join("; ")
    })
}

fn safe_value(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    if [
        "url(",
        "var(",
        "calc(",
        "attr(",
        "expression(",
        "@import",
        "javascript:",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
    {
        return false;
    }
    for (index, _) in value
        .char_indices()
        .filter(|(_, character)| *character == '(')
    {
        let prefix = &value[..index];
        let name_start = prefix
            .rfind(|character: char| !character.is_ascii_alphabetic())
            .map_or(0, |position| position + 1);
        let function = prefix[name_start..].to_ascii_lowercase();
        if function != "rgb" && function != "rgba" && function != "hsl" && function != "hsla" {
            return false;
        }
    }
    value.chars().all(|character| {
        character.is_ascii_alphanumeric()
            || matches!(
                character,
                '#' | '%' | '.' | ',' | '/' | '-' | '_' | '(' | ')' | ' ' | '\t' | '+'
            )
    })
}

fn valid_for_property(property: &str, value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    match property {
        "text-align" => matches!(
            lower.as_str(),
            "left" | "right" | "center" | "justify" | "start" | "end"
        ),
        "vertical-align" => matches!(
            lower.as_str(),
            "baseline" | "top" | "middle" | "bottom" | "sub" | "super" | "text-top" | "text-bottom"
        ),
        "border-collapse" => matches!(lower.as_str(), "collapse" | "separate"),
        "table-layout" => matches!(lower.as_str(), "auto" | "fixed"),
        "white-space" => matches!(
            lower.as_str(),
            "normal" | "nowrap" | "pre" | "pre-wrap" | "pre-line"
        ),
        "font-style" => matches!(lower.as_str(), "normal" | "italic" | "oblique"),
        "font-weight" => {
            matches!(lower.as_str(), "normal" | "bold" | "bolder" | "lighter")
                || lower
                    .parse::<u16>()
                    .is_ok_and(|n| (100..=900).contains(&n) && n % 100 == 0)
        }
        "text-decoration" => lower
            .split_whitespace()
            .all(|part| matches!(part, "none" | "underline" | "overline" | "line-through")),
        "color" | "background-color" | "border-color" => valid_color(value),
        "font-size" => valid_font_size(value),
        "line-height" => valid_line_height(value),
        "text-indent" | "border-spacing" | "margin" | "margin-top" | "margin-right"
        | "margin-bottom" | "margin-left" | "padding" | "padding-top" | "padding-right"
        | "padding-bottom" | "padding-left" | "border-width" => valid_length_list(value),
        "border" | "border-top" | "border-right" | "border-bottom" | "border-left"
        | "border-style" => css_tokens(value).into_iter().all(|part| {
            matches!(
                part.to_ascii_lowercase().as_str(),
                "none"
                    | "hidden"
                    | "dotted"
                    | "dashed"
                    | "solid"
                    | "double"
                    | "groove"
                    | "ridge"
                    | "inset"
                    | "outset"
                    | "thin"
                    | "medium"
                    | "thick"
            ) || valid_color(part)
                || valid_length_list(part)
        }),
        _ => false,
    }
}

fn css_tokens(value: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut start = None;
    let mut depth = 0_u32;
    for (index, character) in value.char_indices() {
        match character {
            '(' => {
                depth = depth.saturating_add(1);
                if start.is_none() {
                    start = Some(index);
                }
            }
            ')' => depth = depth.saturating_sub(1),
            character if character.is_ascii_whitespace() && depth == 0 => {
                if let Some(start) = start.take() {
                    tokens.push(&value[start..index]);
                }
            }
            _ if start.is_none() => start = Some(index),
            _ => {}
        }
    }
    if let Some(start) = start {
        tokens.push(&value[start..]);
    }
    tokens
}

fn valid_color(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    NAMED_COLORS.contains(&lower.as_str())
        || (lower.starts_with('#')
            && (lower.len() == 4 || lower.len() == 7 || lower.len() == 9)
            && lower[1..].chars().all(|c| c.is_ascii_hexdigit()))
        || valid_color_function(&lower, "rgb", false)
        || valid_color_function(&lower, "rgba", true)
        || valid_color_function(&lower, "hsl", false)
        || valid_color_function(&lower, "hsla", true)
}

fn valid_color_function(value: &str, name: &str, alpha: bool) -> bool {
    let prefix = format!("{name}(");
    let Some(inner) = value
        .strip_prefix(&prefix)
        .and_then(|value| value.strip_suffix(')'))
    else {
        return false;
    };
    let parts = inner.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() != if alpha { 4 } else { 3 } {
        return false;
    }
    if name == "hsl" {
        let hue = parts[0].parse::<f32>().ok();
        let saturation = parts[1]
            .strip_suffix('%')
            .and_then(|value| value.parse::<f32>().ok());
        let lightness = parts[2]
            .strip_suffix('%')
            .and_then(|value| value.parse::<f32>().ok());
        if !hue.is_some_and(|value| value.is_finite() && (0.0..=360.0).contains(&value))
            || !saturation.is_some_and(|value| value.is_finite() && (0.0..=100.0).contains(&value))
            || !lightness.is_some_and(|value| value.is_finite() && (0.0..=100.0).contains(&value))
        {
            return false;
        }
    } else if !parts[..3].iter().all(|value| {
        value
            .parse::<f32>()
            .is_ok_and(|number| number.is_finite() && (0.0..=255.0).contains(&number))
    }) {
        return false;
    }
    !alpha
        || parts[3]
            .parse::<f32>()
            .is_ok_and(|value| value.is_finite() && (0.0..=1.0).contains(&value))
}

fn valid_font_size(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "xx-small"
            | "x-small"
            | "small"
            | "medium"
            | "large"
            | "x-large"
            | "xx-large"
            | "xxx-large"
            | "smaller"
            | "larger"
    ) || valid_length_list(value)
}

fn valid_line_height(value: &str) -> bool {
    value
        .parse::<f32>()
        .is_ok_and(|number| number.is_finite() && (0.0..=10.0).contains(&number))
        || valid_length_list(value)
}

fn valid_length_list(value: &str) -> bool {
    value.split_whitespace().all(|part| {
        matches!(part, "0" | "auto" | "normal")
            || part
                .strip_suffix("px")
                .or_else(|| part.strip_suffix("pt"))
                .or_else(|| part.strip_suffix("em"))
                .or_else(|| part.strip_suffix("rem"))
                .or_else(|| part.strip_suffix("%"))
                .is_some_and(|number| {
                    number
                        .parse::<f32>()
                        .is_ok_and(|n| n.is_finite() && (-10000.0..=10000.0).contains(&n))
                })
    })
}

#[cfg(test)]
mod tests {
    use super::sanitize;

    #[test]
    fn keeps_safe_properties_in_stable_order() {
        assert_eq!(
            sanitize("color: #123456; text-align: center; padding: 4px").unwrap(),
            "color: #123456; padding: 4px; text-align: center"
        );
    }

    #[test]
    fn removes_color_properties_and_color_tokens_from_borders() {
        assert_eq!(
            super::remove_colors(
                "color: red; background-color: #fff; border: 2px solid currentColor; padding: 4px"
            ),
            Some("border: 2px solid; padding: 4px".to_owned())
        );
        for color in ["rgb(1, 2, 3)", "hsl(180, 50%, 50%)"] {
            assert_eq!(
                super::remove_colors(&format!("border: 2px solid {color}; padding: 4px")),
                Some("border: 2px solid; padding: 4px".to_owned())
            );
        }
    }

    #[test]
    fn drops_unsafe_values_and_unknown_properties() {
        assert!(
            sanitize("color: red; width: calc(100%); background: url(x)")
                .unwrap()
                .eq("color: red")
        );
        assert!(sanitize("color: red; /* bad */").is_none());
        assert_eq!(
            sanitize("color: rgb(1, 2, 3); padding: mystery(1px)").unwrap(),
            "color: rgb(1, 2, 3)"
        );
        assert!(
            sanitize("color: hsl(180, 50%, 50%); line-height: 1.5; font-size: xx-large").is_some()
        );
        assert!(sanitize("color: hsl(720, 50%, 50%)").is_none());
    }
}
