//! What the Swift backend writes for the read of each shape: the type's own read under
//! `init(swiftSchemaUnjudged:)`, and `init(from:)`, which judges the value read. What a read
//! answers is run under a Swift toolchain in
//! `service_schema_emitted_client_tests/run_swift_read.rs`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use crate::tests::{
    FlattenedExtra, adjacent_tagged_swift, all_widths_swift, by_bool_swift, by_enum_swift,
    by_u32_swift, correlation_id_swift, envelope_swift, external_tagged_swift,
    flattened_extra_schema, flattened_extra_swift, internal_tagged_swift, reserved_member_swift,
    reserved_members_swift, status_swift, tuple_point_swift, untagged_swift, user_data_swift,
    window_request_swift, with_flatten_swift, with_nullable_swift,
};
use crate::validate::{
    Parcel, Slug, freight_swift, inspection_swift, manifest_swift, marker_swift, parcel_schema,
    parcel_swift, pier_swift, reading_swift, routing_swift, sealed_slug_swift, sealed_swift,
    slug_schema, slug_swift,
};

/// What opens the read of a type alone.
const UNJUDGED: &str = "public init(swiftSchemaUnjudged decoder: Decoder) throws";

/// The read every caller makes: the type's own read, then `validate()` on the value read.
const JUDGED: &str = concat!(
    "public init(from decoder: Decoder) throws { try self.init(swiftSchemaUnjudged: decoder); ",
    "let violations = validate(); if !violations.isEmpty { throw DecodingError.dataCorrupted(",
    "DecodingError.Context(codingPath: decoder.codingPath, ",
    r#"debugDescription: violations.joined(separator: "; "))) } }"#,
);

/// What opens the read a wrapper of a map's keys makes of the values it holds unjudged.
const UNJUDGED_IN_A_WRAPPER: &str = concat!(
    "init(swiftSchemaUnjudged decoder: Decoder) throws { ",
    "let container = try decoder.container(keyedBy: DynamicCodingKey.self)",
);

/// An optional and a nullable member over a string and over a model, in a struct whose read Swift
/// synthesized before one was written.
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mooring {
    #[model_schema_prop(nullable)]
    pub berth_note: Option<String>,
    #[model_schema_prop(nullable)]
    pub held_parcel: Option<Parcel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare_note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare_parcel: Option<Parcel>,
}

/// The same nullable members in a struct whose read was written before: it flattens a member.
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Anchorage {
    #[model_schema_prop(nullable)]
    pub anchor_note: Option<String>,
    #[serde(flatten)]
    pub flattened: FlattenedExtra,
    #[model_schema_prop(nullable)]
    pub held_parcel: Option<Parcel>,
}

/// A model under two wrappers at once, in a tuple, and in a map under each kind of key.
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stowage {
    pub by_deck: HashMap<isize, Parcel>,
    pub by_number: HashMap<u32, Parcel>,
    pub deck_pair: (String, Parcel),
    pub grid: Vec<Vec<Parcel>>,
    pub lists_by_dock: HashMap<String, Vec<Parcel>>,
    pub maybe_each: Vec<Option<Parcel>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare_list: Option<Vec<Parcel>>,
}

/// A flattened member that has a rule.
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Berthed {
    #[serde(flatten)]
    pub parcel: Parcel,
    pub quay: String,
}

/// A tuple struct of several slots: a bound on the first, a model in the second.
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Consigned(#[model_schema_prop(minLength = 1)] pub String, pub Parcel);

/// A unit struct.
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Idle;

/// An enum with no tag attribute that holds a model in a slot, a list of them, and in a tuple.
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all_fields = "camelCase")]
pub enum Carried {
    Boxed(Parcel),
    Listed(Vec<Parcel>),
    Loose { loose_label: String },
    Split(String, Parcel),
    Unladen,
}

/// An enum under a tag alone.
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all_fields = "camelCase")]
pub enum Lifted {
    Boxed(Parcel),
    Loose { loose_label: String },
    Unladen,
}

/// An enum under a tag and a content key.
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "held", rename_all_fields = "camelCase")]
pub enum Hauled {
    Boxed(Parcel),
    Loose { loose_label: String },
    Split(String, Parcel),
    Unladen,
}

