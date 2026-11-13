//! A `usize` is a `ULong` in each place a member holds one, read through the serializer of one,
//! and a `literal` of one is compared as a `ULong` literal.

use super::usize_positions_models::{placed_kotlin, slot_index_kotlin};

/// `written` with each `@Serializable(with = ...::class)` annotation left out.
fn unannotated(written: &str) -> String {
    const OPENING: &str = "@Serializable(with = ";
    const CLOSING: &str = "::class) ";
    let mut left = String::new();
    let mut rest = written;
    while let Some(start) = rest.find(OPENING) {
        let Some(length) = rest[start..].find(CLOSING) else {
            break;
        };
        left.push_str(&rest[..start]);
        rest = &rest[start + length + CLOSING.len()..];
    }
    left.push_str(rest);
    left
}

#[test]
fn a_usize_is_a_ulong_in_each_place_a_member_holds_one() {
    let placed = placed_kotlin::kotlin_definition();
    let declared = unannotated(&placed);
    for expected in [
        "val at: ULong,",
        "val defaulted: ULong = ",
        "val held: Holding<ULong>,",
        "val indexed: Map<ULong, String>,",
        "val keyed: Map<String, ULong>,",
        "val listed: List<ULong>,",
        "val maybe: ULong? = null,",
        "(val slot0: ULong, val slot1: String)",
        "val pinned: ULong,",
        "val slot: SlotIndex,",
    ] {
        assert!(
            declared.contains(expected),
            "no `{expected}` in: {declared}"
        );
    }
    assert!(
        declared.contains("<ULong>) { fun validationIssues(): List<String>"),
        "the range's bound is no `ULong`: {declared}"
    );
    let brand = slot_index_kotlin::kotlin_definition();
    assert!(
        unannotated(&brand).contains("value class SlotIndex private constructor(val value: ULong)"),
        "got: {brand}"
    );
    // The serializers are declared once each, by the first type of the crate that reads one.
    let both = format!("{brand}\n{placed}");
    for expected in [
        ": KSerializer<ULong> {",
        "val read = number.toULongOrNull() ?: ",
    ] {
        assert!(both.contains(expected), "no `{expected}` in: {both}");
    }
}

#[test]
fn a_literal_and_a_default_of_a_usize_are_written_as_its_digits() {
    let placed = placed_kotlin::kotlin_definition();
    for expected in [
        "if (pinned != 18446744073709551615uL) violations.add(\"'pinned': not the literal \
         18446744073709551615, got ${pinned}\")",
        ", \"18446744073709551615\"),",
    ] {
        assert!(placed.contains(expected), "no `{expected}` in: {placed}");
    }
}

/// A `usize`'s serializer refuses a fraction and the string of its digits in serde's words.
#[test]
fn a_usize_serializer_refuses_in_serdes_words() {
    let placed = placed_kotlin::kotlin_definition();
    for expected in [
        "\"invalid type: \" + (if (element is JsonPrimitive) \"string \" else \"\") + element + \
         \", expected usize\"",
        "\"invalid type: floating point `\" + floating(wide) + \"`, expected usize\"",
    ] {
        assert!(placed.contains(expected), "no `{expected}` in: {placed}");
    }
}
