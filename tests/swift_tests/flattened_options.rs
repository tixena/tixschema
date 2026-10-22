//! What the Swift emitter writes and reads of a flattened `Option`: the keys of the value it holds
//! into its holder's object, and nothing for `nil`; the value its type reads from the object, and
//! `nil` wherever that read throws. A struct flattened bare writes and reads what it did. The
//! writes and the reads are run under the Swift toolchain in
//! `tests/service_schema_emitted_client_tests/run_swift_flattened_options.rs`.

#[cfg(feature = "chrono")]
use super::flattened_options_models::swaging_swift;
use super::flattened_options_models::{
    crowning_swift, fidding_swift, flaking_swift, kinking_swift, lapping_swift, sennit_swift,
    tarring_swift, veering_swift,
};

/// Asserts that `definition` holds `fragment`.
fn assert_written(definition: &str, fragment: &str) {
    assert!(
        definition.contains(fragment),
        "missing `{fragment}` in: {definition}"
    );
}

/// A flattened `Option` writes the value it holds through the holder's encoder, after the holder's
/// own member in a struct and ahead of it in a variant under a tag key.
#[test]
fn a_flattened_option_writes_the_value_it_holds_beside_its_holders_own_members() {
    assert_written(
        &lapping_swift::swift_definition(),
        "public func encode(to encoder: Encoder) throws { var container = \
         encoder.container(keyedBy: CodingKeys.self); try container.encode(self.note, forKey: \
         .note); if let held = self.tabling { try held.encode(to: encoder) } }",
    );
    assert_written(
        &sennit_swift::swift_definition(),
        "public func encode(to encoder: Encoder) throws { var container = \
         encoder.container(keyedBy: CodingKeys.self); if let held = self.tabling { try \
         held.encode(to: encoder) }; try container.encode(self.note, forKey: .note) }",
    );
}

/// A flattened `Option` alone in its holder opens the holder's object, which it may write nothing
/// into, and a map of dates is written as the text serde writes.
#[test]
fn a_flattened_option_alone_opens_its_holders_object() {
    assert_written(
        &flaking_swift::swift_definition(),
        "public func encode(to encoder: Encoder) throws { _ = encoder.container(keyedBy: \
         CodingKeys.self); if let held = self.tabling { try held.encode(to: encoder) } }",
    );
    #[cfg(feature = "chrono")]
    assert_written(
        &swaging_swift::swift_definition(),
        "public func encode(to encoder: Encoder) throws { _ = encoder.container(keyedBy: \
         CodingKeys.self); if let held = self.stamps { try held.mapValues { \
         Self.swiftSchemaFormatIso8601Date($0) }.encode(to: encoder) } }",
    );
}

/// A struct flattened bare writes through the holder's encoder, as it wrote before a flattened
/// `Option` had a write of its own.
#[test]
fn a_struct_flattened_bare_writes_what_it_wrote() {
    assert_written(
        &crowning_swift::swift_definition(),
        "public func encode(to encoder: Encoder) throws { var container = \
         encoder.container(keyedBy: CodingKeys.self); try container.encode(self.note, forKey: \
         .note); try self.tabling.encode(to: encoder) }",
    );
}

/// A flattened `Option` is read as its type is read where it is flattened bare, and as `nil`
/// wherever that read throws: from the holder's decoder alone in its holder, and from the object
/// without its holder's own keys beside them, in a variant under a tag key too.
#[test]
fn a_flattened_option_reads_nil_wherever_its_type_does_not_read() {
    assert_written(
        &flaking_swift::swift_definition(),
        "public init(swiftSchemaUnjudged decoder: Decoder) throws { do { self.tabling = try \
         Tabling(from: decoder) } catch { self.tabling = nil } }",
    );
    assert_written(
        &lapping_swift::swift_definition(),
        "self.note = try container.decode(String.self, forKey: .note); do { let tablingPayload = \
         try SwiftSchemaPayload(from: decoder, without: taken + [\"note\"]).value; self.tabling = \
         try SwiftSchemaPayload.read(Tabling.self, from: tablingPayload, at: decoder.codingPath) } \
         catch { self.tabling = nil } }",
    );
    assert_written(
        &sennit_swift::swift_definition(),
        "do { let tablingPayload = try SwiftSchemaPayload(from: decoder, without: taken + \
         [\"note\"]).value; self.tabling = try SwiftSchemaPayload.read(Tabling.self, from: \
         tablingPayload, at: decoder.codingPath) } catch { self.tabling = nil }; self.note = try \
         container.decode(String.self, forKey: .note) }",
    );
}

/// A flattened `Option` of a model `validate()` walks reads the model with no judgement, and one of
/// a type that reads the keys no member around it took hands it those keys; each reads `nil`
/// wherever that read throws.
#[test]
fn a_flattened_option_reads_a_walked_model_or_a_type_handed_keys_as_nil_where_it_throws() {
    assert_written(
        &kinking_swift::swift_definition(),
        "public init(swiftSchemaUnjudged decoder: Decoder) throws { do { self.plaiting = try \
         Plaiting(swiftSchemaUnjudged: decoder) } catch { self.plaiting = nil } }",
    );
    assert_written(
        &tarring_swift::swift_definition(),
        "do { let plaitingPayload = try SwiftSchemaPayload(from: decoder, without: taken + \
         [\"note\"]).value; self.plaiting = try \
         SwiftSchemaPayload.read(SwiftSchemaReadPlaiting.self, from: plaitingPayload, at: \
         decoder.codingPath).value } catch { self.plaiting = nil } }",
    );
    assert_written(
        &tarring_swift::swift_definition(),
        "private struct SwiftSchemaReadPlaiting: Decodable { let value: Plaiting; init(from \
         decoder: Decoder) throws { self.value = try Plaiting(swiftSchemaUnjudged: decoder) } }",
    );
    assert_written(
        &fidding_swift::swift_definition(),
        "swiftSchemaWithout taken: [String]) throws { do { self.crowning = try \
         Crowning(swiftSchemaUnjudged: decoder, swiftSchemaWithout: taken) } catch { \
         self.crowning = nil } }",
    );
}

/// A flattened `Option` that names a function reads its type from what the function answers, and
/// a read that throws refuses the payload.
#[test]
fn a_flattened_option_that_names_a_function_reads_what_it_answers() {
    assert_written(
        &veering_swift::swift_definition(),
        "self.note = try container.decode(String.self, forKey: .note); let tablingPayload = try \
         SwiftSchemaPayload(from: decoder, without: taken + [\"note\"]).value; self.tabling = try \
         SwiftSchemaPayload.read(Tabling?.self, from: asGiven(tablingPayload), at: \
         decoder.codingPath) };",
    );
}

/// A struct flattened bare reads what it read before a flattened `Option` read `nil` where its
/// type throws: a read that throws refuses the payload.
#[test]
fn a_struct_flattened_bare_reads_what_it_read() {
    assert_written(
        &crowning_swift::swift_definition(),
        "self.note = try container.decode(String.self, forKey: .note); let tablingPayload = try \
         SwiftSchemaPayload(from: decoder, without: taken + [\"note\"]).value; self.tabling = try \
         SwiftSchemaPayload.read(Tabling.self, from: tablingPayload, at: decoder.codingPath) };",
    );
}