/// An untagged enum: a bound on a slot, a brand with a bound, a bound on a named member, a list
/// of models and a model.
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged, rename_all_fields = "camelCase")]
pub enum Sorted {
    Brief(#[model_schema_prop(maxLength = 3)] String),
    Coded(Slug),
    Counted {
        #[model_schema_prop(minimum = 1)]
        sorted_count: u32,
    },
    Listed(Vec<Parcel>),
    Whole(Parcel),
    Wordy(String),
}

/// A plain enum that names a check.
#[model_schema(check = ["withARating"])]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rating {
    High,
    Low,
}

fn with_a_rating(value: &Rating) -> Result<(), Vec<String>> {
    if matches!(value, Rating::Low) {
        return Err(vec!["a low rating is not carried".to_owned()]);
    }
    Ok(())
}

/// Asserts each of `written` is in `swift`.
fn holds(swift: &str, written: &[&str]) {
    for expected in written {
        assert!(swift.contains(expected), "{expected} is not in: {swift}");
    }
}

/// The memberwise initializer, the type's own read, then the read that judges. `encode(to:)`
/// stays the one Swift synthesizes.
#[test]
fn a_struct_reads_its_members_then_judges_the_value_it_read() {
    let swift = parcel_swift::swift_definition();
    let written = format!(
        "{}{UNJUDGED} {}; {JUDGED}; public func validate() -> [String] {{ ",
        concat!(
            "public init(fragileItemCodes: [String], itemCodeList: [String], label: String) ",
            "{ self.fragileItemCodes = fragileItemCodes; self.itemCodeList = itemCodeList; ",
            "self.label = label;  }; ",
        ),
        concat!(
            "{ let container = try decoder.container(keyedBy: CodingKeys.self); ",
            "self.fragileItemCodes = try container.decode([String].self, forKey: .fragileItemCodes); ",
            "self.itemCodeList = try container.decode([String].self, forKey: .itemCodeList); ",
            "self.label = try container.decode(String.self, forKey: .label) }",
        ),
    );
    assert!(swift.contains(&written), "{written} is not in: {swift}");
    assert!(!swift.contains("func encode(to encoder:"), "got: {swift}");
}

/// A model a holder names is read through its own unjudged initializer, handed a decoder of its
/// own: in a field, a list, a map and an optional. A key that is absent stays the error it was.
#[test]
fn a_holder_reads_each_model_it_names_unjudged() {
    let swift = manifest_swift::swift_definition();
    let readme = include_str!("../../README.md");
    for statements in [
        vec![
            "_ = try container.decodeNil(forKey: .firstParcel)",
            concat!(
                "self.firstParcel = try Parcel(swiftSchemaUnjudged: ",
                "container.superDecoder(forKey: .firstParcel))",
            ),
        ],
        vec![
            "var listed = try container.nestedUnkeyedContainer(forKey: .parcelList)",
            "var items: [Parcel] = []",
            concat!(
                "while !listed.isAtEnd { items.append(try Parcel(swiftSchemaUnjudged: ",
                "listed.superDecoder())) }",
            ),
            "self.parcelList = items",
        ],
        vec![
            concat!(
                "let keyed = try container.nestedContainer(keyedBy: SwiftSchemaAnyKey.self, ",
                "forKey: .parcelsByDock)",
            ),
            "var entries: [String: Parcel] = [:]",
            concat!(
                "for entry in keyed.allKeys { entries[entry.stringValue] = try Parcel(",
                "swiftSchemaUnjudged: keyed.superDecoder(forKey: entry)) }",
            ),
            "self.parcelsByDock = entries",
        ],
        vec![concat!(
            "if container.contains(.spareParcel), try !container.decodeNil(forKey: .spareParcel) ",
            "{ self.spareParcel = try Parcel(swiftSchemaUnjudged: ",
            "container.superDecoder(forKey: .spareParcel)) } else { self.spareParcel = nil }",
        )],
    ] {
        let written = statements.join("; ");
        assert!(swift.contains(&written), "{written} is not in: {swift}");
        for shown in statements {
            assert!(
                readme.contains(shown),
                "the README no longer shows: {shown}"
            );
        }
    }
    holds(
        &swift,
        &[
            concat!(
                "private struct SwiftSchemaAnyKey: CodingKey { var stringValue: String; ",
                "var intValue: Int? { nil }; ",
                "init?(stringValue: String) { self.stringValue = stringValue }; ",
                "init?(intValue: Int) { nil } }; ",
            ),
            JUDGED,
        ],
    );
    assert!(!swift.contains("func encode(to encoder:"), "got: {swift}");
}

