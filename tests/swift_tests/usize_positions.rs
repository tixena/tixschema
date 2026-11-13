//! A `usize` is a `UInt64` in each place a member holds one, read through the fitted read of a
//! fixed-width integer, and a `literal` and a default of one are written as its digits.

use super::usize_positions_models::{placed_swift, slot_index_swift};

#[test]
fn a_usize_is_a_uint64_in_each_place_a_member_holds_one() {
    let placed = placed_swift::swift_definition();
    for expected in [
        "public var at: UInt64;",
        "public var defaulted: UInt64;",
        "public var held: Holding<UInt64>;",
        "public var indexed: [UInt64: String];",
        "public var keyed: [String: UInt64];",
        "public var listed: [UInt64];",
        "public var maybe: UInt64?;",
        "public struct PlacedPairedTuple: Codable, Sendable { let slot0: UInt64;",
        "public var pinned: UInt64;",
        "public var slot: SlotIndex;",
        "public struct PlacedSpanRange: Codable, Sendable { public let start: UInt64; public let \
         end: UInt64;",
        "public struct PlacedIndexedKeyedMap: Codable, Sendable { public var storage: [UInt64: \
         String];",
        "guard let key = UInt64(number) else {",
        "self.at = try container.decode(SwiftSchemaFit<UInt64>.self, forKey: .at).value;",
        "self.listed = try container.decode([SwiftSchemaFit<UInt64>].self, forKey: \
         .listed).map({ $0.value });",
    ] {
        assert!(placed.contains(expected), "no `{expected}` in: {placed}");
    }
    let brand = slot_index_swift::swift_definition();
    assert!(
        brand.contains("public struct SlotIndex: Codable, Sendable { public let value: UInt64;"),
        "got: {brand}"
    );
}

/// The fitted read names a `UInt64` `usize`, in serde's words.
#[test]
fn the_fitted_read_names_a_uint64_usize() {
    let placed = placed_swift::swift_definition();
    assert!(
        placed.contains("\"UInt32\": \"u32\", \"UInt64\": \"usize\"]"),
        "got: {placed}"
    );
}

#[test]
fn a_literal_and_a_default_of_a_usize_are_written_as_its_digits() {
    let placed = placed_swift::swift_definition();
    for expected in [
        "if pinned != 18446744073709551615 { errors.append(\"'pinned': not the literal \
         18446744073709551615, got \\(pinned)\") }",
        "Data(\"18446744073709551615\".utf8)",
    ] {
        assert!(placed.contains(expected), "no `{expected}` in: {placed}");
    }
}
