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

use super::corrected_options_models::{
    marled_swift, moused_swift, reeved_swift, seized_swift, served_swift, wormed_swift,
};
use super::flattened_options_models::{
    crowning_swift, easing_swift, fidding_swift, flaking_swift, heeling_swift, kinking_swift,
    lapping_swift, lowering_swift, painter_swift, pitching_swift, ratline_swift, sennit_swift,
    squaring_swift, tarring_swift, veering_swift,
};

/// What closes the statements that read a flattened member, which place a refusal inside it at
/// the holder.
const CAUGHT: &str = "catch let refused as DecodingError { throw \
     Self.swiftSchemaFlattened(refused, at: decoder.codingPath) }";

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
/// into.
#[test]
fn a_flattened_option_alone_opens_its_holders_object() {
    assert_written(
        &flaking_swift::swift_definition(),
        "public func encode(to encoder: Encoder) throws { _ = encoder.container(keyedBy: \
         CodingKeys.self); if let held = self.tabling { try held.encode(to: encoder) } }",
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
        "if let held = self.thimble { switch held.draft { case .calm: var entry = \
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
        "fileprivate init(swiftSchemaUnjudged decoder: Decoder, swiftSchemaWithout taken: \
         [String]) throws { do { if try decoder.container(keyedBy: \
         SwiftSchemaAnyKey.self).allKeys.contains(where: { \
         [\"label\"].contains($0.stringValue) }) { self.tabling = try \
         Tabling(swiftSchemaUnjudged: decoder, swiftSchemaWithout: taken) } else { \
         self.tabling = nil } }",
    );
    assert_written(
        &flaking,
        "private struct SwiftSchemaAnyKey: CodingKey { var stringValue: String; var intValue: \
         Int? { nil }; init?(stringValue: String) { self.stringValue = stringValue }; \
         init?(intValue: Int) { nil } }",
    );
    assert_written(
        &lapping_swift::swift_definition(),
        "self.note = try container.decode(String.self, forKey: .note); do { if try \
         decoder.container(keyedBy: SwiftSchemaAnyKey.self).allKeys.contains(where: { \
         [\"label\"].contains($0.stringValue) }) { self.tabling = try \
         Tabling(swiftSchemaUnjudged: decoder, swiftSchemaWithout: taken + [\"note\"]) } \
         else { self.tabling = nil } } catch let refused as DecodingError { throw \
         Self.swiftSchemaFlattened(refused, at: decoder.codingPath) } }",
    );
    assert_written(
        &sennit_swift::swift_definition(),
        "if try decoder.container(keyedBy: SwiftSchemaAnyKey.self).allKeys.contains(where: \
         { [\"label\"].contains($0.stringValue) }) { self.tabling = try \
         Tabling(swiftSchemaUnjudged: decoder, swiftSchemaWithout: taken + [\"note\"]) } \
         else { self.tabling = nil } } catch let refused as DecodingError { throw \
         Self.swiftSchemaFlattened(refused, at: decoder.codingPath) }; self.note = try \
         container.decode(String.self, forKey: .note) }",
    );
    assert_written(
        &painter_swift::swift_definition(),
        "fileprivate init(swiftSchemaUnjudged decoder: Decoder, swiftSchemaWithout taken: \
         [String]) throws { do { if try decoder.container(keyedBy: \
         SwiftSchemaAnyKey.self).allKeys.contains(where: { [\"label\", \
         \"turns\"].contains($0.stringValue) }) { self.gasket = try \
         Gasket(swiftSchemaUnjudged: decoder, swiftSchemaWithout: taken) } else { \
         self.gasket = nil } } catch let refused as DecodingError { throw \
         Self.swiftSchemaFlattened(refused, at: decoder.codingPath) } }",
    );
}

