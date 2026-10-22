//! What the Swift read of a `preprocess` is written from: the call of the functions, the payload
//! each source hands them, and the types that read the keys the types around them took.

use core::iter;

use proc_macro2::Span;

use super::{
    Entry, Member, Plain, beside, called, entries_of, entry_payload, keys_standing_for, member,
    naming_its_owner, note, note_entries, note_keys, note_tag_keys, taken_with, takes,
};
use crate::features::model_schema_prop::{FunctionName, ModelSchemaPropMeta};
use crate::features::swift::read::{Presence, Source};
use crate::field_type::get_field_def;

fn named(names: &[&str]) -> Vec<FunctionName> {
    names
        .iter()
        .map(|&name| FunctionName {
            name: name.to_owned(),
            written_at: Span::call_site(),
        })
        .collect()
}

fn keyed(presence: Presence) -> Source {
    Source::Keyed {
        container: "container".to_owned(),
        key: ".label".to_owned(),
        presence,
    }
}

/// What a text that names `functions` is read with from `source`, and the types written for it.
fn text_read(functions: &[FunctionName], source: &Source) -> (String, Vec<String>) {
    let def = get_field_def("label", &syn::parse_quote!(String), "");
    let mut twins = Vec::new();
    let read = member(
        &Member {
            def: &def,
            functions,
            hint: "Hint",
            keys: "CodingKeys",
            plain: Plain::Declared,
            real_type: "String",
            stem: "label",
            walked: false,
        },
        source,
        &|held| format!("self.label = {held}"),
        &mut twins,
    );
    (read, twins)
}

/// Each function is called on the answer of the one before, in the order written, and a word
/// Swift reserves is called between backticks.
#[test]
fn the_functions_are_called_in_the_order_written() {
    assert_eq!(called(&named(&["trimmed"]), "held"), "trimmed(held)");
    assert_eq!(
        called(&named(&["unwrapped", "trimmed", "default"]), "held"),
        "`default`(trimmed(unwrapped(held)))"
    );
    assert_eq!(called(&[], "held"), "held");
}

/// A key is read as the payload type, which refuses an absent key as a plain `decode` does, and
/// the value is read from what the functions answer, at the coding path of the key.
#[test]
fn a_member_under_a_key_is_read_from_what_its_functions_answer() {
    let (read, twins) = text_read(&named(&["trimmed"]), &keyed(Presence::Refused));
    assert_eq!(
        read,
        concat!(
            "let labelPayload = try container.decode(SwiftSchemaPayload.self, forKey: .label).value; ",
            "self.label = try SwiftSchemaPayload.read(String.self, from: trimmed(labelPayload), ",
            "at: container.codingPath + [CodingKeys.label])",
        )
    );
    assert!(twins.is_empty(), "got: {twins:?}");
    assert_eq!(
        text_read(&named(&["trimmed"]), &keyed(Presence::Known)).0,
        read
    );
}

/// A member whose absent key is `nil` is asked for its key first: no function is called for a key
/// the payload does not hold.
#[test]
fn a_member_whose_absent_key_is_nil_is_asked_for_its_key_first() {
    let (read, _) = text_read(&named(&["trimmed"]), &keyed(Presence::Nil));
    assert_eq!(
        read,
        concat!(
            "if container.contains(.label) { ",
            "let labelPayload = try container.decode(SwiftSchemaPayload.self, forKey: .label).value; ",
            "self.label = try SwiftSchemaPayload.read(String.self, from: trimmed(labelPayload), ",
            "at: container.codingPath + [CodingKeys.label]) } else { self.label = nil }",
        )
    );
}

/// A key that is a value of its own is written into the coding path as it is.
#[test]
fn a_key_held_in_a_local_is_its_own_step_of_the_coding_path() {
    let source = Source::Keyed {
        container: "container".to_owned(),
        key: "key".to_owned(),
        presence: Presence::Known,
    };
    let (read, _) = text_read(&named(&["trimmed"]), &source);
    assert!(
        read.ends_with("from: trimmed(labelPayload), at: container.codingPath + [key])"),
        "got: {read}"
    );
}

/// A slot is read through a decoder of its own, which has the coding path of the slot. A value
/// read from the decoder the type was handed sits at that decoder's path, and one read beside the
/// keys of other types is handed the payload without them.
#[test]
fn a_slot_and_the_decoder_itself_each_hand_their_payload_and_their_path() {
    assert_eq!(
        text_read(&named(&["trimmed"]), &Source::Listed("listed".to_owned())).0,
        concat!(
            "let labelDecoder = try listed.superDecoder(); ",
            "let labelPayload = try SwiftSchemaPayload(from: labelDecoder).value; ",
            "self.label = try SwiftSchemaPayload.read(String.self, from: trimmed(labelPayload), ",
            "at: labelDecoder.codingPath)",
        )
    );
    assert_eq!(
        text_read(&named(&["trimmed"]), &Source::Decoder).0,
        concat!(
            "let labelPayload = try SwiftSchemaPayload(from: decoder).value; ",
            "self.label = try SwiftSchemaPayload.read(String.self, from: trimmed(labelPayload), ",
            "at: decoder.codingPath)",
        )
    );
    assert_eq!(
        text_read(
            &named(&["trimmed"]),
            &Source::Beside("taken + [\"dock\"]".to_owned())
        )
        .0,
        concat!(
            "let labelPayload = try SwiftSchemaPayload(from: decoder, ",
            "without: taken + [\"dock\"]).value; ",
            "self.label = try SwiftSchemaPayload.read(String.self, from: trimmed(labelPayload), ",
            "at: decoder.codingPath)",
        )
    );
}