/// One read of its own and one that judges per struct and enum the emitter writes. A wrapper of a
/// map's keys is the form a map is read in and is judged by nothing, and a plain enum that names
/// no check keeps the read Swift synthesizes.
#[test]
fn every_emitted_type_has_both_reads() {
    for swift in [
        adjacent_tagged_swift::swift_definition(),
        all_widths_swift::swift_definition(),
        by_bool_swift::swift_definition(),
        by_enum_swift::swift_definition(),
        by_u32_swift::swift_definition(),
        correlation_id_swift::swift_definition(),
        envelope_swift::swift_definition(),
        external_tagged_swift::swift_definition(),
        flattened_extra_swift::swift_definition(),
        freight_swift::swift_definition(),
        inspection_swift::swift_definition(),
        internal_tagged_swift::swift_definition(),
        marker_swift::swift_definition(),
        pier_swift::swift_definition(),
        reading_swift::swift_definition(),
        reserved_member_swift::swift_definition(),
        reserved_members_swift::swift_definition(),
        routing_swift::swift_definition(),
        sealed_slug_swift::swift_definition(),
        sealed_swift::swift_definition(),
        status_swift::swift_definition(),
        tuple_point_swift::swift_definition(),
        untagged_swift::swift_definition(),
        user_data_swift::swift_definition(),
        window_request_swift::swift_definition(),
        with_flatten_swift::swift_definition(),
        with_nullable_swift::swift_definition(),
        berthed_swift::swift_definition(),
        carried_swift::swift_definition(),
        consigned_swift::swift_definition(),
        hauled_swift::swift_definition(),
        idle_swift::swift_definition(),
        lifted_swift::swift_definition(),
        sorted_swift::swift_definition(),
    ] {
        let declared = swift.matches("public struct ").count()
            + swift.matches("public enum ").count()
            - swift.matches("KeyedMap: Codable").count();
        let plain = swift.matches(": String, Codable, Sendable").count();
        assert_eq!(swift.matches(UNJUDGED).count(), declared, "got: {swift}");
        assert_eq!(
            swift.matches(JUDGED).count(),
            declared - plain,
            "got: {swift}"
        );
    }
}

/// A struct with no hand-written `encode(to:)` keeps the synthesized one, and one that had it
/// keeps its text.
#[test]
fn a_type_keeps_the_encode_it_had() {
    for synthesized in [
        window_request_swift::swift_definition(),
        parcel_swift::swift_definition(),
        manifest_swift::swift_definition(),
    ] {
        assert!(
            !synthesized.contains("func encode(to encoder:"),
            "got: {synthesized}"
        );
    }
    holds(
        &with_nullable_swift::swift_definition(),
        &[concat!(
            "public func encode(to encoder: Encoder) throws { ",
            "var container = encoder.container(keyedBy: CodingKeys.self); ",
            "try container.encode(note, forKey: .note) }; ",
        )],
    );
    holds(
        &with_flatten_swift::swift_definition(),
        &[concat!(
            "public func encode(to encoder: Encoder) throws { ",
            "var container = encoder.container(keyedBy: CodingKeys.self); ",
            "try extra.encode(to: encoder); try container.encode(id, forKey: .id) }; ",
        )],
    );
}

/// Every struct declares its memberwise initializer, the synthesized one being gone with a
/// written `init(from:)`.
#[test]
fn every_struct_has_a_public_memberwise_initializer() {
    holds(
        &window_request_swift::swift_definition(),
        &[concat!(
            "public init(conversationId: String, limit: UInt32?) ",
            "{ self.conversationId = conversationId; self.limit = limit;  }; ",
        )],
    );
    holds(
        &reserved_members_swift::swift_definition(),
        &[concat!(
            "public init(`class`: String, `in`: String, type: String) ",
            "{ self.`class` = `class`; self.`in` = `in`; self.type = type;  }; ",
        )],
    );
    holds(
        &inspection_swift::swift_definition(),
        &[concat!(
            "public init(errors: [String], validate_: String) ",
            "{ self.errors = errors; self.validate_ = validate_;  }; ",
        )],
    );
    holds(&idle_swift::swift_definition(), &["public init() {  }; "]);
    holds(
        &consigned_swift::swift_definition(),
        &[
            "public init(slot0: String, slot1: Parcel) { self.slot0 = slot0; self.slot1 = slot1;  }; ",
            "public init(value: ConsignedTuple) { self.value = value }; ",
        ],
    );
}

