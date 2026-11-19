//! Shared parser for build-time JSON with comments and trailing commas.
use serde::de::DeserializeOwned;

pub(crate) fn parse<T: DeserializeOwned>(
    source: &str,
) -> Result<T, jsonc_parser::errors::ParseError> {
    jsonc_parser::parse_to_serde_value(
        source,
        &jsonc_parser::ParseOptions {
            allow_comments: true,
            allow_trailing_commas: true,
            allow_loose_object_property_names: false,
            allow_missing_commas: false,
            allow_single_quoted_strings: false,
            allow_hexadecimal_numbers: false,
            allow_unary_plus_numbers: false,
            allow_bare_decimal_point_numbers: false,
            allow_non_finite_numbers: false,
            allow_extended_string_escapes: false,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn accepts_jsonc_without_changing_urls_or_allowing_invalid_json_extensions() {
        let value: serde_json::Value = parse(
            r#"{
          // A URL contains slashes that are not comments.
          "url": "https://example.com/a//b", /* block comment */
          "text": "escaped \" quote", "items": [1, 2,],
        }"#,
        )
        .unwrap();
        assert_eq!(value["url"], "https://example.com/a//b");
        for source in [
            r#"{"a": 1 "b": 2}"#,
            "{unquoted: 1}",
            "{'a': 1}",
            "{\"a\": NaN}",
            "/* unfinished",
        ] {
            assert!(parse::<serde_json::Value>(source).is_err());
        }
    }
}