/// A member `validate()` walks is read through a type written for it, whose read is the one the
/// member had: each model through its own read alone, from the decoder of the corrected payload.
#[test]
fn a_walked_member_is_read_through_a_type_written_for_it() {
    let def = get_field_def("held", &syn::parse_quote!(Option<Vec<HeldLater>>), "");
    let mut twins = Vec::new();
    let read = member(
        &Member {
            def: &def,
            functions: &named(&["unwrapped"]),
            hint: "Hint",
            keys: "CodingKeys",
            plain: Plain::Declared,
            real_type: "[HeldLater]?",
            stem: "heldList",
            walked: true,
        },
        &keyed(Presence::Refused),
        &|held| format!("self.heldList = {held}"),
        &mut twins,
    );
    assert_eq!(
        read,
        concat!(
            "let heldListPayload = try container.decode(SwiftSchemaPayload.self, forKey: .label).value; ",
            "self.heldList = try SwiftSchemaPayload.read(SwiftSchemaReadHeldList.self, ",
            "from: unwrapped(heldListPayload), at: container.codingPath + [CodingKeys.label]).value",
        )
    );
    assert_eq!(
        twins,
        [concat!(
            "private struct SwiftSchemaReadHeldList: Decodable { let value: [HeldLater]?; ",
            "init(from decoder: Decoder) throws { ",
            "if try !decoder.singleValueContainer().decodeNil() { ",
            "var listed = try decoder.unkeyedContainer(); var items: [HeldLater] = []; ",
            "while !listed.isAtEnd { ",
            "items.append(try HeldLater(swiftSchemaUnjudged: listed.superDecoder())) }; ",
            "self.value = items } else { self.value = nil } } }",
        )]
    );
}

/// A member read through another type than its own is read as that type from what its functions
/// answer, and turned into its own from the local.
#[test]
fn a_member_read_as_another_type_is_turned_into_its_own_from_the_local() {
    let def = get_field_def("held", &syn::parse_quote!(String), "");
    let read = member(
        &Member {
            def: &def,
            functions: &named(&["unwrapped"]),
            hint: "Hint",
            keys: "CodingKeys",
            plain: Plain::Wire {
                convert: &|local| format!("{local}.storage"),
                wire: "HintKeyedMap",
            },
            real_type: "[UInt32: String]",
            stem: "held",
            walked: false,
        },
        &keyed(Presence::Refused),
        &|held| format!("self.held = {held}"),
        &mut Vec::new(),
    );
    assert_eq!(
        read,
        concat!(
            "let heldPayload = try container.decode(SwiftSchemaPayload.self, forKey: .label).value; ",
            "let wireHeld = try SwiftSchemaPayload.read(HintKeyedMap.self, from: unwrapped(heldPayload), ",
            "at: container.codingPath + [CodingKeys.label]); self.held = wireHeld.storage",
        )
    );
}

/// In a type declared inside another, `Self` is the inner one: the type written for a member
/// names its owner where the member's type is written `Self`.
#[test]
fn a_type_written_for_a_member_names_its_owner_where_self_is_written() {
    assert_eq!(
        naming_its_owner(
            "let value: [Self]; items.append(try Self(swiftSchemaUnjudged: decoder))",
            "Branch"
        ),
        "let value: [Branch]; items.append(try Branch(swiftSchemaUnjudged: decoder))"
    );
    assert_eq!(
        naming_its_owner("let value: SelfService; self.value = MySelf()", "Branch"),
        "let value: SelfService; self.value = MySelf()"
    );
    assert_eq!(naming_its_owner("Self", "Branch"), "Branch");
}

/// The keys a holder took are its own after those it was handed, each as a Swift string literal.
#[test]
fn the_keys_taken_are_the_ones_handed_and_the_holders_own() {
    assert_eq!(taken_with(iter::empty()), "taken");
    assert_eq!(
        taken_with(["dock", "rack\"Row"].into_iter()),
        "taken + [\"dock\", \"rack\\\"Row\"]"
    );
}

