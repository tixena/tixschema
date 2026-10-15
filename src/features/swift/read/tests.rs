//! What the Swift read is written from: where a value is read from, and what an absent key is to
//! each read of one.

use syn::Item;

use super::{JUDGED, Locals, Presence, Source, both, held, takes_keys, unjudged};
use crate::features::model_schema_prop::{FunctionName, parse_model_schema_prop_attributes};
use crate::features::swift::preprocess::{PAYLOAD, Written};
use crate::features::swift::{SwiftTypeArgs, swift_schema_dispatch};
use crate::field_type::get_field_def;

/// The read of a type alone, where it has one that takes the keys the types around it took.
const ALONE: &str = concat!(
    "public init(swiftSchemaUnjudged decoder: Decoder) throws { ",
    "try self.init(swiftSchemaUnjudged: decoder, swiftSchemaWithout: []) }",
);

/// What opens the read of a type handed the keys the types around it took.
const TAKING: &str =
    "public init(swiftSchemaUnjudged decoder: Decoder, swiftSchemaWithout taken: [String]) throws";

/// The Swift source the emitter answers for `item`, written with no argument of `model_schema`.
fn emitted(item: &Item) -> String {
    emitted_with(item, None)
}

/// The Swift source the emitter answers for `item`, written with `minLength` where one is given.
fn emitted_with(item: &Item, min_length: Option<usize>) -> String {
    swift_schema_dispatch(
        item,
        &SwiftTypeArgs {
            check: &[],
            default_types: &[],
            max_length: None,
            member_meta: &|member| parse_model_schema_prop_attributes(&member.attrs),
            min_length,
            name_override: None,
            pattern: None,
            preprocess: &[],
        },
    )
    .to_string()
    .replace("\\\"", "\"")
}

fn function(name: &str) -> FunctionName {
    FunctionName {
        name: name.to_owned(),
        written_at: proc_macro2::Span::call_site(),
    }
}

fn keyed(presence: Presence) -> Source {
    Source::Keyed {
        container: "container".to_owned(),
        key: ".held".to_owned(),
        presence,
    }
}

/// A held type is handed a decoder of its own. Under a key that may be absent the key is read
/// first: `superDecoder` answers a decoder over `null` for an absent one.
#[test]
fn a_held_type_is_handed_a_decoder_of_its_own() {
    let member = |read: &str| format!("self.held = {read}");
    assert_eq!(
        held(&Source::Decoder, "ReadHeld", &member),
        "self.held = try ReadHeld(swiftSchemaUnjudged: decoder)"
    );
    assert_eq!(
        held(&Source::Listed("listed".to_owned()), "ReadHeld", &member),
        "self.held = try ReadHeld(swiftSchemaUnjudged: listed.superDecoder())"
    );
    assert_eq!(
        held(&keyed(Presence::Known), "ReadHeld", &member),
        "self.held = try ReadHeld(swiftSchemaUnjudged: container.superDecoder(forKey: .held))"
    );
    assert_eq!(
        held(&keyed(Presence::Refused), "ReadHeld", &member),
        concat!(
            "_ = try container.decodeNil(forKey: .held); ",
            "self.held = try ReadHeld(swiftSchemaUnjudged: container.superDecoder(forKey: .held))",
        )
    );
}

/// An optional under a key that may be absent is `nil` for no key and for `null`, under a key
/// that is required the error it was for no key, and in a list `nil` for `null`.
#[test]
fn an_optional_is_read_by_the_rule_of_where_it_is_held() {
    let def = get_field_def("held", &syn::parse_quote!(Option<ReadLater>), "");
    let member = |read: &str| format!("self.held = {read}");
    let read = |source: &Source| unjudged(&def, "Hint", source, &member, &mut Locals::default());
    assert_eq!(
        read(&keyed(Presence::Nil)),
        concat!(
            "if container.contains(.held), try !container.decodeNil(forKey: .held) ",
            "{ self.held = try ReadLater(swiftSchemaUnjudged: container.superDecoder(forKey: .held)) } ",
            "else { self.held = nil }",
        )
    );
    assert_eq!(
        read(&keyed(Presence::Refused)),
        concat!(
            "if try !container.decodeNil(forKey: .held) ",
            "{ self.held = try ReadLater(swiftSchemaUnjudged: container.superDecoder(forKey: .held)) } ",
            "else { self.held = nil }",
        )
    );
    assert_eq!(
        read(&Source::Listed("listed".to_owned())),
        concat!(
            "if try !listed.decodeNil() ",
            "{ self.held = try ReadLater(swiftSchemaUnjudged: listed.superDecoder()) } ",
            "else { self.held = nil }",
        )
    );
    assert_eq!(
        read(&Source::Decoder),
        concat!(
            "if try !decoder.singleValueContainer().decodeNil() ",
            "{ self.held = try ReadLater(swiftSchemaUnjudged: decoder) } else { self.held = nil }",
        )
    );
}