/// A flattened `Option` of a model `validate()` walks reads the model with no judgement where the
/// object holds one of its keys, and `nil` where it holds none.
#[test]
fn a_flattened_option_reads_a_walked_model_where_one_of_its_keys_is_there() {
    assert_written(
        &kinking_swift::swift_definition(),
        "fileprivate init(swiftSchemaUnjudged decoder: Decoder, swiftSchemaWithout taken: \
         [String]) throws { do { if try decoder.container(keyedBy: \
         SwiftSchemaAnyKey.self).allKeys.contains(where: { \
         [\"label\"].contains($0.stringValue) }) { self.plaiting = try \
         Plaiting(swiftSchemaUnjudged: decoder, swiftSchemaWithout: taken) } else { \
         self.plaiting = nil } }",
    );
    assert_written(
        &tarring_swift::swift_definition(),
        "if try decoder.container(keyedBy: SwiftSchemaAnyKey.self).allKeys.contains(where: { [\"label\"].contains($0.stringValue) }) { self.plaiting = try Plaiting(swiftSchemaUnjudged: decoder, swiftSchemaWithout: taken + [\"note\"]) } else { self.plaiting = nil } }",
    );
}

/// A flattened `Option` of a type whose keys are not known is read as `nil` wherever its read
/// throws: a struct with a flattened member of its own, read from the whole object. A struct
/// declared below its holder is looked for by the keys it publishes, read at run time.
#[test]
fn a_flattened_option_of_a_type_whose_keys_are_not_known_reads_nil_where_it_throws() {
    assert_written(
        &fidding_swift::swift_definition(),
        "fileprivate init(swiftSchemaUnjudged decoder: Decoder, swiftSchemaWithout taken: \
         [String]) throws { do { self.crowning = try Crowning(swiftSchemaUnjudged: decoder, \
         swiftSchemaWithout: taken) } catch { self.crowning = nil } }",
    );
    let ratline = ratline_swift::swift_definition();
    assert_written(
        &ratline,
        r#"var nipperingFound: [String] = []; let nipperingGroups: [[String]] = []; let nipperingUnits: [String] = []; if nipperingGroups.isEmpty { do { let nipperingStanding: [String] = ["label"]; if !nipperingStanding.isEmpty { if try decoder.container(keyedBy: SwiftSchemaAnyKey.self).allKeys.contains(where: { nipperingStanding.contains($0.stringValue) && !taken.contains($0.stringValue) }) { let nipperingPayload = try SwiftSchemaPayload(from: decoder, without: taken).value; self.nippering = try SwiftSchemaPayload.read(SwiftSchemaReadNippering.self, from: SwiftSchemaPayload.flat(SwiftSchemaReadNippering.self, nipperingPayload, at: decoder.codingPath), at: decoder.codingPath).value } else { self.nippering = nil } } else { do { let nipperingPayload = try SwiftSchemaPayload(from: decoder, without: taken).value; self.nippering = try SwiftSchemaPayload.read(SwiftSchemaReadNippering.self, from: SwiftSchemaPayload.flat(SwiftSchemaReadNippering.self, nipperingPayload, at: decoder.codingPath), at: decoder.codingPath).value } catch { self.nippering = nil } } } catch let refused as DecodingError { throw Self.swiftSchemaFlattened(refused, at: decoder.codingPath) } } else {"#,
    );
    assert_written(
        &ratline,
        "for unknown in try decoder.container(keyedBy: SwiftSchemaAnyKey.self).allKeys where \
         ![\"label\"].contains(unknown.stringValue)",
    );
}