/// A type is one a holder hands the keys taken once it is noted as taking them, under an
/// `Option` and under no list.
#[test]
fn a_noted_type_is_read_beside_the_keys_taken_under_no_list() {
    note("PreprocessNotedHere", true);
    assert!(takes("PreprocessNotedHere"));
    assert!(!takes("PreprocessNotedNowhere"));
    let read = |written: syn::Type| beside(&get_field_def("held", &written, ""));
    assert!(read(syn::parse_quote!(PreprocessNotedHere)));
    assert!(read(syn::parse_quote!(Option<PreprocessNotedHere>)));
    assert!(!read(syn::parse_quote!(Vec<PreprocessNotedHere>)));
    assert!(!read(syn::parse_quote!(PreprocessNotedNowhere)));
    assert!(!read(syn::parse_quote!(String)));
    note("PreprocessNotedHere", false);
    assert!(!takes("PreprocessNotedHere"));
}

/// An enum is one a holder reads from its one entry once its names are noted, under an `Option`
/// and under no list, where the member names no function. An enum noted with no name is not.
#[test]
fn a_noted_enum_is_read_from_its_one_entry_under_no_list() {
    let entry = |name: &str, variant: &str, unit| Entry {
        name: name.to_owned(),
        unit,
        variant: variant.to_owned(),
    };
    note_entries(
        "PreprocessEntriesHere",
        vec![entry("Bay", "Bay", false), entry("Quay", "Bay", false)],
    );
    let names = |written: syn::Type| {
        entries_of(&get_field_def("held", &written, "")).map(|(named, entries)| {
            let names: Vec<String> = entries.into_iter().map(|noted| noted.name).collect();
            format!("{named}: {}", names.join(", "))
        })
    };
    let noted = Some("PreprocessEntriesHere: Bay, Quay".to_owned());
    assert_eq!(names(syn::parse_quote!(PreprocessEntriesHere)), noted);
    assert_eq!(
        names(syn::parse_quote!(Option<PreprocessEntriesHere>)),
        noted
    );
    assert_eq!(names(syn::parse_quote!(Vec<PreprocessEntriesHere>)), None);
    assert_eq!(names(syn::parse_quote!(PreprocessEntriesNowhere)), None);
    note_entries("PreprocessEntriesHere", Vec::new());
    assert_eq!(names(syn::parse_quote!(PreprocessEntriesHere)), None);
}

/// A flattened `Option` is tested by the keys of a struct noted with keys, and by the tag and the
/// content key of an enum noted with them, under no function. An enum noted with no key, and a
/// type noted nowhere, are tested by none.
#[test]
fn an_option_is_tested_by_a_structs_keys_and_an_enums_tag() {
    note_keys("PreprocessKeysHere", vec!["label".to_owned()]);
    note_tag_keys(
        "PreprocessTagHere",
        vec!["kind".to_owned(), "body".to_owned()],
    );
    let keys = |written: syn::Type| keys_standing_for(&get_field_def("held", &written, ""));
    assert_eq!(
        keys(syn::parse_quote!(Option<PreprocessKeysHere>)),
        ["label"]
    );
    assert_eq!(
        keys(syn::parse_quote!(Option<PreprocessTagHere>)),
        ["kind", "body"]
    );
    assert!(
        keys(syn::parse_quote!(Option<PreprocessTagNowhere>)).is_empty(),
        "a type noted nowhere is tested by no key"
    );
    let mut corrected = get_field_def("held", &syn::parse_quote!(Option<PreprocessTagHere>), "");
    corrected.model_schema_prop_meta = Some(ModelSchemaPropMeta {
        preprocess: named(&["asGiven"]),
        ..ModelSchemaPropMeta::default()
    });
    assert!(
        keys_standing_for(&corrected).is_empty(),
        "a member that names a function is tested by no key"
    );
    note_tag_keys("PreprocessTagHere", Vec::new());
    assert!(
        keys(syn::parse_quote!(Option<PreprocessTagHere>)).is_empty(),
        "an enum noted with no key is tested by none"
    );
}

/// The enum's own read is handed a unit variant's name where its entry holds `null`, an alias's
/// entry under its variant's name, and any other entry as it is.
#[test]
fn an_entry_is_handed_as_the_enums_own_read_takes_it() {
    let entry = |name: &str, variant: &str, unit| Entry {
        name: name.to_owned(),
        unit,
        variant: variant.to_owned(),
    };
    assert_eq!(
        entry_payload("hatch", &[entry("Bay", "Bay", false)]),
        "let hatchPayload: Any = [hatchKey: hatchHeld]"
    );
    assert_eq!(
        entry_payload(
            "hatch",
            &[
                entry("Bay", "Bay", false),
                entry("Calm", "Calm", true),
                entry("Still", "Calm", true),
                entry("Quay", "Bay", false),
            ]
        ),
        "let hatchPayload: Any; switch hatchKey { \
         case \"Calm\" where hatchHeld is NSNull: hatchPayload = \"Calm\"; \
         case \"Still\" where hatchHeld is NSNull: hatchPayload = \"Calm\"; \
         case \"Quay\": hatchPayload = [\"Bay\": hatchHeld]; \
         default: hatchPayload = [hatchKey: hatchHeld] }"
    );
}
