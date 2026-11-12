//! The serializer the Kotlin emitter writes for a plain enum, as the emitter writes it.

use core::iter::once;

use super::kotlin_pathed::PATHED;

/// `{Type}Unjudged`, the serializer of the plain enum `named` of `variants`, each a Kotlin name, a
/// wire name and the aliases it is read under, which reads the payload `correction` answers.
pub fn plain_enum_unjudged(
    named: &str,
    variants: &[(&str, &str, &[&str])],
    correction: &str,
) -> String {
    let written = variants
        .iter()
        .map(|(name, wire, _)| format!("{named}.{name} -> \"{wire}\""))
        .collect::<Vec<_>>()
        .join("; ");
    let read = variants
        .iter()
        .map(|(name, wire, aliases)| {
            let listed = once(wire)
                .chain(aliases.iter())
                .map(|read_under| format!("\"{read_under}\""))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{listed} -> {named}.{name}")
        })
        .collect::<Vec<_>>()
        .join("; ");
    format!(
        "object {named}Unjudged : KSerializer<{named}> {{ override val descriptor: SerialDescriptor \
         = {named}.generatedSerializer().descriptor; override fun serialize(encoder: Encoder, \
         value: {named}) = encoder.encodeString(when (value) {{ {written} }}); {PATHED}override \
         fun deserialize(decoder: Decoder): {named} {{ val input = decoder as JsonDecoder; val \
         element = input.decodeJsonElement(); return kotlinSchemaAt(input) {{ val held = \
         {correction}; val entry = (held as? JsonObject)?.entries?.singleOrNull(); val name = if \
         (held is JsonPrimitive && held.isString) held.content else entry?.key ?: throw \
         SerializationException(\"expected value\"); val variant = when (name) {{ {read}; else -> \
         throw SerializationException(\"unknown variant `\" + name + \"`\") }}; if (entry != null \
         && entry.value !is JsonNull) kotlinSchemaUnder(\".\" + entry.key) {{ throw \
         SerializationException(\"invalid type: expected unit\") }}; variant }} }} }}"
    )
}
