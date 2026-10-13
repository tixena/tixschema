//! What the Swift backend writes for `preprocess`: the payload type, the call of a member's
//! functions ahead of the member's read, and the call of a type's on the whole payload. What a
//! read answers is run under a Swift toolchain in
//! `service_schema_emitted_client_tests/run_swift_preprocess.rs`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::model_schema;

/// What opens the payload type, written into each type that names a function.
const PAYLOAD: &str = "private struct SwiftSchemaPayload: Decodable { let value: Any; ";

/// What opens the read of a type alone.
const UNJUDGED: &str = "public init(swiftSchemaUnjudged decoder: Decoder) throws";

/// What opens the read of a type beside the keys the types around it took.
const BESIDE: &str =
    "public init(swiftSchemaUnjudged decoder: Decoder, swiftSchemaWithout taken: [String]) throws";

/// The read of a type alone, where it has one that takes the keys the types around it took.
const ALONE: &str = concat!(
    "public init(swiftSchemaUnjudged decoder: Decoder) throws { ",
    "try self.init(swiftSchemaUnjudged: decoder, swiftSchemaWithout: []) }; ",
);

/// The parcel of the documents, under a name of this file: a function on the type and one on a
/// member that has a bound.
#[model_schema(preprocess = ["withoutFragile"])]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Carton {
    pub fragile_item_codes: Vec<String>,
    pub item_code_list: Vec<String>,
    #[model_schema_prop(minLength = 1, preprocess = ["trimmed"])]
    pub label: String,
}

/// A carton held under a key, in a list, in a map and in an optional, by a type that names no
/// function.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Loads {
    pub carton_list: Vec<Carton>,
    pub cartons_by_dock: HashMap<String, Carton>,
    pub first_carton: Carton,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare_carton: Option<Carton>,
}

/// A function on a member of each kind a struct reads.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pallet {
    #[model_schema_prop(preprocess = ["unwrapped"])]
    pub by_number: HashMap<u32, String>,
    #[model_schema_prop(preprocess = ["unwrapped"])]
    pub carton_list: Vec<Carton>,
    #[model_schema_prop(preprocess = ["unwrapped"])]
    pub first_carton: Carton,
    #[model_schema_prop(nullable, preprocess = ["orUnnamed"])]
    pub quay_note: Option<String>,
    #[model_schema_prop(preprocess = ["unwrapped"])]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare_carton: Option<Carton>,
    #[model_schema_prop(preprocess = ["orUnnamed"])]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare_note: Option<String>,
    #[model_schema_prop(preprocess = ["unwrapped", "trimmed"])]
    pub title: String,
}

/// A type that names no function and holds no type that names one.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bay {
    pub keys: Vec<String>,
}

/// A holder that flattens a carton.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Racked {
    #[serde(flatten)]
    pub carton: Carton,
    pub dock: String,
    pub rack_row: u32,
}

/// A holder that flattens that holder.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stacked {
    #[serde(flatten)]
    pub racked: Racked,
    pub tier: String,
}

/// A flattened member that names a function of its own, and one that names none.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bayed {
    #[serde(flatten)]
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    pub bay: Bay,
    pub dock: String,
}

/// A brand over a carton.
#[model_schema(no_display)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Wrap(pub Carton);

/// Another name for a carton.
#[model_schema()]
pub type LaterCarton = Carton;

/// A holder that flattens a carton under its other name.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Aliased {
    #[serde(flatten)]
    pub carton: LaterCarton,
    pub dock: String,
}

/// A holder declared above the type it flattens, which names a function.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ahead {
    #[serde(flatten)]
    pub behind: Behind,
    pub dock: String,
}

/// A type that names a function, declared below its holder.
#[model_schema(preprocess = ["withItsKeys"])]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Behind {
    pub keys: Vec<String>,
}

/// A unit struct.
#[model_schema(preprocess = ["unwrapped"])]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shut;

/// A tuple struct that names a function on itself and on a slot.
#[model_schema(preprocess = ["unwrapped"])]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Heap(
    #[model_schema_prop(preprocess = ["trimmed"])] pub String,
    #[model_schema_prop(preprocess = ["unwrapped"])] pub Carton,
    pub u32,
);

/// A brand, with two functions.
#[model_schema(preprocess = ["unwrapped", "trimmed"])]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Tagline(pub String);

/// A brand with a parameter.
#[model_schema(default_types(IdType = String), preprocess = ["unwrapped"])]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Ticket<IdType>(pub IdType);

/// A struct with a parameter.
#[model_schema(default_types(CodeType = String), preprocess = ["unwrapped"])]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bin<CodeType> {
    #[model_schema_prop(preprocess = ["unwrapped"])]
    pub codes: Vec<CodeType>,
    pub label: String,
}

/// A plain enum.
#[model_schema(preprocess = ["unwrapped"])]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tier {
    Express,
    Standard,
}