/// A list and a map are read from the container their source hands them, each level under locals
/// of its own.
#[test]
fn a_list_and_a_map_are_read_from_the_container_of_their_source() {
    let listed = get_field_def("held", &syn::parse_quote!(Vec<ReadLater>), "");
    let mapped = get_field_def(
        "held",
        &syn::parse_quote!(HashMap<String, Vec<ReadLater>>),
        "",
    );
    let handed = |read: &str| format!("return {read}");
    let mut locals = Locals::default();
    assert_eq!(
        unjudged(&listed, "Hint", &Source::Decoder, &handed, &mut locals),
        concat!(
            "var listed = try decoder.unkeyedContainer(); var items: [ReadLater] = []; ",
            "while !listed.isAtEnd { items.append(try ReadLater(swiftSchemaUnjudged: ",
            "listed.superDecoder())) }; return items",
        )
    );
    assert_eq!(
        unjudged(
            &mapped,
            "Hint",
            &Source::Listed("outer".to_owned()),
            &handed,
            &mut locals
        ),
        concat!(
            "let keyed = try outer.nestedContainer(keyedBy: SwiftSchemaAnyKey.self); ",
            "var entries: [String: [ReadLater]] = [:]; for entry in keyed.allKeys { ",
            "var listed1 = try keyed.nestedUnkeyedContainer(forKey: entry); ",
            "var items1: [ReadLater] = []; while !listed1.isAtEnd { items1.append(try ReadLater(",
            "swiftSchemaUnjudged: listed1.superDecoder())) }; ",
            "entries[entry.stringValue] = items1 }; return entries",
        )
    );
}

/// A type that names no function, and whose statements name no key a type around it took, has
/// the two reads it had.
#[test]
fn a_type_that_names_no_function_has_the_two_reads_it_had() {
    let reads = both("self.held = try decoder.read()", &Written::default());
    assert_eq!(
        reads,
        format!(
            "public init(swiftSchemaUnjudged decoder: Decoder) throws \
             {{ self.held = try decoder.read() }}; {JUDGED}; "
        )
    );
    assert!(!takes_keys(&reads));
}

/// A type whose statements name the keys the types around it took has its own read twice: handed
/// those keys, and alone.
#[test]
fn a_type_whose_statements_name_the_keys_taken_is_handed_them() {
    let body = "self.held = try Held(swiftSchemaUnjudged: decoder, swiftSchemaWithout: taken)";
    let reads = both(body, &Written::default());
    assert_eq!(reads, format!("{TAKING} {{ {body} }}; {ALONE}; {JUDGED}; "));
    assert!(takes_keys(&reads));
    let handed = "let heldPayload = try SwiftSchemaPayload(from: decoder, without: taken).value";
    assert!(takes_keys(&both(handed, &Written::default())));
}

