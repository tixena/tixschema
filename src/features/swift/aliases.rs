//! The names serde reads a variant and a member under beside the one it writes: each
//! `#[serde(alias = "...")]`. The Swift read takes a payload under an alias as it takes one under
//! the written name, and the write keeps the written name, as serde writes it.

use core::fmt::Write as _;

use syn::Attribute;

use super::swift_quoted;
use crate::bound_message::swift_literal_text;
#[cfg(feature = "serde")]
use crate::features::serde::parse_serde_field_attributes;

/// The container of every key of the object a struct's members are read from, which a member
/// serde reads under an alias too is read from: its key is the name the payload holds it under.
pub(super) const CONTAINER: &str =
    "let aliased = try decoder.container(keyedBy: SwiftSchemaAnyKey.self); ";

/// The names `#[serde(alias = "...")]` writes on a variant or a field, in the order written.
#[cfg(feature = "serde")]
pub(super) fn written_on(attrs: &[Attribute]) -> Vec<String> {
    parse_serde_field_attributes(attrs).aliases
}

/// Without `serde` no attribute is read, so nothing is read under an alias.
#[cfg(not(feature = "serde"))]
pub(super) const fn written_on(_attrs: &[Attribute]) -> Vec<String> {
    Vec::new()
}

/// The names a `case` of a `switch` over a variant's name takes the variant under: `wire`, then
/// each of `aliases`.
pub(super) fn case_names(wire: &str, aliases: &[String]) -> String {
    aliases.iter().fold(swift_quoted(wire), |mut names, alias| {
        write!(names, ", {}", swift_quoted(alias)).unwrap();
        names
    })
}

/// The `init?(rawValue:)` of a plain enum that reads a variant under an alias, in place of the one
/// Swift synthesizes: it takes each variant under its raw value and each alias, so every read of
/// the enum's raw value takes them, a map's key among them. `variants` holds each variant's case,
/// raw value and aliases. Empty where no variant has an alias.
pub(super) fn raw_value_init(variants: &[(String, String, Vec<String>)]) -> String {
    if variants.iter().all(|(_, _, aliases)| aliases.is_empty()) {
        return String::new();
    }
    let arms = variants
        .iter()
        .fold(String::new(), |mut arms, (case_name, wire, aliases)| {
            write!(
                arms,
                "case {}: self = .{case_name}; ",
                case_names(wire, aliases)
            )
            .unwrap();
            arms
        });
    format!(
        "public init?(rawValue: String) {{ switch rawValue {{ {arms}default: return nil }} }}; "
    )
}

/// The key a member serde reads under an alias too is read under, found by [`lookup`].
pub(super) fn key(stem: &str) -> String {
    format!("{stem}Key")
}

/// The statements that find under which of its names, `wire` then each of `aliases`, the object
/// [`CONTAINER`] reads holds the member `stem`: the one it holds, or `wire` where it holds none,
/// so an absent key is read as it was. A member under two of its names is refused, as serde
/// refuses a duplicate field.
pub(super) fn lookup(stem: &str, wire: &str, aliases: &[String]) -> String {
    let written = swift_literal_text(wire);
    format!(
        "let {stem}Names = aliased.allKeys.filter {{ [{}].contains($0.stringValue) }}; \
         if {stem}Names.count > 1 {{ throw DecodingError.dataCorrupted(DecodingError.Context(\
         codingPath: aliased.codingPath, debugDescription: \"duplicate field `{written}`\")) }}; \
         let {} = {stem}Names.first ?? SwiftSchemaAnyKey(stringValue: \"{written}\")!; ",
        case_names(wire, aliases),
        key(stem)
    )
}