/// An absent key and a `null` are read as they were. A struct whose read Swift synthesized took
/// an absent key of any optional member as `nil`, and one whose read was written refused the
/// absent key of a nullable member.
#[test]
fn an_optional_and_a_nullable_member_keep_the_rule_they_had() {
    holds(
        &mooring_swift::swift_definition(),
        &[
            "self.berthNote = try container.decodeIfPresent(String.self, forKey: .berthNote); ",
            concat!(
                "if container.contains(.heldParcel), try !container.decodeNil(forKey: .heldParcel) ",
                "{ self.heldParcel = try Parcel(swiftSchemaUnjudged: ",
                "container.superDecoder(forKey: .heldParcel)) } else { self.heldParcel = nil }; ",
            ),
            "self.spareNote = try container.decodeIfPresent(String.self, forKey: .spareNote); ",
            concat!(
                "if container.contains(.spareParcel), try !container.decodeNil(forKey: .spareParcel) ",
                "{ self.spareParcel = try Parcel(swiftSchemaUnjudged: ",
                "container.superDecoder(forKey: .spareParcel)) } else { self.spareParcel = nil }",
            ),
        ],
    );
    holds(
        &anchorage_swift::swift_definition(),
        &[
            "self.anchorNote = try container.decode(String?.self, forKey: .anchorNote); ",
            "self.flattened = try FlattenedExtra(from: decoder); ",
            concat!(
                "if try !container.decodeNil(forKey: .heldParcel) ",
                "{ self.heldParcel = try Parcel(swiftSchemaUnjudged: ",
                "container.superDecoder(forKey: .heldParcel)) } else { self.heldParcel = nil }",
            ),
        ],
    );
}

/// A list of lists, a map of lists, a list of optionals and an optional list are read unjudged at
/// every level, each level under locals of its own.
#[test]
fn a_model_under_two_wrappers_is_read_unjudged_at_every_level() {
    holds(
        &stowage_swift::swift_definition(),
        &[
            concat!(
                "var listed = try container.nestedUnkeyedContainer(forKey: .grid); ",
                "var items: [[Parcel]] = []; while !listed.isAtEnd { ",
                "var listed1 = try listed.nestedUnkeyedContainer(); var items1: [Parcel] = []; ",
                "while !listed1.isAtEnd { items1.append(try Parcel(swiftSchemaUnjudged: ",
                "listed1.superDecoder())) }; items.append(items1) }; self.grid = items; ",
            ),
            concat!(
                "let keyed1 = try container.nestedContainer(keyedBy: SwiftSchemaAnyKey.self, ",
                "forKey: .listsByDock); var entries1: [String: [Parcel]] = [:]; ",
                "for entry1 in keyed1.allKeys { ",
                "var listed2 = try keyed1.nestedUnkeyedContainer(forKey: entry1); ",
                "var items2: [Parcel] = []; while !listed2.isAtEnd { items2.append(try Parcel(",
                "swiftSchemaUnjudged: listed2.superDecoder())) }; ",
                "entries1[entry1.stringValue] = items2 }; self.listsByDock = entries1; ",
            ),
            concat!(
                "var listed3 = try container.nestedUnkeyedContainer(forKey: .maybeEach); ",
                "var items3: [Parcel?] = []; while !listed3.isAtEnd { ",
                "if try !listed3.decodeNil() { items3.append(try Parcel(swiftSchemaUnjudged: ",
                "listed3.superDecoder())) } else { items3.append(nil) } }; ",
                "self.maybeEach = items3; ",
            ),
            concat!(
                "if container.contains(.spareList), try !container.decodeNil(forKey: .spareList) ",
                "{ var listed4 = try container.nestedUnkeyedContainer(forKey: .spareList); ",
                "var items4: [Parcel] = []; while !listed4.isAtEnd { items4.append(try Parcel(",
                "swiftSchemaUnjudged: listed4.superDecoder())) }; self.spareList = items4 } ",
                "else { self.spareList = nil }",
            ),
        ],
    );
}