/// A flattened `Option` of an enum is read as `nil` where the object it is handed holds none of the
/// enum's keys, and as the enum otherwise, a read that throws refusing the payload: an externally
/// tagged enum by the names its variants are read under, and an enum under a tag key alone by its
/// tag and one under a tag and a content key by both, in the whole object, as TypeScript and Zod
/// read them. Beside the `nonstrict` `Lowering` a read that throws is `nil`, as serde and Zod read
/// it. An untagged enum, which has no keys, is read as `nil` wherever its read throws, and an enum
/// declared below its holder is read as one declared above it is, from the names it publishes.
#[test]
fn a_flattened_option_of_an_enum_reads_nil_where_none_of_its_keys_is_there() {
    assert_written(
        &easing_swift::swift_definition(),
        &format!(
            "if let thimbleKey = thimbleFound.first, let thimbleHeld = \
             thimbleObject[thimbleKey] {{ do {{ let thimblePayload: Any; switch thimbleKey {{ case \
             \"Calm\" where thimbleHeld is NSNull: thimblePayload = \"Calm\"; default: \
             thimblePayload = [thimbleKey: thimbleHeld] }}; self.thimble = try \
             SwiftSchemaPayload.read(Thimble.self, from: thimblePayload, at: decoder.codingPath) \
             }} {CAUGHT} }} else {{ self.thimble = nil }} }}"
        ),
    );
    let tested = |keys: &str, member: &str, read_as: &str| {
        format!(
            "do {{ if try decoder.container(keyedBy: \
             SwiftSchemaAnyKey.self).allKeys.contains(where: {{ [{keys}].contains($0.stringValue) \
             }}) {{ self.{member} = try {read_as}(swiftSchemaUnjudged: decoder, swiftSchemaWithout: \
             taken + [\"note\"]) }} else {{ self.{member} = nil }} }} {CAUGHT}; self.note = try \
             container.decode(String.self, forKey: .note) }}"
        )
    };
    // A `nonstrict` enum takes no key the types around it took, and is read through its own read.
    assert_written(
        &lowering_swift::swift_definition(),
        "if try decoder.container(keyedBy: SwiftSchemaAnyKey.self).allKeys.contains(where: \
         { [\"kind\"].contains($0.stringValue) }) { do { self.grommet = try Grommet(from: \
         decoder) } catch { self.grommet = nil } } else { self.grommet = nil } } catch let \
         refused as DecodingError { throw Self.swiftSchemaFlattened(refused, at: \
         decoder.codingPath) }; self.note = try container.decode(String.self, forKey: \
         .note) }",
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
        r#"self.note = try container.decode(String.self, forKey: .note); var shackleFound: [String] = []; let shackleGroups: [[String]] = [["Calm"], ["Round"]]; let shackleUnits: [String] = []; if shackleGroups.isEmpty { do { let shackleStanding: [String] = []; if !shackleStanding.isEmpty { if try decoder.container(keyedBy: SwiftSchemaAnyKey.self).allKeys.contains(where: { shackleStanding.contains($0.stringValue) && !(taken + ["note"]).contains($0.stringValue) }) { let shacklePayload = try SwiftSchemaPayload(from: decoder, without: taken + ["note"]).value; self.shackle = try SwiftSchemaPayload.read(SwiftSchemaReadShackle.self, from: SwiftSchemaPayload.flat(SwiftSchemaReadShackle.self, shacklePayload, at: decoder.codingPath), at: decoder.codingPath).value } else { self.shackle = nil } } else { do { let shacklePayload = try SwiftSchemaPayload(from: decoder, without: taken + ["note"]).value; self.shackle = try SwiftSchemaPayload.read(SwiftSchemaReadShackle.self, from: SwiftSchemaPayload.flat(SwiftSchemaReadShackle.self, shacklePayload, at: decoder.codingPath), at: decoder.codingPath).value } catch { self.shackle = nil } } } catch let refused as DecodingError { throw Self.swiftSchemaFlattened(refused, at: decoder.codingPath) } } else {"#,
    );
}

/// A struct flattened bare is read from the whole object its holder reads, as TypeScript and Zod
/// read it: a read that throws refuses the payload.
#[test]
fn a_struct_flattened_bare_reads_the_whole_object() {
    assert_written(
        &crowning_swift::swift_definition(),
        "self.note = try container.decode(String.self, forKey: .note); do { self.tabling = \
         try Tabling(swiftSchemaUnjudged: decoder, swiftSchemaWithout: taken + [\"note\"]) \
         } catch let refused as DecodingError { throw Self.swiftSchemaFlattened(refused, \
         at: decoder.codingPath) } }; fileprivate init(swiftSchemaUnjudged decoder: Decoder) \
         throws { try self.init(swiftSchemaUnjudged: decoder, swiftSchemaWithout: []) }; \
         public init(from decoder: Decoder) throws { try self.init(swiftSchemaUnjudged: \
         decoder); let violations = validationIssues(); if !violations.isEmpty { throw \
         DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, \
         debugDescription: violations.joined(separator: \"; \"))) } }; public func \
         encode(to encoder: Encoder) throws { var container = encoder.container(keyedBy: \
         CodingKeys.self); try container.encode(self.note, forKey: .note); try \
         self.tabling.encode(to: encoder) };",
    );
}

