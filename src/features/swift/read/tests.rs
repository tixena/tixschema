//! What the Swift read is written from: where a value is read from, and what an absent key is to
//! each read of one.

use syn::Item;

use super::{Locals, Presence, Source, held, unjudged};
use crate::features::model_schema_prop::parse_model_schema_prop_attributes;
use crate::features::swift::{SwiftTypeArgs, swift_schema_dispatch};
use crate::field_type::get_field_def;

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
        },
    )
    .to_string()
    .replace("\\\"", "\"")
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
