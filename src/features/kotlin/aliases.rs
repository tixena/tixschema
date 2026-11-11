//! The names serde reads a variant and a member under beside the one it writes: each
//! `#[serde(alias = "...")]`. The Kotlin read takes a payload under an alias as it takes one under
//! the written name, and the write keeps the written name, as serde writes it.

use core::fmt::Write as _;
use core::iter::once;

use syn::Attribute;

use super::read::{PATHED, read_at};
use super::{KotlinField, kotlin_quoted};
#[cfg(feature = "serde")]
use crate::features::serde::parse_serde_field_attributes;
use crate::field_type::EXPECTED_VALUE;

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

/// `{Type}Unjudged`, the one serializer of a plain enum: it writes each variant under its wire
/// name, and reads it as serde does, from a string or from an object of one key over `null`, under
/// that name and each alias. A value of another kind is refused in serde's words for text that is
/// not the document, a value under the key other than `null` as a unit variant is refused, and a
/// name no variant takes as an unknown variant. `variants` holds each variant's Kotlin name, wire
/// name and aliases, and `correction` the Kotlin expression of the payload `element` holds,
/// corrected by the functions the enum names.
pub fn plain_enum_serializer(
    export_name: &str,
    variants: &[(String, String, Vec<String>)],
    correction: &str,
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
    let reads = format!(
        "val held = {correction}; val entry = (held as? JsonObject)?.entries?.singleOrNull(); val \
         name = if (held is JsonPrimitive && held.isString) held.content else entry?.key ?: throw \
         SerializationException(\"{EXPECTED_VALUE}\"); val variant = when (name) {{ {read}; else -> \
         throw SerializationException(\"unknown variant `\" + name + \"`\") }}; if (entry != null \
         && entry.value !is JsonNull) kotlinSchemaUnder(\".\" + entry.key) {{ throw \
         SerializationException(\"invalid type: expected unit\") }}; variant"
    );
    format!(
        "object {export_name}Unjudged : KSerializer<{export_name}> {{ override val descriptor: \
         SerialDescriptor = {export_name}.generatedSerializer().descriptor; \
         override fun serialize(encoder: Encoder, value: {export_name}) = \
         encoder.encodeString(when (value) {{ {written} }}); {PATHED}override fun \
         deserialize(decoder: Decoder): {export_name} {{ val input = decoder as JsonDecoder; {} }} }}",
        read_at("input", "input.decodeJsonElement()", &reads)
    )
}