/// Beside a `nonstrict` holder, an optional flattened member that names a function is `nil` where
/// its functions answer `null`, and is read from any other answer as a payload of its own, so a
/// read that throws, a key its type does not declare among them, refuses the payload, as serde
/// reads it. A member flattened with no `Option` is read from the answer the same way, through
/// the read its type declares beside the payload helper.
#[test]
fn a_nonstrict_holder_reads_an_optional_type_that_names_a_function_as_serde_does() {
    let note_and_tholepin = |function: &str| {
        format!(
            "self.note = try container.decode(String.self, forKey: .note); do {{ let \
             tholepinHandedPayload = try SwiftSchemaPayload(from: decoder, without: taken + \
             [\"note\"]).value; let tholepinPayload: Any = {function}(tholepinHandedPayload); if \
             tholepinPayload is NSNull {{ self.tholepin = nil }} else {{ self.tholepin = try \
             SwiftSchemaPayload.readRefusing(Tholepin.self, from: tholepinPayload, at: \
             decoder.codingPath) }} }} {CAUGHT} }};"
        )
    };
    assert_written(
        &reeved_swift::swift_definition(),
        &note_and_tholepin("asGiven"),
    );
    assert_written(
        &wormed_swift::swift_definition(),
        &note_and_tholepin("asNull"),
    );
    assert_written(
        &veering_swift::swift_definition(),
        &format!(
            "self.note = try container.decode(String.self, forKey: .note); do {{ let \
             tablingHandedPayload = try SwiftSchemaPayload(from: decoder, without: taken + \
             [\"note\"]).value; let tablingPayload: Any = asGiven(tablingHandedPayload); if \
             tablingPayload is NSNull {{ self.tabling = nil }} else {{ self.tabling = try \
             SwiftSchemaPayload.readRefusing(Tabling.self, from: tablingPayload, at: \
             decoder.codingPath) }} }} {CAUGHT} }};"
        ),
    );
    for holder in [
        moused_swift::swift_definition(),
        marled_swift::swift_definition(),
    ] {
        assert_written(
            &holder,
            &format!(
                "do {{ let frappedHandedPayload = try SwiftSchemaPayload(from: decoder, without: \
                 taken + [\"note\"]).value; let frappedPayload: Any = \
                 asGiven(frappedHandedPayload); if frappedPayload is NSNull {{ self.frapped = nil \
                 }} else {{ self.frapped = try SwiftSchemaPayload.readRefusing(Frapped.self, from: \
                 frappedPayload, at: decoder.codingPath) }} }} {CAUGHT}; self.note = try \
                 container.decode(String.self, forKey: .note) }};"
            ),
        );
    }
    assert_written(
        &served_swift::swift_definition(),
        &format!(
            "self.note = try container.decode(String.self, forKey: .note); do {{ let \
             tholepinHandedPayload = try SwiftSchemaPayload(from: decoder, without: taken + \
             [\"note\"]).value; let tholepinPayload: Any = asNull(tholepinHandedPayload); if \
             tholepinPayload is NSNull {{ self.tholepin = nil }} else {{ self.tholepin = try \
             SwiftSchemaPayload.readRefusing(Tholepin.self, from: tholepinPayload, at: \
             decoder.codingPath) }} }} {CAUGHT} }};"
        ),
    );
    assert_written(
        &seized_swift::swift_definition(),
        &format!(
            "do {{ let tholepinPayload = try SwiftSchemaPayload(from: decoder, without: taken + \
             [\"note\"]).value; self.tholepin = try \
             SwiftSchemaPayload.readRefusing(Tholepin.self, from: asGiven(tholepinPayload), at: \
             decoder.codingPath) }} {CAUGHT} }};"
        ),
    );
    assert_written(
        &seized_swift::swift_definition(),
        "static func readRefusing<Read: Decodable>",
    );
}