/// An enum with no tag attribute: a function on itself, on a slot and on a named member.
#[model_schema(preprocess = ["unwrapped"])]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Hop {
    Held,
    Rail(#[model_schema_prop(preprocess = ["trimmed"])] String),
    Road {
        #[model_schema_prop(preprocess = ["trimmed"])]
        plate: String,
    },
}

/// An enum under a tag alone: a function on itself, and a variant that holds a carton.
#[model_schema(preprocess = ["unwrapped"])]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Gated {
    Held(Carton),
    Lost,
    Named(#[model_schema_prop(preprocess = ["withItsKeys"])] Bay),
}

/// An enum under a tag alone that names no function, and whose variant holds a carton.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Lane {
    Held(Carton),
    Lost,
}

/// An enum under a tag and a content key.
#[model_schema(preprocess = ["unwrapped"])]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "detail")]
pub enum Leg {
    Left(#[model_schema_prop(preprocess = ["trimmed"])] String),
    Signed { by: String },
}

/// An untagged enum: a function on itself, a slot that names one, and a variant that holds a
/// carton.
#[model_schema(preprocess = ["unwrapped"])]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Pick {
    Named(#[model_schema_prop(preprocess = ["trimmed"])] String),
    Whole(Carton),
}

/// A date that names a function.
#[cfg(feature = "chrono")]
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timed {
    #[model_schema_prop(preprocess = ["unwrapped"])]
    pub at: chrono::DateTime<chrono::Utc>,
}

/// `unnamed` for a payload that holds nothing, and any other payload as it is.
fn or_unnamed(payload: Value) -> Value {
    if payload.is_null() {
        Value::String("unnamed".to_owned())
    } else {
        payload
    }
}

/// The payload of a text without the spaces at its ends, and any other payload as it is.
fn trimmed(payload: Value) -> Value {
    if let Value::String(text) = payload {
        Value::String(text.trim().to_owned())
    } else {
        payload
    }
}

/// What an envelope holds under `payload`, and a payload that is no envelope as it is.
fn unwrapped(payload: Value) -> Value {
    if let Value::Object(mut entries) = payload {
        entries
            .remove("payload")
            .unwrap_or_else(|| Value::Object(entries))
    } else {
        payload
    }
}

/// An object that lists, under `keys`, the keys it was handed with, in order.
fn with_its_keys(mut payload: Value) -> Value {
    if let Some(object) = payload.as_object_mut() {
        let mut keys: Vec<String> = object.keys().cloned().collect();
        keys.sort_unstable();
        object.insert(
            "keys".to_owned(),
            Value::Array(keys.into_iter().map(Value::String).collect()),
        );
    }
    payload
}

/// An object with an empty `fragileItemCodes` where it holds none.
fn without_fragile(mut payload: Value) -> Value {
    if let Some(object) = payload.as_object_mut() {
        object
            .entry("fragileItemCodes")
            .or_insert_with(|| Value::Array(Vec::new()));
    }
    payload
}

/// Asserts each of `written` is in `swift`.
fn holds(swift: &str, written: &[&str]) {
    for expected in written {
        assert!(swift.contains(expected), "{expected} is not in: {swift}");
    }
}

/// How often the payload type is written into `swift`.
fn payload_types(swift: &str) -> usize {
    swift.matches(PAYLOAD).count()
}

/// What the three reads of a type that names `called` on itself are, around the statements
/// `corrected` of its read of a corrected payload.
fn corrected_reads(named: &str, called: &str, closing: &str, corrected: &str) -> String {
    format!(
        "private struct SwiftSchemaRead: Decodable {{ let value: {named}; \
         init(from decoder: Decoder) throws {{ value = try {named}(swiftSchemaCorrected: decoder) }} }}; \
         private init(swiftSchemaCorrected decoder: Decoder) throws {{ {corrected} }}; \
         {BESIDE} {{ self = try SwiftSchemaPayload.read(SwiftSchemaRead.self, \
         from: {called}try SwiftSchemaPayload(from: decoder, without: taken).value{closing}, \
         at: decoder.codingPath).value }}; {ALONE}public init(from decoder: Decoder) throws {{ "
    )
}

/// A member's function is called on the payload of its key, and the member is read from what it
/// answers, at the coding path of the key. The type's own is called on the whole payload, and
/// the members are read from what it answers.
#[test]
fn a_struct_calls_the_function_of_a_member_and_its_own() {
    let swift = carton_swift::swift_definition();
    let written = corrected_reads(
        "Carton",
        "withoutFragile(",
        ")",
        concat!(
            "let container = try decoder.container(keyedBy: CodingKeys.self); ",
            "self.fragileItemCodes = try container.decode([String].self, forKey: .fragileItemCodes); ",
            "self.itemCodeList = try container.decode([String].self, forKey: .itemCodeList); ",
            "let labelPayload = try container.decode(SwiftSchemaPayload.self, forKey: .label).value; ",
            "self.label = try SwiftSchemaPayload.read(String.self, from: trimmed(labelPayload), ",
            "at: container.codingPath + [CodingKeys.label])",
        ),
    );
    assert!(swift.contains(&written), "{written} is not in: {swift}");
    assert_eq!(payload_types(&swift), 1, "got: {swift}");
    assert!(!swift.contains("func encode(to encoder:"), "got: {swift}");
}

/// The payload type reads any JSON value, reads a type back from one, and writes the coding path
/// of the value into each error of that read.
#[test]
fn the_payload_type_reads_any_value_and_reads_a_type_back_from_one() {
    holds(
        &carton_swift::swift_definition(),
        &[
            concat!(
                "private struct SwiftSchemaPayload: Decodable { let value: Any; ",
                "init(from decoder: Decoder) throws { ",
                "if let keyed = try? decoder.container(keyedBy: Key.self) { ",
                "var members: [String: Any] = [:]; for key in keyed.allKeys { ",
                "members[key.stringValue] = try keyed.decode(SwiftSchemaPayload.self, forKey: key).value }; ",
                "value = members } else if var listed = try? decoder.unkeyedContainer() { ",
                "var items: [Any] = []; while !listed.isAtEnd { ",
                "items.append(try listed.decode(SwiftSchemaPayload.self).value) }; value = items } ",
                "else { let single = try decoder.singleValueContainer(); ",
                "if single.decodeNil() { value = NSNull() } ",
                "else if let flag = try? single.decode(Bool.self) { value = flag } ",
                "else if let whole = try? single.decode(Int64.self) { value = whole } ",
                "else if let number = try? single.decode(Double.self) { value = number } ",
                "else { value = try single.decode(String.self) } } }; ",
            ),
            concat!(
                "init(from decoder: Decoder, without taken: [String]) throws { ",
                "let whole = try SwiftSchemaPayload(from: decoder).value; ",
                "guard var members = whole as? [String: Any] else { value = whole; return }; ",
                "for key in taken { members.removeValue(forKey: key) }; value = members }; ",
            ),
            concat!(
                "static func read<Read: Decodable>(_ type: Read.Type, from payload: Any, ",
                "at path: [CodingKey]) throws -> Read { ",
                "guard JSONSerialization.isValidJSONObject([payload]), ",
                "let written = try? JSONSerialization.data(withJSONObject: payload, ",
                "options: [.fragmentsAllowed]) else { ",
                "throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: path, ",
                r#"debugDescription: "a preprocess answered a value that is not JSON")) }; "#,
            ),
            concat!(
                "do { return try JSONDecoder().decode(type, from: written) } ",
                "catch DecodingError.typeMismatch(let asked, let context) { ",
                "throw DecodingError.typeMismatch(asked, placed(context)) } ",
                "catch DecodingError.valueNotFound(let asked, let context) { ",
                "throw DecodingError.valueNotFound(asked, placed(context)) } ",
                "catch DecodingError.keyNotFound(let key, let context) { ",
                "throw DecodingError.keyNotFound(key, placed(context)) } ",
                "catch DecodingError.dataCorrupted(let context) { ",
                "throw DecodingError.dataCorrupted(placed(context)) } }; ",
            ),
        ],
    );
}

/// The payload type is written once into each type that names a function, on itself or on a
/// member, and into no other: a type that holds one reads it as it reads any model.
#[test]
fn the_payload_type_is_written_into_each_type_that_names_a_function_and_no_other() {
    for (named, swift) in [
        ("Pallet", pallet_swift::swift_definition()),
        ("Bayed", bayed_swift::swift_definition()),
        ("Shut", shut_swift::swift_definition()),
        ("Tagline", tagline_swift::swift_definition()),
        ("Ticket", ticket_swift::swift_definition()),
        ("Bin", bin_swift::swift_definition()),
        ("Tier", tier_swift::swift_definition()),
        ("Leg", leg_swift::swift_definition()),
        ("Pick", pick_swift::swift_definition()),
    ] {
        assert_eq!(payload_types(&swift), 1, "{named}: {swift}");
    }
    // The tuple a tuple struct wraps and the struct of a variant's members each name their own.
    assert_eq!(payload_types(&heap_swift::swift_definition()), 2);
    assert_eq!(payload_types(&hop_swift::swift_definition()), 2);
    assert_eq!(payload_types(&gated_swift::swift_definition()), 1);
    let holder = loads_swift::swift_definition();
    assert_eq!(payload_types(&holder), 0, "got: {holder}");
    holds(
        &holder,
        &[
            concat!(
                "_ = try container.decodeNil(forKey: .firstCarton); self.firstCarton = ",
                "try Carton(swiftSchemaUnjudged: container.superDecoder(forKey: .firstCarton)); ",
            ),
            "items.append(try Carton(swiftSchemaUnjudged: listed.superDecoder()))",
            "entries[entry.stringValue] = try Carton(swiftSchemaUnjudged: keyed.superDecoder(forKey: entry))",
        ],
    );
    for unnamed in [
        bay_swift::swift_definition(),
        racked_swift::swift_definition(),
        stacked_swift::swift_definition(),
        lane_swift::swift_definition(),
    ] {
        assert_eq!(payload_types(&unnamed), 0, "got: {unnamed}");
    }
}

/// A type that names no function, and reads no type that names one from its own decoder, is
/// written as it was: one read of its own, opened as every read was.
#[test]
fn a_type_that_names_no_function_is_written_as_it_was() {
    let swift = bay_swift::swift_definition();
    let written = format!(
        "public init(keys: [String]) {{ self.keys = keys;  }}; {UNJUDGED} {{ \
         let container = try decoder.container(keyedBy: CodingKeys.self); \
         self.keys = try container.decode([String].self, forKey: .keys) }}; \
         public init(from decoder: Decoder) throws {{ "
    );
    assert!(swift.contains(&written), "{written} is not in: {swift}");
    assert!(!swift.contains("swiftSchemaWithout"), "got: {swift}");
    assert!(!swift.contains("swiftSchemaCorrected"), "got: {swift}");
    let holder = loads_swift::swift_definition();
    assert!(!holder.contains("swiftSchemaWithout"), "got: {holder}");
}

/// A member's function is called where the key is held: a member whose absent key is `nil` is
/// asked for its key first, and any other is read as a required one. The value is read from what
/// the function answers in the way the member was read: a model through its own read alone, in a
/// type written for the member, and a map Swift reads through a wrapper through that wrapper.
#[test]
fn a_member_is_read_from_what_its_functions_answer_in_the_way_it_was_read() {
    let swift = pallet_swift::swift_definition();
    holds(
        &swift,
        &[
            concat!(
                "let byNumberPayload = try container.decode(SwiftSchemaPayload.self, forKey: .byNumber).value; ",
                "let wireByNumber = try SwiftSchemaPayload.read(PalletByNumberKeyedMap.self, ",
                "from: unwrapped(byNumberPayload), at: container.codingPath + [CodingKeys.byNumber]); ",
                "self.byNumber = wireByNumber.storage; ",
            ),
            concat!(
                "let cartonListPayload = try container.decode(SwiftSchemaPayload.self, forKey: .cartonList).value; ",
                "self.cartonList = try SwiftSchemaPayload.read(SwiftSchemaReadCartonList.self, ",
                "from: unwrapped(cartonListPayload), at: container.codingPath + [CodingKeys.cartonList]).value; ",
            ),
            concat!(
                "let firstCartonPayload = try container.decode(SwiftSchemaPayload.self, forKey: .firstCarton).value; ",
                "self.firstCarton = try SwiftSchemaPayload.read(SwiftSchemaReadFirstCarton.self, ",
                "from: unwrapped(firstCartonPayload), at: container.codingPath + [CodingKeys.firstCarton]).value; ",
            ),
            concat!(
                "let quayNotePayload = try container.decode(SwiftSchemaPayload.self, forKey: .quayNote).value; ",
                "self.quayNote = try SwiftSchemaPayload.read(String?.self, from: orUnnamed(quayNotePayload), ",
                "at: container.codingPath + [CodingKeys.quayNote]); ",
            ),
            concat!(
                "if container.contains(.spareCarton) { ",
                "let spareCartonPayload = try container.decode(SwiftSchemaPayload.self, forKey: .spareCarton).value; ",
                "self.spareCarton = try SwiftSchemaPayload.read(SwiftSchemaReadSpareCarton.self, ",
                "from: unwrapped(spareCartonPayload), at: container.codingPath + [CodingKeys.spareCarton]).value } ",
                "else { self.spareCarton = nil }; ",
            ),
            concat!(
                "if container.contains(.spareNote) { ",
                "let spareNotePayload = try container.decode(SwiftSchemaPayload.self, forKey: .spareNote).value; ",
                "self.spareNote = try SwiftSchemaPayload.read(String?.self, from: orUnnamed(spareNotePayload), ",
                "at: container.codingPath + [CodingKeys.spareNote]) } else { self.spareNote = nil }; ",
            ),
            concat!(
                "let titlePayload = try container.decode(SwiftSchemaPayload.self, forKey: .title).value; ",
                "self.title = try SwiftSchemaPayload.read(String.self, from: trimmed(unwrapped(titlePayload)), ",
                "at: container.codingPath + [CodingKeys.title]) }; ",
            ),
            concat!(
                "private struct SwiftSchemaReadCartonList: Decodable { let value: [Carton]; ",
                "init(from decoder: Decoder) throws { var listed = try decoder.unkeyedContainer(); ",
                "var items: [Carton] = []; while !listed.isAtEnd { ",
                "items.append(try Carton(swiftSchemaUnjudged: listed.superDecoder())) }; ",
                "self.value = items } }; ",
            ),
            concat!(
                "private struct SwiftSchemaReadFirstCarton: Decodable { let value: Carton; ",
                "init(from decoder: Decoder) throws { ",
                "self.value = try Carton(swiftSchemaUnjudged: decoder) } }; ",
            ),
            concat!(
                "private struct SwiftSchemaReadSpareCarton: Decodable { let value: Carton?; ",
                "init(from decoder: Decoder) throws { ",
                "if try !decoder.singleValueContainer().decodeNil() { ",
                "self.value = try Carton(swiftSchemaUnjudged: decoder) } else { self.value = nil } } }; ",
            ),
        ],
    );
    // A type that names a function on a member alone has one read of its own.
    assert!(!swift.contains("swiftSchemaWithout"), "got: {swift}");
    assert!(!swift.contains("swiftSchemaCorrected"), "got: {swift}");
}

/// A date is read from what its function answers through the text it is written as.
#[cfg(feature = "chrono")]
#[test]
fn a_date_is_read_from_what_its_function_answers_as_the_text_it_is_written_as() {
    holds(
        &timed_swift::swift_definition(),
        &[concat!(
            "let atPayload = try container.decode(SwiftSchemaPayload.self, forKey: .at).value; ",
            "let wireAt = try SwiftSchemaPayload.read(String.self, from: unwrapped(atPayload), ",
            "at: container.codingPath + [CodingKeys.at]); ",
            "self.at = try Self.swiftSchemaParseIso8601Date(wireAt, codingPath: decoder.codingPath) }; ",
        )],
    );
}

/// A type that names a function reads the keys the types around it did not take, and so does a
/// type that reads one from its own decoder: a holder that flattens it hands it its own keys
/// beside those it was handed, and an enum under a tag alone its tag.
#[test]
fn a_type_read_from_the_decoder_of_its_holder_is_handed_the_keys_taken() {
    let racked = racked_swift::swift_definition();
    holds(
        &racked,
        &[
            &format!(
                "{BESIDE} {{ let container = try decoder.container(keyedBy: CodingKeys.self); \
                 self.carton = try Carton(swiftSchemaUnjudged: decoder, \
                 swiftSchemaWithout: taken + [\"dock\", \"rackRow\"]); "
            ),
            ALONE,
        ],
    );
    holds(
        &stacked_swift::swift_definition(),
        &[
            concat!(
                "self.racked = try Racked(swiftSchemaUnjudged: decoder, ",
                r#"swiftSchemaWithout: taken + ["tier"]); "#,
            ),
            ALONE,
        ],
    );
    holds(
        &lane_swift::swift_definition(),
        &[
            concat!(
                r#"case "Held": self = .held(try Carton(swiftSchemaUnjudged: decoder, "#,
                r#"swiftSchemaWithout: taken + ["kind"])); "#,
            ),
            ALONE,
        ],
    );
    // A member's own function is handed the same keys, and the member is read from its answer.
    holds(
        &bayed_swift::swift_definition(),
        &[
            concat!(
                "let bayPayload = try SwiftSchemaPayload(from: decoder, ",
                r#"without: taken + ["dock"]).value; "#,
                "self.bay = try SwiftSchemaPayload.read(Bay.self, from: withItsKeys(bayPayload), ",
                "at: decoder.codingPath); ",
            ),
            ALONE,
        ],
    );
}

/// A brand over such a type hands it the keys it was handed, and judges it as the type's own read
/// judges it. An alias of such a type is one too. A type declared below the type that reads it is
/// not known to take the keys, and is read as any model is.
#[test]
fn a_brand_and_an_alias_hand_the_keys_and_a_type_declared_below_is_handed_none() {
    let wrap = wrap_swift::swift_definition();
    let read = format!(
        "{BESIDE} {{ self.value = try Carton(swiftSchemaUnjudged: decoder, \
         swiftSchemaWithout: taken); let held = value.validate(); if !held.isEmpty {{ \
         throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, \
         debugDescription: held.joined(separator: \"; \"))) }} }}; {ALONE}"
    );
    assert!(wrap.contains(&read), "{read} is not in: {wrap}");
    assert_eq!(payload_types(&wrap), 0, "got: {wrap}");
    holds(
        &aliased_swift::swift_definition(),
        &[
            concat!(
                "self.carton = try LaterCartonType(swiftSchemaUnjudged: decoder, ",
                r#"swiftSchemaWithout: taken + ["dock"]); "#,
            ),
            ALONE,
        ],
    );
    let ahead = ahead_swift::swift_definition();
    holds(
        &ahead,
        &["self.behind = try Behind(swiftSchemaUnjudged: decoder); "],
    );
    assert!(!ahead.contains("swiftSchemaWithout"), "got: {ahead}");
}

/// A unit struct, a tuple struct, a brand and a type with a parameter each call their function
/// on the whole payload, and read the value from what it answers.
#[test]
fn every_struct_shape_calls_its_own_function_on_the_whole_payload() {
    let heap = heap_swift::swift_definition();
    for (swift, read) in [
        (
            shut_swift::swift_definition(),
            corrected_reads("Shut", "unwrapped(", ")", ""),
        ),
        (
            tagline_swift::swift_definition(),
            corrected_reads(
                "Tagline",
                "trimmed(unwrapped(",
                "))",
                "let container = try decoder.singleValueContainer(); \
                 self.value = try container.decode(String.self)",
            ),
        ),
        (
            ticket_swift::swift_definition(),
            corrected_reads(
                "Ticket",
                "unwrapped(",
                ")",
                "let container = try decoder.singleValueContainer(); \
                 self.value = try container.decode(IdType.self)",
            ),
        ),
        (
            bin_swift::swift_definition(),
            corrected_reads(
                "Bin",
                "unwrapped(",
                ")",
                "let container = try decoder.container(keyedBy: CodingKeys.self); \
                 let codesPayload = try container.decode(SwiftSchemaPayload.self, forKey: .codes).value; \
                 self.codes = try SwiftSchemaPayload.read([CodeType].self, from: unwrapped(codesPayload), \
                 at: container.codingPath + [CodingKeys.codes]); \
                 self.label = try container.decode(String.self, forKey: .label)",
            ),
        ),
        (
            heap.clone(),
            corrected_reads(
                "Heap",
                "unwrapped(",
                ")",
                "self.value = try HeapTuple(swiftSchemaUnjudged: decoder)",
            ),
        ),
    ] {
        assert!(swift.contains(&read), "{read} is not in: {swift}");
    }
    // A slot's function is called in the tuple the struct wraps, on the payload of the slot.
    holds(
        &heap,
        &[
            concat!(
                "var container = try decoder.unkeyedContainer(); ",
                "let slot0Decoder = try container.superDecoder(); ",
                "let slot0Payload = try SwiftSchemaPayload(from: slot0Decoder).value; ",
                "slot0 = try SwiftSchemaPayload.read(String.self, from: trimmed(slot0Payload), ",
                "at: slot0Decoder.codingPath); ",
                "let slot1Decoder = try container.superDecoder(); ",
                "let slot1Payload = try SwiftSchemaPayload(from: slot1Decoder).value; ",
                "slot1 = try SwiftSchemaPayload.read(SwiftSchemaReadSlot1.self, ",
                "from: unwrapped(slot1Payload), at: slot1Decoder.codingPath).value; ",
                "slot2 = try container.decode(UInt32.self);  }; ",
            ),
            concat!(
                "private struct SwiftSchemaReadSlot1: Decodable { let value: Carton; ",
                "init(from decoder: Decoder) throws { ",
                "self.value = try Carton(swiftSchemaUnjudged: decoder) } }; ",
            ),
        ],
    );
}

/// An enum calls its own function once, on the whole payload, ahead of the choice of a variant,
/// under every tagging and with none.
#[test]
fn every_enum_calls_its_own_function_ahead_of_the_choice_of_a_variant() {
    let tier = tier_swift::swift_definition();
    let read = corrected_reads(
        "Tier",
        "unwrapped(",
        ")",
        concat!(
            "let container = try decoder.singleValueContainer(); ",
            "let raw = try container.decode(String.self); ",
            "guard let read = Tier(rawValue: raw) else { ",
            "throw DecodingError.dataCorruptedError(in: container, ",
            r#"debugDescription: "Cannot initialize Tier from invalid String value \(raw)") }; "#,
            "self = read",
        ),
    );
    assert!(tier.contains(&read), "{read} is not in: {tier}");

    for (named, swift, opening) in [
        (
            "Hop",
            hop_swift::swift_definition(),
            "if let single = try? decoder.singleValueContainer(), let tag = try? single.decode(String.self) { ",
        ),
        (
            "Leg",
            leg_swift::swift_definition(),
            "let container = try decoder.container(keyedBy: SwiftSchemaAdjacentCodingKeys.self); ",
        ),
    ] {
        let opened =
            format!("private init(swiftSchemaCorrected decoder: Decoder) throws {{ {opening}");
        let called = format!(
            "{BESIDE} {{ self = try SwiftSchemaPayload.read(SwiftSchemaRead.self, \
             from: unwrapped(try SwiftSchemaPayload(from: decoder, without: taken).value), \
             at: decoder.codingPath).value }}; {ALONE}"
        );
        holds(&swift, &[&opened, &called]);
        assert!(
            swift.contains(&format!(
                "private struct SwiftSchemaRead: Decodable {{ let value: {named}; "
            )),
            "got: {swift}"
        );
    }
    // The payload of a corrected enum holds no key a type around it took.
    holds(
        &gated_swift::swift_definition(),
        &[
            concat!(
                "private init(swiftSchemaCorrected decoder: Decoder) throws { ",
                "let taken: [String] = []; ",
                "let container = try decoder.container(keyedBy: SwiftSchemaTagCodingKeys.self); ",
            ),
            concat!(
                r#"case "Held": self = .held(try Carton(swiftSchemaUnjudged: decoder, "#,
                r#"swiftSchemaWithout: taken + ["kind"])); "#,
            ),
            concat!(
                r#"case "Named": let namedPayload = try SwiftSchemaPayload(from: decoder, "#,
                r#"without: taken + ["kind"]).value; "#,
                "self = .named(try SwiftSchemaPayload.read(Bay.self, from: withItsKeys(namedPayload), ",
                "at: decoder.codingPath)); ",
            ),
        ],
    );
    holds(
        &pick_swift::swift_definition(),
        &[
            concat!(
                "private static func swiftSchemaReadNamed(_ decoder: Decoder, _ taken: [String]) ",
                "throws -> String { ",
                "let namedPayload = try SwiftSchemaPayload(from: decoder, without: taken).value; ",
                "return try SwiftSchemaPayload.read(String.self, from: trimmed(namedPayload), ",
                "at: decoder.codingPath) }; ",
            ),
            concat!(
                "private init(swiftSchemaCorrected decoder: Decoder) throws { ",
                "let taken: [String] = []; ",
                "if let value = try? Pick.swiftSchemaReadNamed(decoder, taken) { self = .named(value); return }; ",
                "if let value = try? Carton(swiftSchemaUnjudged: decoder, swiftSchemaWithout: taken) ",
                "{ self = .whole(value); return }; ",
            ),
        ],
    );
}

/// A slot and a named member of a variant call their function on the payload serde writes for
/// them: under the variant's name, and under the content key.
#[test]
fn a_member_of_a_variant_calls_its_function_on_its_own_payload() {
    holds(
        &hop_swift::swift_definition(),
        &[
            concat!(
                r#"case "Rail": let railPayload = try container.decode(SwiftSchemaPayload.self, "#,
                "forKey: key).value; self = .rail(try SwiftSchemaPayload.read(String.self, ",
                "from: trimmed(railPayload), at: container.codingPath + [key])); ",
            ),
            concat!(
                "let platePayload = try container.decode(SwiftSchemaPayload.self, forKey: .plate).value; ",
                "self.plate = try SwiftSchemaPayload.read(String.self, from: trimmed(platePayload), ",
                "at: container.codingPath + [CodingKeys.plate]) }; ",
            ),
        ],
    );
    holds(
        &leg_swift::swift_definition(),
        &[concat!(
            r#"case "Left": let leftPayload = try container.decode(SwiftSchemaPayload.self, "#,
            "forKey: .swiftSchemaContent).value; ",
            "self = .left(try SwiftSchemaPayload.read(String.self, from: trimmed(leftPayload), ",
            "at: container.codingPath + [SwiftSchemaAdjacentCodingKeys.swiftSchemaContent])); ",
        )],
    );
}

/// A carton as every read of this file answers it.
fn carton() -> Carton {
    Carton {
        fragile_item_codes: Vec::new(),
        item_code_list: vec!["x".to_owned()],
        label: "A".to_owned(),
    }
}

/// serde reads a carton, and each type that holds one, through the functions the Swift read
/// calls.
#[test]
fn every_holder_of_this_file_is_read_by_serde_through_its_functions() {
    let written = json!({ "label": "  A  ", "itemCodeList": ["x"] });
    let enveloped = json!({ "payload": written });
    assert_eq!(
        serde_json::from_value::<Carton>(written.clone()).unwrap(),
        carton()
    );
    assert_eq!(
        serde_json::from_value::<Loads>(json!({
            "cartonList": [written],
            "cartonsByDock": { "north": written },
            "firstCarton": written,
        }))
        .unwrap(),
        Loads {
            carton_list: vec![carton()],
            cartons_by_dock: HashMap::from([("north".to_owned(), carton())]),
            first_carton: carton(),
            spare_carton: None,
        }
    );
    assert_eq!(
        serde_json::from_value::<Pallet>(json!({
            "byNumber": { "payload": { "7": "seven" } },
            "cartonList": { "payload": [written] },
            "firstCarton": enveloped,
            "quayNote": null,
            "spareCarton": enveloped,
            "title": { "payload": "  T  " },
        }))
        .unwrap(),
        Pallet {
            by_number: HashMap::from([(7, "seven".to_owned())]),
            carton_list: vec![carton()],
            first_carton: carton(),
            quay_note: Some("unnamed".to_owned()),
            spare_carton: Some(carton()),
            spare_note: None,
            title: "T".to_owned(),
        }
    );
    assert_eq!(
        serde_json::from_value::<Stacked>(json!({
            "dock": "north", "itemCodeList": ["x"], "label": " A ", "rackRow": 2_u32, "tier": "t",
        }))
        .unwrap(),
        Stacked {
            racked: Racked {
                carton: carton(),
                dock: "north".to_owned(),
                rack_row: 2,
            },
            tier: "t".to_owned(),
        }
    );
    assert_eq!(
        serde_json::from_value::<Bayed>(json!({ "dock": "north", "tag": "t" })).unwrap(),
        Bayed {
            bay: Bay {
                keys: vec!["tag".to_owned()],
            },
            dock: "north".to_owned(),
        }
    );
    assert_eq!(
        serde_json::from_value::<Aliased>(json!({
            "dock": "north", "itemCodeList": ["x"], "label": " A ",
        }))
        .unwrap(),
        Aliased {
            carton: carton(),
            dock: "north".to_owned(),
        }
    );
    assert_eq!(
        serde_json::from_value::<Ahead>(json!({ "dock": "north", "tag": "t" })).unwrap(),
        Ahead {
            behind: Behind {
                keys: vec!["tag".to_owned()],
            },
            dock: "north".to_owned(),
        }
    );
    assert_eq!(
        serde_json::from_value::<Wrap>(written).unwrap(),
        Wrap(carton())
    );
}

/// serde reads each struct shape of this file through the functions the Swift read calls.
#[test]
fn every_struct_shape_of_this_file_is_read_by_serde_through_its_functions() {
    let enveloped = json!({ "payload": { "label": "  A  ", "itemCodeList": ["x"] } });
    assert_eq!(
        serde_json::from_value::<Shut>(json!({ "payload": {} })).unwrap(),
        Shut
    );
    assert_eq!(
        serde_json::from_value::<Heap>(json!({ "payload": ["  A  ", enveloped, 2_u32] })).unwrap(),
        Heap("A".to_owned(), carton(), 2)
    );
    assert_eq!(
        serde_json::from_value::<Tagline>(json!({ "payload": "  A  " })).unwrap(),
        Tagline("A".to_owned())
    );
    assert_eq!(
        serde_json::from_value::<Ticket<u32>>(json!({ "payload": 7_u32 })).unwrap(),
        Ticket(7)
    );
    assert_eq!(
        serde_json::from_value::<Bin<u32>>(json!({
            "payload": { "codes": { "payload": [7_u32] }, "label": "b" },
        }))
        .unwrap(),
        Bin {
            codes: vec![7],
            label: "b".to_owned(),
        }
    );
    assert_eq!(
        serde_json::from_value::<Tier>(json!({ "payload": "Express" })).unwrap(),
        Tier::Express
    );
}

/// serde reads each enum of this file through the functions the Swift read calls.
#[test]
fn every_enum_of_this_file_is_read_by_serde_through_its_functions() {
    assert_eq!(
        [
            serde_json::from_value::<Hop>(json!({ "payload": "Held" })).unwrap(),
            serde_json::from_value::<Hop>(json!({ "payload": { "Rail": "  A  " } })).unwrap(),
            serde_json::from_value::<Hop>(json!({ "Road": { "plate": "  A  " } })).unwrap(),
        ],
        [
            Hop::Held,
            Hop::Rail("A".to_owned()),
            Hop::Road {
                plate: "A".to_owned(),
            },
        ]
    );
    assert_eq!(
        [
            serde_json::from_value::<Gated>(json!({
                "payload": { "kind": "Held", "label": " A ", "itemCodeList": ["x"] },
            }))
            .unwrap(),
            serde_json::from_value::<Gated>(json!({ "kind": "Lost" })).unwrap(),
            serde_json::from_value::<Gated>(json!({ "kind": "Named", "tag": "t" })).unwrap(),
        ],
        [
            Gated::Held(carton()),
            Gated::Lost,
            Gated::Named(Bay {
                keys: vec!["tag".to_owned()],
            }),
        ]
    );
    assert_eq!(
        [
            serde_json::from_value::<Lane>(json!({
                "kind": "Held", "label": " A ", "itemCodeList": ["x"],
            }))
            .unwrap(),
            serde_json::from_value::<Lane>(json!({ "kind": "Lost" })).unwrap(),
        ],
        [Lane::Held(carton()), Lane::Lost]
    );
    assert_eq!(
        [
            serde_json::from_value::<Leg>(json!({
                "payload": { "kind": "Left", "detail": "  door  " },
            }))
            .unwrap(),
            serde_json::from_value::<Leg>(json!({ "kind": "Signed", "detail": { "by": "A" } }))
                .unwrap(),
        ],
        [
            Leg::Left("door".to_owned()),
            Leg::Signed { by: "A".to_owned() },
        ]
    );
    assert_eq!(
        [
            serde_json::from_value::<Pick>(json!({ "payload": "  A  " })).unwrap(),
            serde_json::from_value::<Pick>(json!({ "label": " A ", "itemCodeList": ["x"] }))
                .unwrap(),
        ],
        [Pick::Named("A".to_owned()), Pick::Whole(carton())]
    );
}

/// serde reads a date through the function the Swift read calls.
#[cfg(feature = "chrono")]
#[test]
fn a_date_is_read_by_serde_through_its_function() {
    let read: Timed =
        serde_json::from_value(json!({ "at": { "payload": "2023-11-14T22:13:20Z" } })).unwrap();
    assert_eq!(read.at.timestamp(), 1_700_000_000);
}