/// A type that names functions on itself reads its statements from the payload they answer, in
/// an initializer of its own, and hands them the payload without the keys taken. Statements that
/// name those keys read none there: the payload holds none of them.
#[test]
fn a_type_that_names_functions_reads_its_statements_from_what_they_answer() {
    let own = [function("unwrapped"), function("withoutFragile")];
    let written = |body: &str| {
        both(
            body,
            &Written {
                named: "Held",
                own: &own,
                ..Written::default()
            },
        )
    };
    let around = |corrected: &str| {
        format!(
            "{PAYLOAD}; private struct SwiftSchemaRead: Decodable {{ let value: Held; \
             init(from decoder: Decoder) throws {{ value = try Held(swiftSchemaCorrected: decoder) }} }}; \
             private init(swiftSchemaCorrected decoder: Decoder) throws {{ {corrected} }}; \
             {TAKING} {{ self = try SwiftSchemaPayload.read(SwiftSchemaRead.self, \
             from: withoutFragile(unwrapped(try SwiftSchemaPayload(from: decoder, without: taken).value)), \
             at: decoder.codingPath).value }}; {ALONE}; {JUDGED}; "
        )
    };
    assert_eq!(written("self.held = 1"), around("self.held = 1"));
    let beside = "self.held = try Other(swiftSchemaUnjudged: decoder, swiftSchemaWithout: taken)";
    assert_eq!(
        written(beside),
        around(&format!("let taken: [String] = []; {beside}"))
    );
    assert!(takes_keys(&written("self.held = 1")));
}

/// The payload type is written where the statements or the readers beside them name it, the key
/// type where a type written for a member names it and no reader does, and a type written for a
/// member names its owner where `Self` is written.
#[test]
fn the_types_a_read_names_are_written_ahead_of_it() {
    let read = "self.held = try SwiftSchemaPayload.read(Twin.self, from: held, at: path).value";
    let twin = "private struct Twin: Decodable { let value: [String: Self]; \
                init(from decoder: Decoder) throws { \
                let keyed = try decoder.container(keyedBy: SwiftSchemaAnyKey.self) } }";
    let reads = both(
        read,
        &Written {
            named: "Holder",
            twins: vec![twin.to_owned()],
            ..Written::default()
        },
    );
    let opened = format!(
        "private struct SwiftSchemaAnyKey: CodingKey {{ var stringValue: String; \
         var intValue: Int? {{ nil }}; init?(stringValue: String) {{ self.stringValue = stringValue }}; \
         init?(intValue: Int) {{ nil }} }}; {PAYLOAD}; \
         private struct Twin: Decodable {{ let value: [String: Holder]; "
    );
    assert!(reads.starts_with(&opened), "got: {reads}");
    let beside_readers = both(
        "self = try Holder.swiftSchemaReadHeld(decoder)",
        &Written {
            beside: "let keyed = try decoder.container(keyedBy: SwiftSchemaAnyKey.self); \
                     let heldPayload = try SwiftSchemaPayload(from: decoder).value",
            named: "Holder",
            ..Written::default()
        },
    );
    assert!(
        beside_readers.starts_with(&format!("{PAYLOAD}; public init(swiftSchemaUnjudged")),
        "got: {beside_readers}"
    );
}

/// The README shows the two reads and the memberwise initializer of the documents' parcel as
/// the emitter writes them.
#[test]
fn the_readme_shows_the_reads_of_the_documents_parcel() {
    let parcel: Item = syn::parse_quote! {
        #[serde(rename_all = "camelCase")]
        pub struct Parcel {
            #[model_schema_prop(minLength = 1)]
            pub label: String,
            #[model_schema_prop(check = ["withoutDuplicates"])]
            pub item_code_list: Vec<String>,
            pub fragile_item_codes: Vec<String>,
        }
    };
    let written = emitted(&parcel);
    let readme = include_str!("../../../../README.md");
    for shown in [
        "public init(label: String, itemCodeList: [String], fragileItemCodes: [String])",
        "public init(swiftSchemaUnjudged decoder: Decoder) throws {",
        "let container = try decoder.container(keyedBy: CodingKeys.self)",
        "self.label = try container.decode(String.self, forKey: .label)",
        "self.itemCodeList = try container.decode([String].self, forKey: .itemCodeList)",
        "self.fragileItemCodes = try container.decode([String].self, forKey: .fragileItemCodes)",
        "public init(from decoder: Decoder) throws {",
        "try self.init(swiftSchemaUnjudged: decoder)",
        "let violations = validate()",
        "if !violations.isEmpty {",
        concat!(
            "throw DecodingError.dataCorrupted(DecodingError.Context(",
            "codingPath: decoder.codingPath, ",
            r#"debugDescription: violations.joined(separator: "; ")))"#,
        ),
    ] {
        assert!(written.contains(shown), "{shown} is not in: {written}");
        assert!(
            readme.contains(shown),
            "the README no longer shows: {shown}"
        );
    }
}

