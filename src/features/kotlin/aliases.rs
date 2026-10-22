//! The names serde reads a variant and a member under beside the one it writes: each
//! `#[serde(alias = "...")]`. The Kotlin read takes a payload under an alias as it takes one under
//! the written name, and the write keeps the written name, as serde writes it.

use core::fmt::Write as _;
use core::iter::once;

use syn::Attribute;

use super::{KotlinField, kotlin_quoted};
#[cfg(feature = "serde")]
use crate::features::serde::parse_serde_field_attributes;

/// The names `#[serde(alias = "...")]` writes on a variant or a field, in the order written.
#[cfg(feature = "serde")]
pub fn written_on(attrs: &[Attribute]) -> Vec<String> {
    parse_serde_field_attributes(attrs).aliases
}

/// Without `serde` no attribute is read, so nothing is read under an alias.
#[cfg(not(feature = "serde"))]
pub const fn written_on(_attrs: &[Attribute]) -> Vec<String> {
    Vec::new()
}

/// Each alias of a member of `fields`, beside the name the member is written under.
pub fn of_members(fields: &[KotlinField]) -> Vec<(String, String)> {
    fields
        .iter()
        .flat_map(|field| {
            field
                .aliases
                .iter()
                .map(|alias| (alias.clone(), field.wire_name.clone()))
        })
        .collect()
}

/// The arms of a Kotlin `when` over a name that answer each alias of `renamed` as `answer` writes
/// the name it stands for, each followed by `; `.
fn arms(renamed: &[(String, String)], answer: &dyn Fn(&str) -> String) -> String {
    renamed
        .iter()
        .fold(String::new(), |mut written, (alias, name)| {
            let _ = write!(
                written,
                "{} -> {}; ",
                kotlin_quoted(alias),
                answer(&kotlin_quoted(name))
            );
            written
        })
}

/// `payload`, the Kotlin expression of a `JsonElement`, with each key of an object that `renamed`
/// names an alias of read as the name the member is written under, as serde reads it: a member
/// under two of its names is refused, as serde refuses a duplicate field, and a payload that is
/// no object is as it is.
pub fn keys_named(payload: &str, renamed: &[(String, String)]) -> String {
    if renamed.is_empty() {
        return payload.to_owned();
    }
    format!(
        "{payload}.let {{ whole: JsonElement -> if (whole is JsonObject) \
         JsonObject(buildMap<String, JsonElement> {{ whole.forEach {{ (key, held) -> \
         val named = when (key) {{ {}else -> key }}; if (put(named, held) != null) \
         throw SerializationException(\"duplicate field `$named`\") }} }}) else whole }}",
        arms(renamed, &|name| name.to_owned())
    )
}

/// `payload`, the Kotlin expression of a `JsonElement`, with a string that `renamed` names an
/// alias of read as the name the variant is written under, as serde reads a plain enum.
pub fn text_named(payload: &str, renamed: &[(String, String)]) -> String {
    if renamed.is_empty() {
        return payload.to_owned();
    }
    format!(
        "{payload}.let {{ whole: JsonElement -> if (whole is JsonPrimitive && whole.isString) \
         when (whole.content) {{ {}else -> whole }} else whole }}",
        arms(renamed, &|name| format!("JsonPrimitive({name})"))
    )
}

/// `{Type}Unjudged`, the one serializer of a plain enum that reads a variant under an alias: it
/// writes each variant under its wire name, and reads it under that name and each alias, as serde
/// does. `variants` holds each variant's Kotlin name, wire name and aliases.
pub fn plain_enum_serializer(
    export_name: &str,
    variants: &[(String, String, Vec<String>)],
) -> String {
    let written = variants
        .iter()
        .map(|(name, wire, _)| format!("{export_name}.{name} -> {}", kotlin_quoted(wire)))
        .collect::<Vec<_>>()
        .join("; ");
    let read = variants
        .iter()
        .map(|(name, wire, aliases)| {
            let names = once(wire)
                .chain(aliases)
                .map(|read_under| kotlin_quoted(read_under))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{names} -> {export_name}.{name}")
        })
        .collect::<Vec<_>>()
        .join("; ");
    format!(
        "object {export_name}Unjudged : KSerializer<{export_name}> {{ override val descriptor: \
         SerialDescriptor = PrimitiveSerialDescriptor(\"{export_name}\", PrimitiveKind.STRING); \
         override fun serialize(encoder: Encoder, value: {export_name}) = \
         encoder.encodeString(when (value) {{ {written} }}); override fun deserialize(decoder: \
         Decoder): {export_name} = when (val name = decoder.decodeString()) {{ {read}; else -> \
         throw SerializationException(\"unknown variant \" + name) }} }}"
    )
}
