//! `()` in the Kotlin read and write: an object of its own, declared once per item beside the
//! serializer that writes it as the `null` serde writes for `()` and reads that `null` alone. A
//! tuple struct of no slot, which is no `()`, is a plain class read from and written as `[]`.

use super::unit_models::{probe_nothing_kotlin, probe_slotless_kotlin, probe_units_kotlin};

/// The name `definition` declares the object of `()` under, `KotlinUnit` numbered.
fn unit_named(definition: &str) -> String {
    let at = definition.find(" object KotlinUnit").unwrap_or_default() + " object ".len();
    definition
        .get(at..)
        .unwrap_or_default()
        .chars()
        .take_while(char::is_ascii_alphanumeric)
        .collect()
}

/// The object `named` and its serializer, as the emitter declares them.
fn unit_declared(named: &str) -> String {
    format!(
        "@Serializable(with = {named}Serializer::class) object {named} object {named}Serializer : \
         KSerializer<{named}> {{ override val descriptor: SerialDescriptor = \
         buildClassSerialDescriptor(\"{named}\"); override fun serialize(encoder: Encoder, value: \
         {named}) {{ (encoder as JsonEncoder).encodeJsonElement(JsonNull) }}; override fun \
         deserialize(decoder: Decoder): {named} {{ val payload = (decoder as \
         JsonDecoder).decodeJsonElement(); if (payload !is JsonNull) throw \
         SerializationException(\"invalid type: \" + payload + \", expected unit\"); return \
         {named} }} }}"
    )
}

/// An item that holds `()` in several places declares one object for it, which a field, a list's
/// item and a tuple's position name, and a brand over `()` holds one of its own.
#[test]
fn unit_is_one_object_per_item_read_from_null_and_written_as_null() {
    let units = probe_units_kotlin::kotlin_definition();
    let named = unit_named(&units);
    assert!(
        named.starts_with("KotlinUnit"),
        "no object of `()` in: {units}"
    );
    assert!(units.contains(&unit_declared(&named)), "got: {units}");
    assert_eq!(
        units.matches(") object KotlinUnit").count(),
        1,
        "two objects of `()` in: {units}"
    );
    for held in [
        format!("val listed: List<{named}>"),
        format!("val slot1: {named})"),
        format!("val unit: {named})"),
    ] {
        assert!(units.contains(&held), "no {held} in: {units}");
    }
    assert_eq!(
        units.matches("data class KotlinTuple").count(),
        1,
        "a class is declared for `()` beside the pair's: {units}"
    );
    let nothing = probe_nothing_kotlin::kotlin_definition();
    let brand_unit = unit_named(&nothing);
    assert!(
        nothing.contains(&unit_declared(&brand_unit)),
        "got: {nothing}"
    );
    assert!(
        nothing.contains(&format!(
            "value class ProbeNothing(val value: {brand_unit})"
        )),
        "got: {nothing}"
    );
}

/// A tuple struct of no slot is a plain class, which kotlinc takes where it refuses a data class
/// of no property, read from an array of no item and written as one.
#[test]
fn a_tuple_struct_of_no_slot_is_a_class_read_from_an_empty_array() {
    let slotless = probe_slotless_kotlin::kotlin_definition();
    for piece in [
        "@Serializable(with = ProbeSlotlessSerializer::class) class ProbeSlotless() {",
        "output.encodeJsonElement(buildJsonArray {  })",
        "if (array.size != 0) throw SerializationException(\"invalid length \" + array.size + \", \
         expected 0\"); return ProbeSlotless()",
    ] {
        assert!(slotless.contains(piece), "no {piece} in: {slotless}");
    }
}