/// A map under a key Swift reads as a number is read entry by entry, a key that is no number the
/// error it was. A map under any other key is read by its wrapper, which reads each value
/// unjudged, and a tuple by the struct written for it.
#[test]
fn a_model_in_a_map_under_a_number_and_in_a_tuple_is_read_unjudged() {
    holds(
        &stowage_swift::swift_definition(),
        &[
            concat!(
                "let keyed = try container.nestedContainer(keyedBy: SwiftSchemaAnyKey.self, ",
                "forKey: .byDeck); var entries: [Int: Parcel] = [:]; for entry in keyed.allKeys { ",
                "guard let number = Int(entry.stringValue) else { ",
                "throw DecodingError.typeMismatch(Int.self, DecodingError.Context(",
                "codingPath: keyed.codingPath + [entry], ",
                r#"debugDescription: "Expected Int key but found String key instead.")) }; "#,
                "entries[number] = try Parcel(swiftSchemaUnjudged: ",
                "keyed.superDecoder(forKey: entry)) }; self.byDeck = entries; ",
            ),
            concat!(
                "_ = try container.decodeNil(forKey: .byNumber); ",
                "self.byNumber = try StowageByNumberKeyedMap(swiftSchemaUnjudged: ",
                "container.superDecoder(forKey: .byNumber)).storage; ",
            ),
            concat!(
                "init(swiftSchemaUnjudged decoder: Decoder) throws { ",
                "let container = try decoder.container(keyedBy: DynamicCodingKey.self); ",
                "var result: [UInt32: Parcel] = [:]; for codingKey in container.allKeys { ",
                "let wireKey = codingKey.stringValue; guard let key = UInt32(wireKey) else { ",
                "throw DecodingError.dataCorruptedError(forKey: codingKey, in: container, ",
                r#"debugDescription: "bad key \(wireKey)") }; "#,
                "result[key] = try Parcel(swiftSchemaUnjudged: ",
                "container.superDecoder(forKey: codingKey)) }; storage = result }; ",
            ),
            concat!(
                "_ = try container.decodeNil(forKey: .deckPair); ",
                "self.deckPair = try StowageDeckPairTuple(swiftSchemaUnjudged: ",
                "container.superDecoder(forKey: .deckPair)); ",
            ),
            concat!(
                "{ var container = try decoder.unkeyedContainer(); ",
                "slot0 = try container.decode(String.self); ",
                "slot1 = try Parcel(swiftSchemaUnjudged: container.superDecoder());  }; ",
            ),
        ],
    );
    let unwalked = by_u32_swift::swift_definition();
    assert!(!unwalked.contains(UNJUDGED_IN_A_WRAPPER), "got: {unwalked}");
    holds(&unwalked, &["self.byU32 = wireByU32.storage"]);
}

/// A flattened member that has a rule is read unjudged through the decoder of its holder, and one
/// that has none as it was.
#[test]
fn a_flattened_member_is_read_through_the_decoder_of_its_holder() {
    holds(
        &berthed_swift::swift_definition(),
        &["self.parcel = try Parcel(swiftSchemaUnjudged: decoder); "],
    );
    holds(
        &with_flatten_swift::swift_definition(),
        &["self.extra = try FlattenedExtra(from: decoder); "],
    );
}

/// A value behind a type parameter is read through `Decodable`, so a model there judges itself.
#[test]
fn a_value_behind_a_type_parameter_is_read_as_it_was() {
    holds(
        &envelope_swift::swift_definition(),
        &[
            "self.note = try container.decode(String.self, forKey: .note); ",
            "self.value = try container.decode(T.self, forKey: .value) }; ",
        ],
    );
}

/// A brand reads the value it wraps as it did, then judges it. One with a bound answers for the
/// bound alone, which is what chooses a variant of an untagged enum.
#[test]
fn a_brand_has_both_reads_and_answers_for_its_bound_alone() {
    let swift = slug_swift::swift_definition();
    let unjudged = format!(
        "{UNJUDGED} {{ let container = try decoder.singleValueContainer(); \
         self.value = try container.decode(String.self) }}; {JUDGED}; "
    );
    assert!(swift.contains(&unjudged), "{unjudged} is not in: {swift}");
    holds(
        &swift,
        &[concat!(
            "public func swiftSchemaValidateBound() -> [String] { var errors: [String] = []; ",
            "if value.unicodeScalars.count < 2 { errors.append(",
            r#""too short: minimum length is 2, got \(value.unicodeScalars.count)") }; "#,
            "if value.unicodeScalars.count > 8 { errors.append(",
            r#""too long: maximum length is 8, got \(value.unicodeScalars.count)") }; "#,
            r#"if value.range(of: "^[a-z]+\\z", options: .regularExpression) == nil "#,
            r#"{ errors.append("does not match pattern '^[a-z]+$'") }; return errors }"#,
        )],
    );
    let unbounded = correlation_id_swift::swift_definition();
    assert!(unbounded.contains(&unjudged), "got: {unbounded}");
    assert!(
        !unbounded.contains("swiftSchemaValidateBound"),
        "got: {unbounded}"
    );
    holds(
        &marker_swift::swift_definition(),
        &[
            "self.value = try container.decode(IdType.self) }; ",
            concat!(
                "public func swiftSchemaValidateBound() -> [String] { var errors: [String] = []; ",
                "if let held = value as? String { if held.unicodeScalars.count < 2 {",
            ),
        ],
    );
}

