//! `()` in the Swift read and write: a struct of its own, read from the `null` serde writes for it
//! through a single-value container and from nothing else, and written as `null`. A tuple struct
//! of no slot, which is no `()`, is read from and written as `[]`, through an unkeyed container it
//! reads and writes nothing through.

use super::unit_models::{probe_nothing_swift, probe_slotless_swift, probe_units_swift};

/// The own read of the struct a `()` is held as.
const UNIT_READ: &str = "public init(swiftSchemaUnjudged decoder: Decoder) throws { let unit = \
     try decoder.singleValueContainer(); guard unit.decodeNil() else { let found = try \
     JSONSerialization.data(withJSONObject: SwiftSchemaPayload(from: decoder).value, options: \
     [.fragmentsAllowed, .sortedKeys, .withoutEscapingSlashes]); throw \
     DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, \
     debugDescription: \"invalid type: \\(String(decoding: found, as: UTF8.self)), expected \
     unit\")) } }; ";

/// The write of the struct a `()` is held as, and the `validate()` that ends it.
const UNIT_WRITE: &str = "public func encode(to encoder: Encoder) throws { var container = \
     encoder.singleValueContainer(); try container.encodeNil() }; public func validate() -> \
     [String] { [] }; public func swiftSchemaValidateBound() -> [String] { [] } };";

/// The text of the struct `named` declares in `definition`, up to the end of its `validate()`.
fn struct_text<'text>(definition: &'text str, named: &str) -> &'text str {
    let declared = format!("public struct {named}: Codable, Sendable {{ public init() {{}}; ");
    let held = definition
        .find(&declared)
        .and_then(|at| definition.get(at..))
        .unwrap_or_default();
    held.split_once(UNIT_WRITE).map_or(held, |(own, _)| own)
}

/// Each position `()` is held in reads it as a struct of its own, which reads `null` alone and
/// writes `null`, and reads no array. A holder declares its members as it did.
#[test]
fn unit_is_a_struct_read_from_null_and_written_as_null() {
    let units = probe_units_swift::swift_definition();
    let nothing = probe_nothing_swift::swift_definition();
    for (definition, named) in [
        (&units, "ProbeUnitsListedTuple"),
        (&units, "ProbeUnitsPairedTupleSlot1Tuple"),
        (&units, "ProbeUnitsUnitTuple"),
        (&nothing, "ProbeNothingTuple"),
    ] {
        let written = struct_text(definition, named);
        assert!(
            written.contains(UNIT_READ),
            "no read of null in: {definition}"
        );
        assert!(
            !written.contains("container = try decoder.unkeyedContainer()"),
            "{named} reads an array: {written}"
        );
        assert!(
            definition.contains(&format!("{written}{UNIT_WRITE}")),
            "{named} writes no null: {definition}"
        );
    }
    assert!(
        units.contains(
            "public let listed: [ProbeUnitsListedTuple]; public let nothing: ProbeNothing; public \
             let paired: ProbeUnitsPairedTuple; public let unit: ProbeUnitsUnitTuple;"
        ),
        "the members are no longer declared as they were: {units}"
    );
}

/// A tuple struct of no slot is read from an array of no item and written as one, through an
/// unkeyed container no statement reads or writes through, which Swift then warns of nothing.
#[test]
fn a_tuple_struct_of_no_slot_reads_and_writes_an_empty_array() {
    let slotless = probe_slotless_swift::swift_definition();
    for piece in [
        "public init(swiftSchemaUnjudged decoder: Decoder) throws { let container = try \
         decoder.unkeyedContainer(); if let count = container.count, count != 0 { throw \
         DecodingError.dataCorrupted(DecodingError.Context(codingPath: container.codingPath, \
         debugDescription: \"invalid length \\(count), expected 0\")) };  }",
        "public func encode(to encoder: Encoder) throws { _ = encoder.unkeyedContainer(); }",
    ] {
        assert!(slotless.contains(piece), "no {piece} in: {slotless}");
    }
}
