//! What the Swift emitter writes and reads of a flattened `Option`: the keys of the value it holds
//! into its holder's object, a unit variant of an externally tagged enum as an entry of its name
//! over `null`, and nothing for `nil`; `nil` where the object holds none of its type's keys, a
//! struct's, the names an externally tagged enum's variants are read under, or the tag and the
//! content key of an enum under a tag key, and the value its type reads from the object otherwise,
//! a read that throws refusing the payload; and, for a type whose keys are not known, `nil`
//! wherever its read throws. One that names a function reads as the Zod schema of its holder
//! reads it. A struct flattened bare writes what it did and reads the whole object. The writes and
//! the reads are run under the Swift toolchain in
//! `tests/service_schema_emitted_client_tests/run_swift_flattened_options.rs`.

use super::corrected_options_models::{moused_swift, reeved_swift, wormed_swift};
#[cfg(feature = "chrono")]
use super::flattened_options_models::swaging_swift;
use super::flattened_options_models::{
    crowning_swift, easing_swift, fidding_swift, flaking_swift, heeling_swift, kinking_swift,
    lapping_swift, lowering_swift, painter_swift, pitching_swift, ratline_swift, sennit_swift,
    squaring_swift, tarring_swift, veering_swift,
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

/// A flattened `Option` of an externally tagged enum written above its holder writes a unit
/// variant as an entry of its name over `null`, as serde writes it. One declared below its holder
/// and one under a tag key write through the holder's encoder, as they wrote.
#[test]
fn a_flattened_option_of_an_enum_writes_a_unit_variant_as_its_name_over_null() {
    assert_written(
        &easing_swift::swift_definition(),
        "if let held = self.thimble { switch held { case .calm: var entry = \
         encoder.container(keyedBy: SwiftSchemaAnyKey.self); try entry.encodeNil(forKey: \
         SwiftSchemaAnyKey(stringValue: \"Calm\")!); default: try held.encode(to: encoder) } }",
    );
    assert_written(
        &pitching_swift::swift_definition(),
        "if let held = self.shackle { if let plain = (held as Any) as? any RawRepresentable, \
         let name = plain.rawValue as? String { var entry = \
         encoder.container(keyedBy: SwiftSchemaAnyKey.self); try \
         entry.encodeNil(forKey: SwiftSchemaAnyKey(stringValue: name)!) } else { try \
         held.encode(to: encoder) } }",
    );
    assert_written(
        &lowering_swift::swift_definition(),
        "if let held = self.grommet { try held.encode(to: encoder) }",
    );
}

/// A flattened `Option` is read as `nil` where the whole object its holder reads holds none of its
/// type's keys, and as its type is read where it is flattened bare otherwise, so a read that throws
/// refuses the payload: alone in its holder, beside its holder's own keys, and in a variant under a
/// tag key. A key its holder reads too counts, as TypeScript and Zod read it. The keys are tested
/// through a key type private to the holder.
#[test]
fn a_flattened_option_reads_nil_where_none_of_its_types_keys_is_there() {
    let flaking = flaking_swift::swift_definition();
    assert_written(
        &flaking,
        "public init(swiftSchemaUnjudged decoder: Decoder) throws { if try \
         decoder.container(keyedBy: SwiftSchemaAnyKey.self).allKeys.contains(where: { \
         [\"label\"].contains($0.stringValue) }) { self.tabling = try Tabling(from: decoder) } \
         else { self.tabling = nil } }",
    );
    assert_written(
        &flaking,
        "private struct SwiftSchemaAnyKey: CodingKey { var stringValue: String; var intValue: \
         Int? { nil }; init?(stringValue: String) { self.stringValue = stringValue }; \
         init?(intValue: Int) { nil } }",
    );
    assert_written(
        &lapping_swift::swift_definition(),
        "self.note = try container.decode(String.self, forKey: .note); if try \
         decoder.container(keyedBy: SwiftSchemaAnyKey.self).allKeys.contains(where: { \
         [\"label\"].contains($0.stringValue) }) { self.tabling = try Tabling(from: decoder) } \
         else { self.tabling = nil } }",
    );
    assert_written(
        &sennit_swift::swift_definition(),
        "if try decoder.container(keyedBy: SwiftSchemaAnyKey.self).allKeys.contains(where: { \
         [\"label\"].contains($0.stringValue) }) { self.tabling = try Tabling(from: decoder) } \
         else { self.tabling = nil }; self.note = try container.decode(String.self, forKey: \
         .note) }",
    );
    assert_written(
        &painter_swift::swift_definition(),
        "public init(swiftSchemaUnjudged decoder: Decoder) throws { if try \
         decoder.container(keyedBy: SwiftSchemaAnyKey.self).allKeys.contains(where: { \
         [\"label\", \"turns\"].contains($0.stringValue) }) { self.gasket = try Gasket(from: \
         decoder) } else { self.gasket = nil } }",
    );
}

/// A flattened `Option` of a model `validate()` walks reads the model with no judgement where the
/// object holds one of its keys, and `nil` where it holds none.
#[test]
fn a_flattened_option_reads_a_walked_model_where_one_of_its_keys_is_there() {
    assert_written(
        &kinking_swift::swift_definition(),
        "public init(swiftSchemaUnjudged decoder: Decoder) throws { if try \
         decoder.container(keyedBy: SwiftSchemaAnyKey.self).allKeys.contains(where: { \
         [\"label\"].contains($0.stringValue) }) { self.plaiting = try \
         Plaiting(swiftSchemaUnjudged: decoder) } else { self.plaiting = nil } }",
    );
    assert_written(
        &tarring_swift::swift_definition(),
        "if try decoder.container(keyedBy: SwiftSchemaAnyKey.self).allKeys.contains(where: { \
         [\"label\"].contains($0.stringValue) }) { self.plaiting = try \
         Plaiting(swiftSchemaUnjudged: decoder) } else { self.plaiting = nil } }",
    );
}

/// A flattened `Option` of a type whose keys are not known is read as `nil` wherever its read
/// throws: a struct with a flattened member of its own, read from the whole object, and a struct
/// declared below its holder, read as a model.
#[test]
fn a_flattened_option_of_a_type_whose_keys_are_not_known_reads_nil_where_it_throws() {
    assert_written(
        &fidding_swift::swift_definition(),
        "public init(swiftSchemaUnjudged decoder: Decoder) throws { do { self.crowning = try \
         Crowning(from: decoder) } catch { self.crowning = nil } }",
    );
    let ratline = ratline_swift::swift_definition();
    assert_written(
        &ratline,
        "public init(swiftSchemaUnjudged decoder: Decoder) throws { do { self.nippering = try \
         Nippering(swiftSchemaUnjudged: decoder) } catch { self.nippering = nil } }",
    );
    let read = ratline.split_once("public init(from decoder").unwrap().0;
    assert!(
        !read.contains("SwiftSchemaAnyKey"),
        "a read that tests no key names no key type: {ratline}"
    );
}

/// A flattened `Option` of an enum is read as `nil` where the object it is handed holds none of the
/// enum's keys, and as the enum otherwise, a read that throws refusing the payload: an externally
/// tagged enum by the names its variants are read under, and an enum under a tag key alone by its
/// tag and one under a tag and a content key by both, in the whole object, as TypeScript and Zod
/// read them. An untagged enum, which has no keys, and an enum declared below its holder are read
/// as `nil` wherever their read throws.
#[test]
fn a_flattened_option_of_an_enum_reads_nil_where_none_of_its_keys_is_there() {
    assert_written(
        &easing_swift::swift_definition(),
        "if let thimbleKey = thimbleFound.first, let thimbleHeld = thimbleObject[thimbleKey] { \
         let thimblePayload: Any; switch thimbleKey { case \"Calm\" where thimbleHeld is \
         NSNull: thimblePayload = \"Calm\"; default: thimblePayload = [thimbleKey: thimbleHeld] \
         }; self.thimble = try SwiftSchemaPayload.read(Thimble.self, from: thimblePayload, at: \
         decoder.codingPath) } else { self.thimble = nil } }",
    );
    let tested = |keys: &str, member: &str, read_as: &str| {
        format!(
            "if try decoder.container(keyedBy: SwiftSchemaAnyKey.self).allKeys.contains(where: {{ \
             [{keys}].contains($0.stringValue) }}) {{ self.{member} = try {read_as}(from: \
             decoder) }} else {{ self.{member} = nil }}; self.note = try \
             container.decode(String.self, forKey: .note) }}"
        )
    };
    assert_written(
        &lowering_swift::swift_definition(),
        &tested("\"kind\"", "grommet", "Grommet"),
    );
    assert_written(
        &squaring_swift::swift_definition(),
        &tested("\"kind\", \"body\"", "becket", "Becket"),
    );
    assert_written(
        &heeling_swift::swift_definition(),
        "self.note = try container.decode(String.self, forKey: .note); do { let togglePayload = \
         try SwiftSchemaPayload(from: decoder, without: taken + [\"note\"]).value; self.toggle = \
         try SwiftSchemaPayload.read(Toggle.self, from: togglePayload, at: decoder.codingPath) } \
         catch { self.toggle = nil } }",
    );
    assert_written(
        &pitching_swift::swift_definition(),
        "self.note = try container.decode(String.self, forKey: .note); do { let shacklePayload = \
         try SwiftSchemaPayload(from: decoder, without: taken + [\"note\"]).value; \
         self.shackle = try SwiftSchemaPayload.read(SwiftSchemaReadShackle.self, from: \
         SwiftSchemaPayload.flat(SwiftSchemaReadShackle.self, shacklePayload, at: \
         decoder.codingPath), at: decoder.codingPath).value } catch { self.shackle = nil } }",
    );
}

/// A flattened `Option` that names a function reads its type from what the function answers, an
/// answer of `null` among it, and where that read throws is `nil` where the object the function is
/// handed holds none of its struct's keys, or no key at all for an enum, whose keys are not known,
/// and refuses the payload where it holds one, as the Zod schema of its holder reads it.
#[test]
fn a_flattened_option_that_names_a_function_reads_what_it_answers_as_zod_does() {
    assert_written(
        &veering_swift::swift_definition(),
        "self.note = try container.decode(String.self, forKey: .note); let tablingPayload = try \
         SwiftSchemaPayload(from: decoder, without: taken + [\"note\"]).value; do { self.tabling \
         = try SwiftSchemaPayload.read(Tabling.self, from: asGiven(tablingPayload), at: \
         decoder.codingPath) } catch { if [\"label\"].contains(where: { (tablingPayload as? \
         [String: Any] ?? [:])[$0] != nil }) { throw error }; self.tabling = nil } };",
    );
    assert_written(
        &reeved_swift::swift_definition(),
        "self.note = try container.decode(String.self, forKey: .note); let tholepinPayload = try \
         SwiftSchemaPayload(from: decoder, without: taken + [\"note\"]).value; do { \
         self.tholepin = try SwiftSchemaPayload.read(Tholepin.self, from: \
         asGiven(tholepinPayload), at: decoder.codingPath) } catch { if \
         [\"label\"].contains(where: { (tholepinPayload as? [String: Any] ?? [:])[$0] != nil }) { \
         throw error }; self.tholepin = nil } };",
    );
    assert_written(
        &wormed_swift::swift_definition(),
        "do { self.tholepin = try SwiftSchemaPayload.read(Tholepin.self, from: \
         asNull(tholepinPayload), at: decoder.codingPath) } catch { if \
         [\"label\"].contains(where: { (tholepinPayload as? [String: Any] ?? [:])[$0] != nil }) { \
         throw error }; self.tholepin = nil } };",
    );
    assert_written(
        &moused_swift::swift_definition(),
        "let frappedPayload = try SwiftSchemaPayload(from: decoder, without: taken + \
         [\"note\"]).value; do { self.frapped = try SwiftSchemaPayload.read(Frapped.self, from: \
         asGiven(frappedPayload), at: decoder.codingPath) } catch { if !(frappedPayload as? \
         [String: Any] ?? [:]).isEmpty { throw error }; self.frapped = nil }; self.note = try \
         container.decode(String.self, forKey: .note) };",
    );
}

/// A struct flattened bare is read from the whole object its holder reads, as TypeScript and Zod
/// read it: a read that throws refuses the payload.
#[test]
fn a_struct_flattened_bare_reads_the_whole_object() {
    assert_written(
        &crowning_swift::swift_definition(),
        "self.note = try container.decode(String.self, forKey: .note); self.tabling = try \
         Tabling(from: decoder) };",
    );
}