/// A tuple struct reads the tuple it wraps unjudged, and the tuple each slot: a model through its
/// unjudged initializer, handed the next decoder of the array.
#[test]
fn a_tuple_struct_reads_its_slots_unjudged() {
    holds(
        &consigned_swift::swift_definition(),
        &[
            concat!(
                "public init(swiftSchemaUnjudged decoder: Decoder) throws { ",
                "var container = try decoder.unkeyedContainer(); ",
                "slot0 = try container.decode(String.self); ",
                "slot1 = try Parcel(swiftSchemaUnjudged: container.superDecoder());  }; ",
            ),
            concat!(
                "public init(swiftSchemaUnjudged decoder: Decoder) throws { ",
                "self.value = try ConsignedTuple(swiftSchemaUnjudged: decoder) }; ",
            ),
        ],
    );
    holds(
        &sealed_swift::swift_definition(),
        &[
            "slot0 = try container.decode(String.self);  }; ",
            "self.value = try SealedTuple(swiftSchemaUnjudged: decoder) }; ",
        ],
    );
}

#[test]
fn a_unit_struct_has_both_reads() {
    let swift = idle_swift::swift_definition();
    let written = format!("public init() {{  }}; {UNJUDGED} {{  }}; {JUDGED}; ");
    assert!(swift.contains(&written), "{written} is not in: {swift}");
}

/// An enum reads the payload of the variant it holds unjudged: under the variant's name with no
/// tag attribute, from its own decoder under a tag alone, and under the content key.
#[test]
fn an_enum_reads_the_payload_of_its_variant_unjudged() {
    holds(
        &carried_swift::swift_definition(),
        &[
            concat!(
                r#"case "Boxed": self = .boxed(try Parcel(swiftSchemaUnjudged: "#,
                "container.superDecoder(forKey: key))); ",
            ),
            concat!(
                r#"case "Listed": var listed = try container.nestedUnkeyedContainer(forKey: key); "#,
                "var items: [Parcel] = []; while !listed.isAtEnd { items.append(try Parcel(",
                "swiftSchemaUnjudged: listed.superDecoder())) }; self = .listed(items); ",
            ),
            concat!(
                r#"case "Loose": self = .loose(try CarriedLoosePayload(swiftSchemaUnjudged: "#,
                "container.superDecoder(forKey: key))); ",
            ),
            concat!(
                r#"case "Split": self = .split(try CarriedSplitTuple(swiftSchemaUnjudged: "#,
                "container.superDecoder(forKey: key))); ",
            ),
            r#"case "Unladen": self = .unladen; return; "#,
            JUDGED,
        ],
    );
    holds(
        &lifted_swift::swift_definition(),
        &[
            r#"case "Boxed": self = .boxed(try Parcel(swiftSchemaUnjudged: decoder)); "#,
            r#"case "Loose": self = .loose(try LiftedLoosePayload(swiftSchemaUnjudged: decoder)); "#,
            r#"case "Unladen": self = .unladen; "#,
            JUDGED,
        ],
    );
    holds(
        &hauled_swift::swift_definition(),
        &[
            concat!(
                r#"case "Boxed": _ = try container.decodeNil(forKey: .swiftSchemaContent); "#,
                "self = .boxed(try Parcel(swiftSchemaUnjudged: ",
                "container.superDecoder(forKey: .swiftSchemaContent))); ",
            ),
            concat!(
                r#"case "Loose": _ = try container.decodeNil(forKey: .swiftSchemaContent); "#,
                "self = .loose(try HauledLoosePayload(swiftSchemaUnjudged: ",
                "container.superDecoder(forKey: .swiftSchemaContent))); ",
            ),
            concat!(
                r#"case "Split": _ = try container.decodeNil(forKey: .swiftSchemaContent); "#,
                "self = .split(try HauledSplitTuple(swiftSchemaUnjudged: ",
                "container.superDecoder(forKey: .swiftSchemaContent))); ",
            ),
            JUDGED,
        ],
    );
    holds(
        &adjacent_tagged_swift::swift_definition(),
        &[concat!(
            r#"case "Flag": self = .flag(try container.decode(Bool.self, "#,
            "forKey: .swiftSchemaContent)); ",
        )],
    );
}