/// A brand with a bound chooses a variant of an untagged enum declared below it, and so does an
/// alias of it. One declared below the enum is a name the emitter has not written, and one with
/// no bound has nothing to choose by.
#[cfg(feature = "serde")]
#[test]
fn a_bounded_brand_declared_above_an_untagged_enum_chooses_its_variant() {
    let bounded: Item = syn::parse_quote! {
        #[serde(transparent)]
        pub struct ChoicePin(pub String);
    };
    let unbounded: Item = syn::parse_quote! {
        #[serde(transparent)]
        pub struct ChoiceNote(pub String);
    };
    let alias: Item = syn::parse_quote! {
        pub type ChoiceAlias = ChoicePin;
    };
    let choice: Item = syn::parse_quote! {
        #[serde(untagged)]
        pub enum Choice {
            Aliased(ChoiceAlias),
            Later(ChoiceLaterPin),
            Noted(ChoiceNote),
            Pinned(Vec<ChoicePin>),
        }
    };
    assert!(emitted_with(&bounded, Some(2)).contains(concat!(
        "public func swiftSchemaValidateBound() -> [String] { var errors: [String] = []; ",
        "if value.unicodeScalars.count < 2 {",
    )));
    assert!(!emitted(&unbounded).contains("swiftSchemaValidateBound"));
    emitted(&alias);
    let written = emitted(&choice);
    for chosen in [
        concat!(
            "private static func swiftSchemaBoundAliased(_ payload: ChoiceAliasType) -> [String] ",
            "{ var errors: [String] = []; errors += payload.swiftSchemaValidateBound(); ",
            "return errors }; ",
        ),
        concat!(
            "private static func swiftSchemaBoundPinned(_ payload: [ChoicePin]) -> [String] ",
            "{ var errors: [String] = []; for (index, held) in payload.enumerated() ",
            "{ errors += held.swiftSchemaValidateBound().map ",
            r#"{ Choice.swiftSchemaUnder("\\(index)", $0) } }; return errors }; "#,
        ),
        "private static func swiftSchemaUnder(_ place: String, _ violation: String) -> String",
    ] {
        assert!(written.contains(chosen), "{chosen} is not in: {written}");
    }
    for unwritten in ["swiftSchemaBoundLater", "swiftSchemaBoundNoted"] {
        assert!(!written.contains(unwritten), "got: {written}");
    }
}

/// A brand over an optional value Swift converts reads it in a closure that throws, called with
/// `try`. A build with a web surface refuses such a brand, so its text is held here.
#[cfg(feature = "serde")]
#[test]
fn a_brand_over_an_optional_converted_value_calls_its_closure_with_try() {
    let hoard: Item = syn::parse_quote! {
        #[serde(transparent)]
        pub struct OptionalHoard(pub Option<HashMap<u32, String>>);
    };
    let written = emitted(&hoard);
    let call = concat!(
        "self.value = try { let wireValue = ",
        "try container.decode(OptionalHoardKeyedMap?.self); return ",
    );
    assert!(written.contains(call), "{call} is not in: {written}");
}

/// [`a_brand_over_an_optional_converted_value_calls_its_closure_with_try`], over a date.
#[cfg(all(feature = "serde", feature = "chrono"))]
#[test]
fn a_brand_over_an_optional_date_calls_its_closure_with_try() {
    let landfall: Item = syn::parse_quote! {
        #[serde(transparent)]
        pub struct OptionalLandfall(pub Option<DateTime<Utc>>);
    };
    let written = emitted(&landfall);
    let call = concat!(
        "self.value = try { let wireValue = try container.decode(String?.self); ",
        "return try wireValue.map { try Self.swiftSchemaParseIso8601Date($0, ",
        "codingPath: decoder.codingPath) } }() }; ",
    );
    assert!(written.contains(call), "{call} is not in: {written}");
}