/// An untagged enum takes the first variant whose payload reads and whose own bounds hold: a
/// member's, and that of a brand it holds. A check and a model a variant holds choose nothing.
#[test]
fn an_untagged_enum_takes_the_first_variant_whose_bounds_hold() {
    let swift = sorted_swift::swift_definition();
    let attempts = [
        concat!(
            "if let value = try? String(from: decoder), ",
            "Sorted.swiftSchemaBoundBrief(value).isEmpty { self = .brief(value); return }; ",
        ),
        concat!(
            "if let value = try? Slug(swiftSchemaUnjudged: decoder), ",
            "Sorted.swiftSchemaBoundCoded(value).isEmpty { self = .coded(value); return }; ",
        ),
        concat!(
            "if let value = try? SortedCountedPayload(swiftSchemaUnjudged: decoder), ",
            "Sorted.swiftSchemaBoundCounted(value).isEmpty { self = .counted(value); return }; ",
        ),
        concat!(
            "if let value = try? Sorted.swiftSchemaReadListed(decoder) ",
            "{ self = .listed(value); return }; ",
        ),
        concat!(
            "if let value = try? Parcel(swiftSchemaUnjudged: decoder) ",
            "{ self = .whole(value); return }; ",
        ),
        "if let value = try? String(from: decoder) { self = .wordy(value); return }; ",
    ];
    let written = format!("{UNJUDGED} {{ {}", attempts.concat());
    assert!(swift.contains(&written), "{written} is not in: {swift}");
    holds(
        &swift,
        &[
            concat!(
                "private static func swiftSchemaBoundBrief(_ payload: String) -> [String] ",
                "{ var errors: [String] = []; if payload.unicodeScalars.count > 3 { errors.append(",
                r#""too long: maximum length is 3, got \(payload.unicodeScalars.count)") }; "#,
                "return errors }; ",
            ),
            concat!(
                "private static func swiftSchemaBoundCoded(_ payload: Slug) -> [String] ",
                "{ var errors: [String] = []; errors += payload.swiftSchemaValidateBound(); ",
                "return errors }; ",
            ),
            concat!(
                "private static func swiftSchemaBoundCounted(_ payload: SortedCountedPayload) ",
                "-> [String] { var errors: [String] = []; if payload.sortedCount < 1 ",
                r#"{ errors.append("'sortedCount': too small: minimum is 1, "#,
                r#"got \(payload.sortedCount)") }; return errors }; "#,
            ),
            concat!(
                "private static func swiftSchemaReadListed(_ decoder: Decoder) throws -> [Parcel] ",
                "{ var listed = try decoder.unkeyedContainer(); var items: [Parcel] = []; ",
                "while !listed.isAtEnd { items.append(try Parcel(swiftSchemaUnjudged: ",
                "listed.superDecoder())) }; return items }; ",
            ),
            r#"debugDescription: "no untagged member matched")) }; "#,
            JUDGED,
        ],
    );
    for unwritten in ["swiftSchemaBoundWhole", "swiftSchemaBoundWordy"] {
        assert!(!swift.contains(unwritten), "got: {swift}");
    }
    holds(
        &untagged_swift::swift_definition(),
        &[
            concat!(
                "if let value = try? UntaggedCountPayload(swiftSchemaUnjudged: decoder) ",
                "{ self = .count(value); return }; ",
            ),
            concat!(
                "if let value = try? UntaggedTextPayload(swiftSchemaUnjudged: decoder) ",
                "{ self = .text(value); return }; ",
            ),
        ],
    );
}

/// A plain enum that names no check keeps the read Swift synthesizes, and its unjudged
/// initializer is that read. One that names a check reads its raw value, then judges it.
#[test]
fn a_plain_enum_keeps_its_read_and_one_with_a_check_judges() {
    let plain = status_swift::swift_definition();
    holds(
        &plain,
        &[&format!("{UNJUDGED} {{ try self.init(from: decoder) }}; ")],
    );
    assert!(!plain.contains("init(from decoder:"), "got: {plain}");
    let checked = rating_swift::swift_definition();
    let written = format!(
        "{UNJUDGED} {}; {JUDGED}; ",
        concat!(
            "{ let container = try decoder.singleValueContainer(); ",
            "let raw = try container.decode(String.self); ",
            "guard let read = Rating(rawValue: raw) else { ",
            "throw DecodingError.dataCorruptedError(in: container, ",
            r#"debugDescription: "Cannot initialize Rating from invalid String value \(raw)") }; "#,
            "self = read }",
        ),
    );
    assert!(checked.contains(&written), "{written} is not in: {checked}");
}

#[test]
fn every_read_struct_is_built_and_written_by_serde() {
    let parcel = Parcel {
        fragile_item_codes: Vec::new(),
        item_code_list: vec!["x".to_owned()],
        label: "A".to_owned(),
    };
    let flattened = FlattenedExtra {
        reason: "r".to_owned(),
    };
    assert_eq!(
        serde_json::to_value(Mooring {
            berth_note: None,
            held_parcel: None,
            spare_note: None,
            spare_parcel: None,
        })
        .unwrap(),
        serde_json::json!({ "berthNote": null, "heldParcel": null })
    );
    assert_eq!(
        serde_json::to_value(Anchorage {
            anchor_note: None,
            flattened,
            held_parcel: None,
        })
        .unwrap(),
        serde_json::json!({ "anchorNote": null, "reason": "r", "heldParcel": null })
    );
    let stowage = Stowage {
        by_deck: HashMap::from([(-1, parcel.clone())]),
        by_number: HashMap::from([(7, parcel.clone())]),
        deck_pair: ("port".to_owned(), parcel.clone()),
        grid: vec![vec![parcel.clone()]],
        lists_by_dock: HashMap::from([("north".to_owned(), vec![parcel.clone()])]),
        maybe_each: vec![None, Some(parcel.clone())],
        spare_list: None,
    };
    let written = serde_json::to_value(&stowage).unwrap();
    assert_eq!(written["byDeck"]["-1"]["label"], "A");
    assert_eq!(written["byNumber"]["7"]["label"], "A");
    assert_eq!(written["deckPair"][1]["label"], "A");
    assert_eq!(written["maybeEach"][0], serde_json::Value::Null);
    assert_eq!(
        serde_json::to_value(Berthed {
            parcel: parcel.clone(),
            quay: "north".to_owned(),
        })
        .unwrap()["label"],
        "A"
    );
    assert_eq!(
        serde_json::to_value(Consigned("c".to_owned(), parcel)).unwrap()[1]["label"],
        "A"
    );
    assert_eq!(
        serde_json::to_value(Idle).unwrap(),
        serde_json::json!({}),
        "a unit struct is written as an empty object"
    );
}

#[test]
fn every_read_enum_is_built_and_written_by_serde() {
    let parcel = Parcel {
        fragile_item_codes: Vec::new(),
        item_code_list: vec!["x".to_owned()],
        label: "A".to_owned(),
    };
    assert_eq!(
        serde_json::to_value([
            Carried::Boxed(parcel.clone()),
            Carried::Listed(Vec::new()),
            Carried::Loose {
                loose_label: "l".to_owned(),
            },
            Carried::Split("s".to_owned(), parcel.clone()),
            Carried::Unladen,
        ])
        .unwrap()[2],
        serde_json::json!({ "Loose": { "looseLabel": "l" } })
    );
    assert_eq!(
        serde_json::to_value([
            Lifted::Boxed(parcel.clone()),
            Lifted::Loose {
                loose_label: "l".to_owned(),
            },
            Lifted::Unladen,
        ])
        .unwrap()[2],
        serde_json::json!({ "kind": "Unladen" })
    );
    assert_eq!(
        serde_json::to_value([
            Hauled::Boxed(parcel.clone()),
            Hauled::Loose {
                loose_label: "l".to_owned(),
            },
            Hauled::Split("s".to_owned(), parcel.clone()),
            Hauled::Unladen,
        ])
        .unwrap()[1],
        serde_json::json!({ "kind": "Loose", "held": { "looseLabel": "l" } })
    );
    assert_eq!(
        serde_json::to_value([
            Sorted::Brief("abc".to_owned()),
            Sorted::Coded(Slug("abcd".to_owned())),
            Sorted::Counted { sorted_count: 1 },
            Sorted::Listed(vec![parcel.clone()]),
            Sorted::Whole(parcel),
            Sorted::Wordy("ABCDEF".to_owned()),
        ])
        .unwrap()[5],
        "ABCDEF"
    );
    assert_eq!(Rating::High.validate(), Ok(()));
    assert_eq!(
        Rating::Low.validate(),
        Err(vec!["a low rating is not carried".to_owned()])
    );
}
