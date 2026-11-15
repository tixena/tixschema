//! What the Swift backend writes for a map under a key Swift reads through a wrapper, a number, a
//! plain enum and a `bool`, in a list, in a list of optionals, in an optional list and in an
//! optional: the list or the optional is read as one of the wrapper and handed on as each
//! wrapper's storage, with no `try` around a conversion that throws nothing, as a field, as a
//! member of a variant under each tagging, as a brand's value, in a tuple struct's one slot, in a
//! tuple's slot, as a map's value and in a variant's slot, in a type that declares a parameter and
//! in one that declares none. A conversion that parses a date keeps its `try`.
//! `service_schema_emitted_client_tests/run_swift_listed_maps.rs` compiles the models of
//! `listed_maps_models.rs` under a Swift toolchain and reads a payload of each.

use std::collections::HashMap;

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::listed_maps_models::{
    AdjacentGalleyDraft, AdjacentStowedGalleyDraft, AtticDraft, CaddyDraft, CellarDraft, Creel,
    CupboardDraft, DresserDraft, GalleyDraft, Hopper, LarderDraft, PantryDraft, Rung,
    SideboardDraft, StowedAtticDraft, StowedCaddyDraft, StowedCellarDraft, StowedCreel,
    StowedCupboardDraft, StowedDresserDraft, StowedGalleyDraft, StowedLarderDraft,
    StowedPantryDraft, StowedSideboardDraft, TaggedGalleyDraft, TaggedStowedGalleyDraft, Trug,
    UntaggedGalleyDraft, UntaggedStowedGalleyDraft, adjacent_galley_swift,
    adjacent_stowed_galley_swift, attic_swift, caddy_swift, cellar_swift, creel_swift,
    cupboard_swift, dresser_swift, galley_swift, hopper_swift, larder_swift, pantry_swift,
    sideboard_swift, stowed_attic_swift, stowed_caddy_swift, stowed_cellar_swift,
    stowed_creel_swift, stowed_cupboard_swift, stowed_dresser_swift, stowed_galley_swift,
    stowed_larder_swift, stowed_pantry_swift, stowed_sideboard_swift, tagged_galley_swift,
    tagged_stowed_galley_swift, trug_swift, untagged_galley_swift, untagged_stowed_galley_swift,
};
#[cfg(feature = "chrono")]
use super::listed_maps_models::{
    DatelineDraft, PlankDraft, TimelineDraft, dateline_swift, timeline_swift,
};

/// The JSON serde writes for `held`, read back by serde as it was written.
fn read_back<Held>(held: &Held) -> String
where
    Held: DeserializeOwned + Serialize,
{
    let written = serde_json::to_string(held).unwrap();
    let read: Held = serde_json::from_str(&written).unwrap();
    assert_eq!(serde_json::to_string(&read).unwrap(), written);
    written
}

/// Each of `written` in `swift`, with `@held` filled with `held` and `@p` with `parameter`. A
/// template that reads another way where the type declares a parameter holds both, the second
/// after a `\u{1}`: for each such type, after its name and a `\u{3}`, the whole statement that hands
/// each reader to the wrapper, the statements of the types apart by a `\u{2}`.
fn holds(swift: &str, held: &str, parameter: &str, written: &[&str]) {
    for template in written {
        let (plain, with_readers) = template.split_once('\u{1}').unwrap_or((template, template));
        let chosen = if parameter.is_empty() {
            plain
        } else {
            with_readers
                .split('\u{2}')
                .filter_map(|entry| entry.split_once('\u{3}'))
                .find_map(|(named, read)| (named == held).then_some(read))
                .unwrap_or(with_readers)
        };
        let text = chosen.replace("@held", held).replace("@p", parameter);
        assert!(swift.contains(&text), "{text}\nin: {swift}");
    }
}

/// The text of every model of `listed_maps_models.rs` but the one that holds dates.
fn every_model() -> [String; 28] {
    [
        pantry_swift::swift_definition(),
        stowed_pantry_swift::swift_definition(),
        larder_swift::swift_definition(),
        stowed_larder_swift::swift_definition(),
        cellar_swift::swift_definition(),
        stowed_cellar_swift::swift_definition(),
        attic_swift::swift_definition(),
        stowed_attic_swift::swift_definition(),
        cupboard_swift::swift_definition(),
        sideboard_swift::swift_definition(),
        dresser_swift::swift_definition(),
        caddy_swift::swift_definition(),
        stowed_cupboard_swift::swift_definition(),
        stowed_sideboard_swift::swift_definition(),
        stowed_dresser_swift::swift_definition(),
        stowed_caddy_swift::swift_definition(),
        creel_swift::swift_definition(),
        trug_swift::swift_definition(),
        hopper_swift::swift_definition(),
        stowed_creel_swift::swift_definition(),
        galley_swift::swift_definition(),
        stowed_galley_swift::swift_definition(),
        adjacent_galley_swift::swift_definition(),
        adjacent_stowed_galley_swift::swift_definition(),
        tagged_galley_swift::swift_definition(),
        tagged_stowed_galley_swift::swift_definition(),
        untagged_galley_swift::swift_definition(),
        untagged_stowed_galley_swift::swift_definition(),
    ]
}

/// A map of `held` under the key `1`.
fn at_one<Held>(held: Held) -> HashMap<u32, Held> {
    HashMap::from([(1, held)])
}

/// A map of `held` under `true`.
fn at_true<Held>(held: Held) -> HashMap<bool, Held> {
    HashMap::from([(true, held)])
}

/// A map of `held` under [`Rung::Low`].
fn at_low<Held>(held: Held) -> HashMap<Rung, Held> {
    HashMap::from([(Rung::Low, held)])
}

/// The text `a`.
fn text() -> String {
    "a".to_owned()
}

#[test]
fn each_struct_of_fields_is_written_as_serde_writes_it() {
    let pantry = r#"{"defaulted":[],"flag_lists":[{"true":"a"}],"flag_lists_or_none":[null,{"true":"a"}],"maybe_flags":{"true":"a"},"maybe_number_lists":[{"1":"a"}],"number_lists":[],"number_lists_or_none":[],"rung_lists":[{"Low":"a"}],"rung_lists_or_none":[],"verbatim_lists":[]}"#;
    for (written, expected) in [
        (
            read_back(&PantryDraft {
                defaulted: Vec::new(),
                flag_lists: vec![at_true(text())],
                flag_lists_or_none: vec![None, Some(at_true(text()))],
                maybe_flag_lists: None,
                maybe_flags: Some(at_true(text())),
                maybe_number_lists: Some(vec![at_one(text())]),
                maybe_numbers: None,
                maybe_rung_lists: None,
                maybe_rungs: None,
                number_lists: Vec::new(),
                number_lists_or_none: Vec::new(),
                rung_lists: vec![at_low(text())],
                rung_lists_or_none: Vec::new(),
                verbatim_lists: Vec::new(),
            }),
            pantry,
        ),
        (
            read_back(&StowedPantryDraft {
                defaulted: Vec::new(),
                flag_lists: vec![at_true(text())],
                flag_lists_or_none: vec![None, Some(at_true(text()))],
                maybe_flag_lists: None,
                maybe_flags: Some(at_true(text())),
                maybe_number_lists: Some(vec![at_one(text())]),
                maybe_numbers: None,
                maybe_rung_lists: None,
                maybe_rungs: None,
                number_lists: Vec::new(),
                number_lists_or_none: Vec::new(),
                rung_lists: vec![at_low(text())],
                rung_lists_or_none: Vec::new(),
                verbatim_lists: Vec::new(),
            }),
            pantry,
        ),
    ] {
        assert_eq!(written, expected);
    }
}

#[test]
fn each_tuple_and_each_map_is_written_as_serde_writes_it() {
    let larder = r#"{"flag_lists_or_none":[[null],1],"maybe_numbers":[{"1":"a"},2],"maybe_rung_lists":[null,3],"number_lists":[[{"1":"a"}],4]}"#;
    let cellar = r#"[[{"1":"a"}],[null],[{"Low":"a"}],null]"#;
    let attic = r#"{"flag_lists_by_number":{"1":[{"true":"a"}]},"maybe_number_lists_by_name":{"n":null},"maybe_rungs_by_rung":{"Low":{"Low":"a"}},"number_lists_by_name":{"n":[{"1":"a"}]},"rung_lists_or_none_by_name":{"r":[null]}}"#;
    for (written, expected) in [
        (
            read_back(&LarderDraft {
                flag_lists_or_none: (vec![None], 1),
                maybe_numbers: (Some(at_one(text())), 2),
                maybe_rung_lists: (None, 3),
                number_lists: (vec![at_one(text())], 4),
            }),
            larder,
        ),
        (
            read_back(&StowedLarderDraft {
                flag_lists_or_none: (vec![None], 1),
                maybe_numbers: (Some(at_one(text())), 2),
                maybe_rung_lists: (None, 3),
                number_lists: (vec![at_one(text())], 4),
            }),
            larder,
        ),
        (
            read_back(&CellarDraft(
                vec![at_one(text())],
                vec![None],
                Some(vec![at_low(text())]),
                None,
            )),
            cellar,
        ),
        (
            read_back(&StowedCellarDraft(
                vec![at_one(text())],
                vec![None],
                Some(vec![at_low(text())]),
                None,
            )),
            cellar,
        ),
        (
            read_back(&AtticDraft {
                flag_lists_by_number: at_one(vec![at_true(text())]),
                maybe_number_lists_by_name: HashMap::from([("n".to_owned(), None)]),
                maybe_rungs_by_rung: at_low(Some(at_low(text()))),
                number_lists_by_name: HashMap::from([("n".to_owned(), vec![at_one(text())])]),
                rung_lists_or_none_by_name: HashMap::from([("r".to_owned(), vec![None])]),
            }),
            attic,
        ),
        (
            read_back(&StowedAtticDraft {
                flag_lists_by_number: at_one(vec![at_true(text())]),
                maybe_number_lists_by_name: HashMap::from([("n".to_owned(), None)]),
                maybe_rungs_by_rung: at_low(Some(at_low(text()))),
                number_lists_by_name: HashMap::from([("n".to_owned(), vec![at_one(text())])]),
                rung_lists_or_none_by_name: HashMap::from([("r".to_owned(), vec![None])]),
            }),
            attic,
        ),
    ] {
        assert_eq!(written, expected);
    }
}

#[test]
fn each_tuple_struct_of_one_slot_and_each_brand_is_written_as_serde_writes_it() {
    for (written, expected) in [
        (
            read_back(&CupboardDraft(vec![at_one(text())])),
            r#"[{"1":"a"}]"#,
        ),
        (
            read_back(&StowedCupboardDraft(vec![at_one(text())])),
            r#"[{"1":"a"}]"#,
        ),
        (
            read_back(&SideboardDraft(vec![Some(at_true(text())), None])),
            r#"[{"true":"a"},null]"#,
        ),
        (
            read_back(&StowedSideboardDraft(vec![Some(at_true(text())), None])),
            r#"[{"true":"a"},null]"#,
        ),
        (
            read_back(&DresserDraft(Some(vec![at_low(text())]))),
            r#"[{"Low":"a"}]"#,
        ),
        (read_back(&StowedDresserDraft::<String>(None)), "null"),
        (read_back(&CaddyDraft(None)), "null"),
        (
            read_back(&StowedCaddyDraft(Some(at_one(text())))),
            r#"{"1":"a"}"#,
        ),
        (
            read_back(&Creel::new(vec![at_one(text())]).unwrap()),
            r#"[{"1":"a"}]"#,
        ),
        (read_back(&Trug::new(vec![None]).unwrap()), "[null]"),
        (
            read_back(&Hopper::new(vec![at_low(text())]).unwrap()),
            r#"[{"Low":"a"}]"#,
        ),
        (
            read_back(&StowedCreel::new(vec![at_low(text())]).unwrap()),
            r#"[{"Low":"a"}]"#,
        ),
    ] {
        assert_eq!(written, expected);
    }
}

#[test]
fn each_variant_is_written_as_its_tagging_writes_it() {
    let stocked = r#"{"flag_lists":[],"number_lists_or_none":[null]}"#;
    let rungs = r#"{"rung_lists":[{"Low":"a"}],"rung_lists_or_none":[null]}"#;
    for (written, expected) in [
        (
            read_back(&GalleyDraft::Paired(vec![at_true(text())], None)),
            r#"{"Paired":[[{"true":"a"}],null]}"#.to_owned(),
        ),
        (
            read_back(&StowedGalleyDraft::<String>::Maybe(None)),
            r#"{"Maybe":null}"#.to_owned(),
        ),
        (
            read_back(&AdjacentGalleyDraft::MaybeListed(Some(vec![
                at_one(text()),
            ]))),
            r#"{"kind":"MaybeListed","held":[{"1":"a"}]}"#.to_owned(),
        ),
        (
            read_back(&AdjacentStowedGalleyDraft::<String>::Stocked {
                flag_lists: Vec::new(),
                maybe_number_lists: None,
                maybe_rungs: None,
                number_lists_or_none: vec![None],
            }),
            format!(r#"{{"kind":"Stocked","held":{stocked}}}"#),
        ),
        (
            read_back(&TaggedGalleyDraft::Stocked {
                maybe_rung_lists: None,
                maybe_rungs: None,
                rung_lists: vec![at_low(text())],
                rung_lists_or_none: vec![None],
            }),
            format!(r#"{{"kind":"Stocked",{}"#, &rungs[1..]),
        ),
        (
            read_back(&TaggedStowedGalleyDraft::Stocked {
                maybe_rung_lists: None,
                maybe_rungs: None,
                rung_lists: vec![at_low(text())],
                rung_lists_or_none: vec![None],
            }),
            format!(r#"{{"kind":"Stocked",{}"#, &rungs[1..]),
        ),
        (
            read_back(&UntaggedGalleyDraft::ListedOrNone(vec![
                Some(at_low(text())),
                None,
            ])),
            r#"[{"Low":"a"},null]"#.to_owned(),
        ),
        (
            read_back(&UntaggedStowedGalleyDraft::Stocked {
                maybe_rung_lists: None,
                maybe_rungs: None,
                rung_lists: vec![at_low(text())],
                rung_lists_or_none: vec![None],
            }),
            rungs.to_owned(),
        ),
    ] {
        assert_eq!(written, expected);
    }
}

/// serde reads a list of maps under a tag alone and in an untagged enum from what it buffers,
/// where it refuses a number key written as text: the models key every map there by a plain enum.
#[test]
fn serde_refuses_a_number_key_it_buffers_in_a_list() {
    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum Numbered {
        Listed(Vec<HashMap<u32, String>>),
    }
    assert!(serde_json::from_str::<Numbered>(r#"[{"1":"a"}]"#).is_err());
    assert!(
        serde_json::from_str::<Numbered>("[{}]")
            .is_ok_and(|Numbered::Listed(listed)| listed.len() == 1)
    );
}

#[test]
fn a_field_hands_on_the_storage_of_each_map_with_no_try() {
    for (swift, held, parameter) in [
        (pantry_swift::swift_definition(), "Pantry", ""),
        (
            stowed_pantry_swift::swift_definition(),
            "StowedPantry",
            "<Stowed>",
        ),
    ] {
        holds(
            &swift,
            held,
            parameter,
            &[
                "let wireFlagLists = try container.decode([@heldFlagListsKeyedMap@p].self, forKey: .flagLists); self.flagLists = wireFlagLists.map { $0.storage }; \u{1}StowedPantry\u{3}var listed1 = try container.nestedUnkeyedContainer(forKey: .flagLists); var items1: [[Bool: Stowed]] = []; while !listed1.isAtEnd { items1.append(try StowedPantryFlagListsKeyedMap<Stowed>(swiftSchemaUnjudged: listed1.superDecoder(), swiftSchemaReaderOfStowed).storage) }; self.flagLists = items1;",
                "let wireFlagListsOrNone = try container.decode([@heldFlagListsOrNoneKeyedMap@p?].self, forKey: .flagListsOrNone); self.flagListsOrNone = wireFlagListsOrNone.map { $0.map { $0.storage } }; \u{1}StowedPantry\u{3}var listed2 = try container.nestedUnkeyedContainer(forKey: .flagListsOrNone); var items2: [[Bool: Stowed]?] = []; while !listed2.isAtEnd { if try !listed2.decodeNil() { items2.append(try StowedPantryFlagListsOrNoneKeyedMap<Stowed>(swiftSchemaUnjudged: listed2.superDecoder(), swiftSchemaReaderOfStowed).storage) } else { items2.append(nil) } }; self.flagListsOrNone = items2;",
                "let wireMaybeFlagLists = try container.decodeIfPresent([@heldMaybeFlagListsKeyedMap@p].self, forKey: .maybeFlagLists); self.maybeFlagLists = wireMaybeFlagLists.map { $0.map { $0.storage } }; \u{1}StowedPantry\u{3}var listed3 = try container.nestedUnkeyedContainer(forKey: .maybeFlagLists); var items3: [[Bool: Stowed]] = []; while !listed3.isAtEnd { items3.append(try StowedPantryMaybeFlagListsKeyedMap<Stowed>(swiftSchemaUnjudged: listed3.superDecoder(), swiftSchemaReaderOfStowed).storage) }; self.maybeFlagLists = items3 } else { self.maybeFlagLists = nil };",
                "let wireMaybeFlags = try container.decodeIfPresent(@heldMaybeFlagsKeyedMap@p.self, forKey: .maybeFlags); self.maybeFlags = wireMaybeFlags.map { $0.storage }; \u{1}StowedPantry\u{3}self.maybeFlags = try StowedPantryMaybeFlagsKeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: .maybeFlags), swiftSchemaReaderOfStowed).storage } else { self.maybeFlags = nil };",
                "let wireMaybeNumberLists = try container.decodeIfPresent([@heldMaybeNumberListsKeyedMap@p].self, forKey: .maybeNumberLists); self.maybeNumberLists = wireMaybeNumberLists.map { $0.map { $0.storage } }; \u{1}StowedPantry\u{3}var listed4 = try container.nestedUnkeyedContainer(forKey: .maybeNumberLists); var items4: [[UInt32: Stowed]] = []; while !listed4.isAtEnd { items4.append(try StowedPantryMaybeNumberListsKeyedMap<Stowed>(swiftSchemaUnjudged: listed4.superDecoder(), swiftSchemaReaderOfStowed).storage) }; self.maybeNumberLists = items4 } else { self.maybeNumberLists = nil };",
                "let wireMaybeNumbers = try container.decodeIfPresent(@heldMaybeNumbersKeyedMap@p.self, forKey: .maybeNumbers); self.maybeNumbers = wireMaybeNumbers.map { $0.storage }; \u{1}StowedPantry\u{3}self.maybeNumbers = try StowedPantryMaybeNumbersKeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: .maybeNumbers), swiftSchemaReaderOfStowed).storage } else { self.maybeNumbers = nil };",
                "let wireMaybeRungLists = try container.decodeIfPresent([@heldMaybeRungListsKeyedMap@p].self, forKey: .maybeRungLists); self.maybeRungLists = wireMaybeRungLists.map { $0.map { $0.storage } }; \u{1}StowedPantry\u{3}var listed5 = try container.nestedUnkeyedContainer(forKey: .maybeRungLists); var items5: [[Rung: Stowed]] = []; while !listed5.isAtEnd { items5.append(try StowedPantryMaybeRungListsKeyedMap<Stowed>(swiftSchemaUnjudged: listed5.superDecoder(), swiftSchemaReaderOfStowed).storage) }; self.maybeRungLists = items5 } else { self.maybeRungLists = nil };",
                "let wireMaybeRungs = try container.decodeIfPresent(@heldMaybeRungsKeyedMap@p.self, forKey: .maybeRungs); self.maybeRungs = wireMaybeRungs.map { $0.storage }; \u{1}StowedPantry\u{3}self.maybeRungs = try StowedPantryMaybeRungsKeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: .maybeRungs), swiftSchemaReaderOfStowed).storage } else { self.maybeRungs = nil };",
                "let wireNumberLists = try container.decode([@heldNumberListsKeyedMap@p].self, forKey: .numberLists); self.numberLists = wireNumberLists.map { $0.storage }; \u{1}StowedPantry\u{3}var listed6 = try container.nestedUnkeyedContainer(forKey: .numberLists); var items6: [[UInt32: Stowed]] = []; while !listed6.isAtEnd { items6.append(try StowedPantryNumberListsKeyedMap<Stowed>(swiftSchemaUnjudged: listed6.superDecoder(), swiftSchemaReaderOfStowed).storage) }; self.numberLists = items6;",
                "let wireNumberListsOrNone = try container.decode([@heldNumberListsOrNoneKeyedMap@p?].self, forKey: .numberListsOrNone); self.numberListsOrNone = wireNumberListsOrNone.map { $0.map { $0.storage } }; \u{1}StowedPantry\u{3}var listed7 = try container.nestedUnkeyedContainer(forKey: .numberListsOrNone); var items7: [[UInt32: Stowed]?] = []; while !listed7.isAtEnd { if try !listed7.decodeNil() { items7.append(try StowedPantryNumberListsOrNoneKeyedMap<Stowed>(swiftSchemaUnjudged: listed7.superDecoder(), swiftSchemaReaderOfStowed).storage) } else { items7.append(nil) } }; self.numberListsOrNone = items7;",
                "let wireRungLists = try container.decode([@heldRungListsKeyedMap@p].self, forKey: .rungLists); self.rungLists = wireRungLists.map { $0.storage }; \u{1}StowedPantry\u{3}var listed8 = try container.nestedUnkeyedContainer(forKey: .rungLists); var items8: [[Rung: Stowed]] = []; while !listed8.isAtEnd { items8.append(try StowedPantryRungListsKeyedMap<Stowed>(swiftSchemaUnjudged: listed8.superDecoder(), swiftSchemaReaderOfStowed).storage) }; self.rungLists = items8;",
                "let wireRungListsOrNone = try container.decode([@heldRungListsOrNoneKeyedMap@p?].self, forKey: .rungListsOrNone); self.rungListsOrNone = wireRungListsOrNone.map { $0.map { $0.storage } }\u{1}StowedPantry\u{3}var listed9 = try container.nestedUnkeyedContainer(forKey: .rungListsOrNone); var items9: [[Rung: Stowed]?] = []; while !listed9.isAtEnd { if try !listed9.decodeNil() { items9.append(try StowedPantryRungListsOrNoneKeyedMap<Stowed>(swiftSchemaUnjudged: listed9.superDecoder(), swiftSchemaReaderOfStowed).storage) } else { items9.append(nil) } }; self.rungListsOrNone = items9;",
                "if container.contains(.defaulted) { let wireDefaulted = try container.decode([@heldDefaultedKeyedMap@p].self, forKey: .defaulted); self.defaulted = wireDefaulted.map { $0.storage } } else { \u{1}StowedPantry\u{3}var listed = try container.nestedUnkeyedContainer(forKey: .defaulted); var items: [[UInt32: Stowed]] = []; while !listed.isAtEnd { items.append(try StowedPantryDefaultedKeyedMap<Stowed>(swiftSchemaUnjudged: listed.superDecoder(), swiftSchemaReaderOfStowed).storage) }; self.defaulted = items } else { ",
                "let wireDefaultedDefault = try SwiftSchemaPayload.read([@heldDefaultedKeyedMap@p].self, from: defaultedDefaultPayload, at: container.codingPath + [CodingKeys.defaulted], within: decoder); self.defaulted = wireDefaultedDefault.map { $0.storage } }\u{1}StowedPantry\u{3}let defaultedDefaultPayload = try JSONSerialization.jsonObject(with: Data(\"[]\".utf8), options: [.fragmentsAllowed]); self.defaulted = try withoutActuallyEscaping(swiftSchemaReaderOfStowed) { swiftSchemaReaderOfStowed in try SwiftSchemaPayload.read(SwiftSchemaReadDefaultedDefault.self, from: defaultedDefaultPayload, at: container.codingPath + [CodingKeys.defaulted], within: decoder, handing: [\"Stowed\": swiftSchemaReaderOfStowed]) }.value }",
                "let wireVerbatimLists = try SwiftSchemaPayload.read([@heldVerbatimListsKeyedMap@p].self, from: verbatim(verbatimListsPayload), at: container.codingPath + [CodingKeys.verbatimLists], within: decoder); self.verbatimLists = wireVerbatimLists.map { $0.storage } }\u{1}StowedPantry\u{3}let verbatimListsPayload = try container.decode(SwiftSchemaPayload.self, forKey: .verbatimLists).value; self.verbatimLists = try withoutActuallyEscaping(swiftSchemaReaderOfStowed) { swiftSchemaReaderOfStowed in try SwiftSchemaPayload.read(SwiftSchemaReadVerbatimLists.self, from: verbatim(verbatimListsPayload), at: container.codingPath + [CodingKeys.verbatimLists], within: decoder, handing: [\"Stowed\": swiftSchemaReaderOfStowed]) }.value }",
                "let wireFlagListsOrNone = self.flagListsOrNone.map { $0.map { \
                 @heldFlagListsOrNoneKeyedMap@p(storage: $0) } }; ",
                "let wireMaybeFlags = self.maybeFlags.map { \
                 @heldMaybeFlagsKeyedMap@p(storage: $0) }; ",
            ],
        );
    }
}

#[test]
fn a_member_of_a_variant_hands_on_the_storage_of_each_map_under_each_tagging() {
    for (swift, held, parameter) in [
        (galley_swift::swift_definition(), "Galley", ""),
        (
            stowed_galley_swift::swift_definition(),
            "StowedGalley",
            "<Stowed>",
        ),
        (
            adjacent_galley_swift::swift_definition(),
            "AdjacentGalley",
            "",
        ),
        (
            adjacent_stowed_galley_swift::swift_definition(),
            "AdjacentStowedGalley",
            "<Stowed>",
        ),
    ] {
        holds(
            &swift,
            held,
            parameter,
            &[
                "self.flagLists = wireFlagLists.map { $0.storage }; \u{1}AdjacentStowedGalley\u{3}var listed = try container.nestedUnkeyedContainer(forKey: .flagLists); var items: [[Bool: Stowed]] = []; while !listed.isAtEnd { items.append(try AdjacentStowedGalleyStockedPayloadFlagListsKeyedMap<Stowed>(swiftSchemaUnjudged: listed.superDecoder(), swiftSchemaReaderOfStowed).storage) }; self.flagLists = items;\u{2}StowedGalley\u{3}var listed = try container.nestedUnkeyedContainer(forKey: .flagLists); var items: [[Bool: Stowed]] = []; while !listed.isAtEnd { items.append(try StowedGalleyStockedPayloadFlagListsKeyedMap<Stowed>(swiftSchemaUnjudged: listed.superDecoder(), swiftSchemaReaderOfStowed).storage) }; self.flagLists = items;\u{2}StowedPantry\u{3}var listed1 = try container.nestedUnkeyedContainer(forKey: .flagLists); var items1: [[Bool: Stowed]] = []; while !listed1.isAtEnd { items1.append(try StowedPantryFlagListsKeyedMap<Stowed>(swiftSchemaUnjudged: listed1.superDecoder(), swiftSchemaReaderOfStowed).storage) }; self.flagLists = items1;",
                "let wireMaybeNumberLists = try container.decodeIfPresent([@heldStockedPayloadMaybeNumberListsKeyedMap@p].self, forKey: .maybeNumberLists); self.maybeNumberLists = wireMaybeNumberLists.map { $0.map { $0.storage } }; \u{1}AdjacentStowedGalley\u{3}var listed1 = try container.nestedUnkeyedContainer(forKey: .maybeNumberLists); var items1: [[UInt32: Stowed]] = []; while !listed1.isAtEnd { items1.append(try AdjacentStowedGalleyStockedPayloadMaybeNumberListsKeyedMap<Stowed>(swiftSchemaUnjudged: listed1.superDecoder(), swiftSchemaReaderOfStowed).storage) }; self.maybeNumberLists = items1 } else { self.maybeNumberLists = nil };\u{2}StowedGalley\u{3}var listed1 = try container.nestedUnkeyedContainer(forKey: .maybeNumberLists); var items1: [[UInt32: Stowed]] = []; while !listed1.isAtEnd { items1.append(try StowedGalleyStockedPayloadMaybeNumberListsKeyedMap<Stowed>(swiftSchemaUnjudged: listed1.superDecoder(), swiftSchemaReaderOfStowed).storage) }; self.maybeNumberLists = items1 } else { self.maybeNumberLists = nil };",
                "self.maybeRungs = wireMaybeRungs.map { $0.storage }; \u{1}AdjacentStowedGalley\u{3}if container.contains(.maybeRungs), try !container.decodeNil(forKey: .maybeRungs) { self.maybeRungs = try AdjacentStowedGalleyStockedPayloadMaybeRungsKeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: .maybeRungs), swiftSchemaReaderOfStowed).storage } else { self.maybeRungs = nil };\u{2}StowedGalley\u{3}if container.contains(.maybeRungs), try !container.decodeNil(forKey: .maybeRungs) { self.maybeRungs = try StowedGalleyStockedPayloadMaybeRungsKeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: .maybeRungs), swiftSchemaReaderOfStowed).storage } else { self.maybeRungs = nil };\u{2}StowedPantry\u{3}if container.contains(.maybeRungs), try !container.decodeNil(forKey: .maybeRungs) { self.maybeRungs = try StowedPantryMaybeRungsKeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: .maybeRungs), swiftSchemaReaderOfStowed).storage } else { self.maybeRungs = nil };\u{2}TaggedStowedGalley\u{3}if container.contains(.maybeRungs), try !container.decodeNil(forKey: .maybeRungs) { self.maybeRungs = try TaggedStowedGalleyStockedPayloadMaybeRungsKeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: .maybeRungs), swiftSchemaReaderOfStowed).storage } else { self.maybeRungs = nil };\u{2}UntaggedStowedGalley\u{3}if container.contains(.maybeRungs), try !container.decodeNil(forKey: .maybeRungs) { self.maybeRungs = try UntaggedStowedGalleyStockedPayloadMaybeRungsKeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: .maybeRungs), swiftSchemaReaderOfStowed).storage } else { self.maybeRungs = nil };",
                "self.numberListsOrNone = wireNumberListsOrNone.map { $0.map { $0.storage } }\u{1}AdjacentStowedGalley\u{3}var listed2 = try container.nestedUnkeyedContainer(forKey: .numberListsOrNone); var items2: [[UInt32: Stowed]?] = []; while !listed2.isAtEnd { if try !listed2.decodeNil() { items2.append(try AdjacentStowedGalleyStockedPayloadNumberListsOrNoneKeyedMap<Stowed>(swiftSchemaUnjudged: listed2.superDecoder(), swiftSchemaReaderOfStowed).storage) } else { items2.append(nil) } }; self.numberListsOrNone = items2 };\u{2}StowedGalley\u{3}var listed2 = try container.nestedUnkeyedContainer(forKey: .numberListsOrNone); var items2: [[UInt32: Stowed]?] = []; while !listed2.isAtEnd { if try !listed2.decodeNil() { items2.append(try StowedGalleyStockedPayloadNumberListsOrNoneKeyedMap<Stowed>(swiftSchemaUnjudged: listed2.superDecoder(), swiftSchemaReaderOfStowed).storage) } else { items2.append(nil) } }; self.numberListsOrNone = items2 };\u{2}StowedPantry\u{3}var listed7 = try container.nestedUnkeyedContainer(forKey: .numberListsOrNone); var items7: [[UInt32: Stowed]?] = []; while !listed7.isAtEnd { if try !listed7.decodeNil() { items7.append(try StowedPantryNumberListsOrNoneKeyedMap<Stowed>(swiftSchemaUnjudged: listed7.superDecoder(), swiftSchemaReaderOfStowed).storage) } else { items7.append(nil) } }; self.numberListsOrNone = items7;",
            ],
        );
    }
    for (swift, held, parameter) in [
        (tagged_galley_swift::swift_definition(), "TaggedGalley", ""),
        (
            tagged_stowed_galley_swift::swift_definition(),
            "TaggedStowedGalley",
            "<Stowed>",
        ),
        (
            untagged_galley_swift::swift_definition(),
            "UntaggedGalley",
            "",
        ),
        (
            untagged_stowed_galley_swift::swift_definition(),
            "UntaggedStowedGalley",
            "<Stowed>",
        ),
    ] {
        holds(
            &swift,
            held,
            parameter,
            &[
                "let wireMaybeRungLists = try container.decodeIfPresent([@heldStockedPayloadMaybeRungListsKeyedMap@p].self, forKey: .maybeRungLists); self.maybeRungLists = wireMaybeRungLists.map { $0.map { $0.storage } }; \u{1}TaggedStowedGalley\u{3}var listed = try container.nestedUnkeyedContainer(forKey: .maybeRungLists); var items: [[Rung: Stowed]] = []; while !listed.isAtEnd { items.append(try TaggedStowedGalleyStockedPayloadMaybeRungListsKeyedMap<Stowed>(swiftSchemaUnjudged: listed.superDecoder(), swiftSchemaReaderOfStowed).storage) }; self.maybeRungLists = items } else { self.maybeRungLists = nil };\u{2}UntaggedStowedGalley\u{3}var listed = try container.nestedUnkeyedContainer(forKey: .maybeRungLists); var items: [[Rung: Stowed]] = []; while !listed.isAtEnd { items.append(try UntaggedStowedGalleyStockedPayloadMaybeRungListsKeyedMap<Stowed>(swiftSchemaUnjudged: listed.superDecoder(), swiftSchemaReaderOfStowed).storage) }; self.maybeRungLists = items } else { self.maybeRungLists = nil };",
                "self.maybeRungs = wireMaybeRungs.map { $0.storage }; \u{1}AdjacentStowedGalley\u{3}if container.contains(.maybeRungs), try !container.decodeNil(forKey: .maybeRungs) { self.maybeRungs = try AdjacentStowedGalleyStockedPayloadMaybeRungsKeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: .maybeRungs), swiftSchemaReaderOfStowed).storage } else { self.maybeRungs = nil };\u{2}StowedGalley\u{3}if container.contains(.maybeRungs), try !container.decodeNil(forKey: .maybeRungs) { self.maybeRungs = try StowedGalleyStockedPayloadMaybeRungsKeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: .maybeRungs), swiftSchemaReaderOfStowed).storage } else { self.maybeRungs = nil };\u{2}StowedPantry\u{3}if container.contains(.maybeRungs), try !container.decodeNil(forKey: .maybeRungs) { self.maybeRungs = try StowedPantryMaybeRungsKeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: .maybeRungs), swiftSchemaReaderOfStowed).storage } else { self.maybeRungs = nil };\u{2}TaggedStowedGalley\u{3}if container.contains(.maybeRungs), try !container.decodeNil(forKey: .maybeRungs) { self.maybeRungs = try TaggedStowedGalleyStockedPayloadMaybeRungsKeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: .maybeRungs), swiftSchemaReaderOfStowed).storage } else { self.maybeRungs = nil };\u{2}UntaggedStowedGalley\u{3}if container.contains(.maybeRungs), try !container.decodeNil(forKey: .maybeRungs) { self.maybeRungs = try UntaggedStowedGalleyStockedPayloadMaybeRungsKeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: .maybeRungs), swiftSchemaReaderOfStowed).storage } else { self.maybeRungs = nil };",
                "self.rungLists = wireRungLists.map { $0.storage }; \u{1}StowedPantry\u{3}var listed8 = try container.nestedUnkeyedContainer(forKey: .rungLists); var items8: [[Rung: Stowed]] = []; while !listed8.isAtEnd { items8.append(try StowedPantryRungListsKeyedMap<Stowed>(swiftSchemaUnjudged: listed8.superDecoder(), swiftSchemaReaderOfStowed).storage) }; self.rungLists = items8;\u{2}TaggedStowedGalley\u{3}var listed1 = try container.nestedUnkeyedContainer(forKey: .rungLists); var items1: [[Rung: Stowed]] = []; while !listed1.isAtEnd { items1.append(try TaggedStowedGalleyStockedPayloadRungListsKeyedMap<Stowed>(swiftSchemaUnjudged: listed1.superDecoder(), swiftSchemaReaderOfStowed).storage) }; self.rungLists = items1;\u{2}UntaggedStowedGalley\u{3}var listed1 = try container.nestedUnkeyedContainer(forKey: .rungLists); var items1: [[Rung: Stowed]] = []; while !listed1.isAtEnd { items1.append(try UntaggedStowedGalleyStockedPayloadRungListsKeyedMap<Stowed>(swiftSchemaUnjudged: listed1.superDecoder(), swiftSchemaReaderOfStowed).storage) }; self.rungLists = items1;",
                "self.rungListsOrNone = wireRungListsOrNone.map { $0.map { $0.storage } }\u{1}StowedPantry\u{3}var listed9 = try container.nestedUnkeyedContainer(forKey: .rungListsOrNone); var items9: [[Rung: Stowed]?] = []; while !listed9.isAtEnd { if try !listed9.decodeNil() { items9.append(try StowedPantryRungListsOrNoneKeyedMap<Stowed>(swiftSchemaUnjudged: listed9.superDecoder(), swiftSchemaReaderOfStowed).storage) } else { items9.append(nil) } }; self.rungListsOrNone = items9;\u{2}TaggedStowedGalley\u{3}var listed2 = try container.nestedUnkeyedContainer(forKey: .rungListsOrNone); var items2: [[Rung: Stowed]?] = []; while !listed2.isAtEnd { if try !listed2.decodeNil() { items2.append(try TaggedStowedGalleyStockedPayloadRungListsOrNoneKeyedMap<Stowed>(swiftSchemaUnjudged: listed2.superDecoder(), swiftSchemaReaderOfStowed).storage) } else { items2.append(nil) } }; self.rungListsOrNone = items2 };\u{2}UntaggedStowedGalley\u{3}var listed2 = try container.nestedUnkeyedContainer(forKey: .rungListsOrNone); var items2: [[Rung: Stowed]?] = []; while !listed2.isAtEnd { if try !listed2.decodeNil() { items2.append(try UntaggedStowedGalleyStockedPayloadRungListsOrNoneKeyedMap<Stowed>(swiftSchemaUnjudged: listed2.superDecoder(), swiftSchemaReaderOfStowed).storage) } else { items2.append(nil) } }; self.rungListsOrNone = items2 };",
            ],
        );
    }
}

#[test]
fn a_slot_of_a_variant_hands_on_the_storage_of_each_map_under_each_tagging() {
    for (swift, held, parameter, key) in [
        (galley_swift::swift_definition(), "Galley", "", "key"),
        (
            stowed_galley_swift::swift_definition(),
            "StowedGalley",
            "<Stowed>",
            "key",
        ),
        (
            adjacent_galley_swift::swift_definition(),
            "AdjacentGalley",
            "",
            ".swiftSchemaContent",
        ),
        (
            adjacent_stowed_galley_swift::swift_definition(),
            "AdjacentStowedGalley",
            "<Stowed>",
            ".swiftSchemaContent",
        ),
    ] {
        let at_key = |template: &str| template.replace("@key", key);
        holds(
            &swift,
            held,
            parameter,
            &[
                &at_key(
                    "let wireListed = try container.decode([@heldListedValueKeyedMap@p].self, \
                     forKey: @key); self = .listed(wireListed.map { $0.storage }); \u{1}case \
                     \"Listed\": var listed = try container.nestedUnkeyedContainer(forKey: @key); \
                     var items: [[UInt32: Stowed]] = []; while !listed.isAtEnd { \
                     items.append(try @heldListedValueKeyedMap@p(swiftSchemaUnjudged: \
                     listed.superDecoder(), swiftSchemaReaderOfStowed).storage) }; \
                     self = .listed(items); ",
                ),
                &at_key(
                    "let wireListedOrNone = try \
                     container.decode([@heldListedOrNoneValueKeyedMap@p?].self, forKey: @key); \
                     self = .listedOrNone(wireListedOrNone.map { $0.map { $0.storage } }); \
                     \u{1}case \"ListedOrNone\": var listed = try \
                     container.nestedUnkeyedContainer(forKey: @key); var items: [[Bool: Stowed]?] \
                     = []; while !listed.isAtEnd { if try !listed.decodeNil() { items.append(try \
                     @heldListedOrNoneValueKeyedMap@p(swiftSchemaUnjudged: listed.superDecoder(), \
                     swiftSchemaReaderOfStowed).storage) } else { items.append(nil) } }; \
                     self = .listedOrNone(items); ",
                ),
                &at_key(
                    "let wireMaybe = try container.decode(@heldMaybeValueKeyedMap@p?.self, \
                     forKey: @key); self = .maybe(wireMaybe.map { $0.storage }); \u{1}\
                     self = .maybe(try @heldMaybeValueKeyedMap@p(swiftSchemaUnjudged: \
                     container.superDecoder(forKey: @key), \
                     swiftSchemaReaderOfStowed).storage) } else { self = .maybe(nil) }; ",
                ),
                &at_key(
                    "let wireMaybeListed = try \
                     container.decode([@heldMaybeListedValueKeyedMap@p]?.self, forKey: @key); \
                     self = .maybeListed(wireMaybeListed.map { $0.map { $0.storage } }); \u{1}\
                     var listed = try container.nestedUnkeyedContainer(forKey: @key); var items: \
                     [[UInt32: Stowed]] = []; while !listed.isAtEnd { items.append(try \
                     @heldMaybeListedValueKeyedMap@p(swiftSchemaUnjudged: listed.superDecoder(), \
                     swiftSchemaReaderOfStowed).storage) }; self = .maybeListed(items) } else { \
                     self = .maybeListed(nil) }; ",
                ),
                "let wireSlot0 = try container.decode([@heldPairedTupleSlot0KeyedMap@p].self); slot0 = wireSlot0.map { $0.storage }; let wireSlot1 = try container.decode(@heldPairedTupleSlot1KeyedMap@p?.self); slot1 = wireSlot1.map { $0.storage }; \u{1}AdjacentStowedGalley\u{3}var listed = try container.nestedUnkeyedContainer(); var items: [[Bool: Stowed]] = []; while !listed.isAtEnd { items.append(try AdjacentStowedGalleyPairedTupleSlot0KeyedMap<Stowed>(swiftSchemaUnjudged: listed.superDecoder(), swiftSchemaReaderOfStowed).storage) }; slot0 = items; if try !container.decodeNil() { slot1 = try AdjacentStowedGalleyPairedTupleSlot1KeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(), swiftSchemaReaderOfStowed).storage } else { slot1 = nil };\u{2}StowedGalley\u{3}var listed = try container.nestedUnkeyedContainer(); var items: [[Bool: Stowed]] = []; while !listed.isAtEnd { items.append(try StowedGalleyPairedTupleSlot0KeyedMap<Stowed>(swiftSchemaUnjudged: listed.superDecoder(), swiftSchemaReaderOfStowed).storage) }; slot0 = items; if try !container.decodeNil() { slot1 = try StowedGalleyPairedTupleSlot1KeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(), swiftSchemaReaderOfStowed).storage } else { slot1 = nil };",
            ],
        );
    }
    for (swift, held, parameter) in [
        (
            untagged_galley_swift::swift_definition(),
            "UntaggedGalley",
            "",
        ),
        (
            untagged_stowed_galley_swift::swift_definition(),
            "UntaggedStowedGalley",
            "<Stowed>",
        ),
    ] {
        holds(
            &swift,
            held,
            parameter,
            &[
                "let wireListed = try [@heldListedValueKeyedMap@p](from: decoder); return wireListed.map { $0.storage } }\u{1}AdjacentStowedGalley\u{3}case \"Listed\": var listed = try container.nestedUnkeyedContainer(forKey: .swiftSchemaContent); var items: [[UInt32: Stowed]] = []; while !listed.isAtEnd { items.append(try AdjacentStowedGalleyListedValueKeyedMap<Stowed>(swiftSchemaUnjudged: listed.superDecoder(), swiftSchemaReaderOfStowed).storage) }; self = .listed(items);\u{2}StowedGalley\u{3}case \"Listed\": var listed = try container.nestedUnkeyedContainer(forKey: key); var items: [[UInt32: Stowed]] = []; while !listed.isAtEnd { items.append(try StowedGalleyListedValueKeyedMap<Stowed>(swiftSchemaUnjudged: listed.superDecoder(), swiftSchemaReaderOfStowed).storage) }; self = .listed(items);\u{2}UntaggedStowedGalley\u{3}var listed = try decoder.unkeyedContainer(); var items: [[Rung: Stowed]] = []; while !listed.isAtEnd { items.append(try UntaggedStowedGalleyListedValueKeyedMap<Stowed>(swiftSchemaUnjudged: listed.superDecoder(), swiftSchemaReaderOfStowed).storage) }; return items }",
                "let wireListedOrNone = try [@heldListedOrNoneValueKeyedMap@p?](from: decoder); return wireListedOrNone.map { $0.map { $0.storage } } }\u{1}AdjacentStowedGalley\u{3}case \"ListedOrNone\": var listed = try container.nestedUnkeyedContainer(forKey: .swiftSchemaContent); var items: [[Bool: Stowed]?] = []; while !listed.isAtEnd { if try !listed.decodeNil() { items.append(try AdjacentStowedGalleyListedOrNoneValueKeyedMap<Stowed>(swiftSchemaUnjudged: listed.superDecoder(), swiftSchemaReaderOfStowed).storage) } else { items.append(nil) } }; self = .listedOrNone(items);\u{2}StowedGalley\u{3}case \"ListedOrNone\": var listed = try container.nestedUnkeyedContainer(forKey: key); var items: [[Bool: Stowed]?] = []; while !listed.isAtEnd { if try !listed.decodeNil() { items.append(try StowedGalleyListedOrNoneValueKeyedMap<Stowed>(swiftSchemaUnjudged: listed.superDecoder(), swiftSchemaReaderOfStowed).storage) } else { items.append(nil) } }; self = .listedOrNone(items);\u{2}UntaggedStowedGalley\u{3}var listed = try decoder.unkeyedContainer(); var items: [[Rung: Stowed]?] = []; while !listed.isAtEnd { if try !listed.decodeNil() { items.append(try UntaggedStowedGalleyListedOrNoneValueKeyedMap<Stowed>(swiftSchemaUnjudged: listed.superDecoder(), swiftSchemaReaderOfStowed).storage) } else { items.append(nil) } }; return items }",
                "let wireMaybe = try @heldMaybeValueKeyedMap@p?(from: decoder); return wireMaybe.map { $0.storage } }\u{1}AdjacentStowedGalley\u{3}self = .maybe(try AdjacentStowedGalleyMaybeValueKeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: .swiftSchemaContent), swiftSchemaReaderOfStowed).storage) } else { self = .maybe(nil) }; case \"MaybeListed\": if try !container.decodeNil(forKey: .swiftSchemaContent) { var listed = try container.nestedUnkeyedContainer(forKey: .swiftSchemaContent); var items: [[UInt32: Stowed]] = []; while !listed.isAtEnd { items.append(try AdjacentStowedGalleyMaybeListedValueKeyedMap<Stowed>(swiftSchemaUnjudged: listed.superDecoder(), swiftSchemaReaderOfStowed).storage) }; self = .maybeListed(items) } else { self = .maybeListed(nil) }; case \"Paired\": _ = try container.decodeNil(forKey: .swiftSchemaContent); self = .paired(try AdjacentStowedGalleyPairedTuple<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: .swiftSchemaContent), swiftSchemaReaderOfStowed));\u{2}StowedGalley\u{3}self = .maybe(try StowedGalleyMaybeValueKeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: key), swiftSchemaReaderOfStowed).storage) } else { self = .maybe(nil) }; case \"MaybeListed\": if try !container.decodeNil(forKey: key) { var listed = try container.nestedUnkeyedContainer(forKey: key); var items: [[UInt32: Stowed]] = []; while !listed.isAtEnd { items.append(try StowedGalleyMaybeListedValueKeyedMap<Stowed>(swiftSchemaUnjudged: listed.superDecoder(), swiftSchemaReaderOfStowed).storage) }; self = .maybeListed(items) } else { self = .maybeListed(nil) }; case \"Paired\": self = .paired(try StowedGalleyPairedTuple<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: key), swiftSchemaReaderOfStowed));\u{2}UntaggedStowedGalley\u{3}return try UntaggedStowedGalleyMaybeValueKeyedMap<Stowed>(swiftSchemaUnjudged: decoder, swiftSchemaReaderOfStowed).storage } else { return nil } }",
            ],
        );
    }
}

#[test]
fn a_brand_and_a_tuple_struct_of_one_slot_hand_on_the_storage_of_each_map() {
    for (swift, written) in [
        (
            cupboard_swift::swift_definition(),
            "self.value = try { let wireValue = try container.decode([CupboardKeyedMap].self); \
             return wireValue.map { $0.storage } }() }; ",
        ),
        (
            sideboard_swift::swift_definition(),
            "self.value = try { let wireValue = try container.decode([SideboardKeyedMap?].self); \
             return wireValue.map { $0.map { $0.storage } } }() }; ",
        ),
        (
            dresser_swift::swift_definition(),
            "self.value = try { let wireValue = try container.decode([DresserKeyedMap]?.self); \
             return wireValue.map { $0.map { $0.storage } } }() }; ",
        ),
        (
            caddy_swift::swift_definition(),
            "self.value = try { let wireValue = try container.decode(CaddyKeyedMap?.self); \
             return wireValue.map { $0.storage } }() }; ",
        ),
        (
            stowed_cupboard_swift::swift_definition(),
            "fileprivate init(swiftSchemaUnjudged decoder: Decoder, _ swiftSchemaReaderOfStowed: (Decoder) throws -> Stowed) throws { var listed = try decoder.unkeyedContainer(); var items: [[UInt32: Stowed]] = []; while !listed.isAtEnd { items.append(try StowedCupboardKeyedMap<Stowed>(swiftSchemaUnjudged: listed.superDecoder(), swiftSchemaReaderOfStowed).storage) }; self.value = items }; fileprivate init(swiftSchemaUnjudged decoder: Decoder) throws { try self.init(swiftSchemaUnjudged: decoder, { try SwiftSchemaFit<Stowed>(from: $0).value }) }; ",
        ),
        (
            stowed_sideboard_swift::swift_definition(),
            "fileprivate init(swiftSchemaUnjudged decoder: Decoder, _ swiftSchemaReaderOfStowed: (Decoder) throws -> Stowed) throws { var listed = try decoder.unkeyedContainer(); var items: [[Bool: Stowed]?] = []; while !listed.isAtEnd { if try !listed.decodeNil() { items.append(try StowedSideboardKeyedMap<Stowed>(swiftSchemaUnjudged: listed.superDecoder(), swiftSchemaReaderOfStowed).storage) } else { items.append(nil) } }; self.value = items }; fileprivate init(swiftSchemaUnjudged decoder: Decoder) throws { try self.init(swiftSchemaUnjudged: decoder, { try SwiftSchemaFit<Stowed>(from: $0).value }) }; ",
        ),
        (
            stowed_dresser_swift::swift_definition(),
            "fileprivate init(swiftSchemaUnjudged decoder: Decoder, _ swiftSchemaReaderOfStowed: (Decoder) throws -> Stowed) throws { if try !decoder.singleValueContainer().decodeNil() { var listed = try decoder.unkeyedContainer(); var items: [[Rung: Stowed]] = []; while !listed.isAtEnd { items.append(try StowedDresserKeyedMap<Stowed>(swiftSchemaUnjudged: listed.superDecoder(), swiftSchemaReaderOfStowed).storage) }; self.value = items } else { self.value = nil } }; fileprivate init(swiftSchemaUnjudged decoder: Decoder) throws { try self.init(swiftSchemaUnjudged: decoder, { try SwiftSchemaFit<Stowed>(from: $0).value }) }; ",
        ),
        (
            stowed_caddy_swift::swift_definition(),
            "fileprivate init(swiftSchemaUnjudged decoder: Decoder, _ swiftSchemaReaderOfStowed: (Decoder) throws -> Stowed) throws { if try !decoder.singleValueContainer().decodeNil() { self.value = try StowedCaddyKeyedMap<Stowed>(swiftSchemaUnjudged: decoder, swiftSchemaReaderOfStowed).storage } else { self.value = nil } }; fileprivate init(swiftSchemaUnjudged decoder: Decoder) throws { try self.init(swiftSchemaUnjudged: decoder, { try SwiftSchemaFit<Stowed>(from: $0).value }) }; ",
        ),
        (
            creel_swift::swift_definition(),
            "self.value = try { let wireValue = try container.decode([CreelKeyedMap].self); \
             return wireValue.map { $0.storage } }() }; ",
        ),
        (
            trug_swift::swift_definition(),
            "self.value = try { let wireValue = try container.decode([TrugKeyedMap?].self); \
             return wireValue.map { $0.map { $0.storage } } }() }; ",
        ),
        (
            hopper_swift::swift_definition(),
            "self.value = try { let wireValue = try container.decode([HopperKeyedMap].self); \
             return wireValue.map { $0.storage } }() }; ",
        ),
        (
            stowed_creel_swift::swift_definition(),
            "self.value = try { let wireValue = try \
             container.decode([StowedCreelKeyedMap<Stowed>].self); \
             return wireValue.map { $0.storage } }() }; ",
        ),
    ] {
        holds(&swift, "", "", &[written]);
    }
}

#[test]
fn a_tuple_hands_on_the_storage_of_each_map_in_its_slot() {
    for (swift, held, parameter) in [
        (larder_swift::swift_definition(), "Larder", ""),
        (
            stowed_larder_swift::swift_definition(),
            "StowedLarder",
            "<Stowed>",
        ),
    ] {
        holds(
            &swift,
            held,
            parameter,
            &[
                "let wireSlot0 = try container.decode([@heldFlagListsOrNoneTupleSlot0KeyedMap@p?].self); slot0 = wireSlot0.map { $0.map { $0.storage } }; \u{1}StowedLarder\u{3}var listed = try container.nestedUnkeyedContainer(); var items: [[Bool: Stowed]?] = []; while !listed.isAtEnd { if try !listed.decodeNil() { items.append(try StowedLarderFlagListsOrNoneTupleSlot0KeyedMap<Stowed>(swiftSchemaUnjudged: listed.superDecoder(), swiftSchemaReaderOfStowed).storage) } else { items.append(nil) } }; slot0 = items;",
                "let wireSlot0 = try container.decode(@heldMaybeNumbersTupleSlot0KeyedMap@p?.self); slot0 = wireSlot0.map { $0.storage }; \u{1}StowedLarder\u{3}slot0 = try StowedLarderMaybeNumbersTupleSlot0KeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(), swiftSchemaReaderOfStowed).storage } else { slot0 = nil };",
                "let wireSlot0 = try container.decode([@heldMaybeRungListsTupleSlot0KeyedMap@p]?.self); slot0 = wireSlot0.map { $0.map { $0.storage } }; \u{1}StowedLarder\u{3}var listed = try container.nestedUnkeyedContainer(); var items: [[Rung: Stowed]] = []; while !listed.isAtEnd { items.append(try StowedLarderMaybeRungListsTupleSlot0KeyedMap<Stowed>(swiftSchemaUnjudged: listed.superDecoder(), swiftSchemaReaderOfStowed).storage) }; slot0 = items } else { slot0 = nil };",
                "let wireSlot0 = try container.decode([@heldNumberListsTupleSlot0KeyedMap@p].self); slot0 = wireSlot0.map { $0.storage }; \u{1}StowedLarder\u{3}var listed = try container.nestedUnkeyedContainer(); var items: [[UInt32: Stowed]] = []; while !listed.isAtEnd { items.append(try StowedLarderNumberListsTupleSlot0KeyedMap<Stowed>(swiftSchemaUnjudged: listed.superDecoder(), swiftSchemaReaderOfStowed).storage) }; slot0 = items;",
            ],
        );
    }
    for (swift, held, parameter) in [
        (cellar_swift::swift_definition(), "Cellar", ""),
        (
            stowed_cellar_swift::swift_definition(),
            "StowedCellar",
            "<Stowed>",
        ),
    ] {
        holds(
            &swift,
            held,
            parameter,
            &[
                "let wireSlot0 = try container.decode([@heldTupleSlot0KeyedMap@p].self); slot0 = wireSlot0.map { $0.storage }; let wireSlot1 = try container.decode([@heldTupleSlot1KeyedMap@p?].self); slot1 = wireSlot1.map { $0.map { $0.storage } }; let wireSlot2 = try container.decode([@heldTupleSlot2KeyedMap@p]?.self); slot2 = wireSlot2.map { $0.map { $0.storage } }; let wireSlot3 = try container.decode(@heldTupleSlot3KeyedMap@p?.self); slot3 = wireSlot3.map { $0.storage }; \u{1}StowedCabinet\u{3}slot0 = try StowedCabinetTupleSlot0KeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(), swiftSchemaReaderOfStowed).storage; slot1 = try StowedCabinetTupleSlot1KeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(), swiftSchemaReaderOfStowed).storage; slot2 = try StowedCabinetTupleSlot2KeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(), swiftSchemaReaderOfStowed).storage; slot3 = try StowedCabinetTupleSlot3KeyedMap(swiftSchemaUnjudged: container.superDecoder()).storage;\u{2}StowedCellar\u{3}var listed = try container.nestedUnkeyedContainer(); var items: [[UInt32: Stowed]] = []; while !listed.isAtEnd { items.append(try StowedCellarTupleSlot0KeyedMap<Stowed>(swiftSchemaUnjudged: listed.superDecoder(), swiftSchemaReaderOfStowed).storage) }; slot0 = items; var listed1 = try container.nestedUnkeyedContainer(); var items1: [[Bool: Stowed]?] = []; while !listed1.isAtEnd { if try !listed1.decodeNil() { items1.append(try StowedCellarTupleSlot1KeyedMap<Stowed>(swiftSchemaUnjudged: listed1.superDecoder(), swiftSchemaReaderOfStowed).storage) } else { items1.append(nil) } }; slot1 = items1; if try !container.decodeNil() { var listed2 = try container.nestedUnkeyedContainer(); var items2: [[Rung: Stowed]] = []; while !listed2.isAtEnd { items2.append(try StowedCellarTupleSlot2KeyedMap<Stowed>(swiftSchemaUnjudged: listed2.superDecoder(), swiftSchemaReaderOfStowed).storage) }; slot2 = items2 } else { slot2 = nil }; if try !container.decodeNil() { slot3 = try StowedCellarTupleSlot3KeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(), swiftSchemaReaderOfStowed).storage } else { slot3 = nil };",
            ],
        );
    }
}

#[test]
fn a_map_hands_on_the_storage_of_each_map_it_holds() {
    for (swift, held, parameter) in [
        (attic_swift::swift_definition(), "Attic", ""),
        (
            stowed_attic_swift::swift_definition(),
            "StowedAttic",
            "<Stowed>",
        ),
    ] {
        holds(
            &swift,
            held,
            parameter,
            &[
                "let wireValue = try \
                 container.decode([@heldFlagListsByNumberValueKeyedMap@p].self, \
                 forKey: codingKey); result[key] = wireValue.map { $0.storage } }; ",
                "self.maybeNumberListsByName = wireMaybeNumberListsByName.mapValues { $0.map { $0.map { $0.storage } } }; \u{1}StowedAttic\u{3}var listed = try keyed.nestedUnkeyedContainer(forKey: entry); var items: [[UInt32: Stowed]] = []; while !listed.isAtEnd { items.append(try StowedAtticMaybeNumberListsByNameValueKeyedMap<Stowed>(swiftSchemaUnjudged: listed.superDecoder(), swiftSchemaReaderOfStowed).storage) }; _ = entries.updateValue(items, forKey: entry.stringValue) } else { _ = entries.updateValue(nil, forKey: entry.stringValue) } }; self.maybeNumberListsByName = entries;",
                "let wireValue = try \
                 container.decode(@heldMaybeRungsByRungValueKeyedMap@p?.self, \
                 forKey: codingKey); \
                 _ = result.updateValue(wireValue.map { $0.storage }, forKey: key) }; ",
                "self.numberListsByName = wireNumberListsByName.mapValues { $0.map { $0.storage } }; \u{1}StowedAttic\u{3}var listed1 = try keyed1.nestedUnkeyedContainer(forKey: entry1); var items1: [[UInt32: Stowed]] = []; while !listed1.isAtEnd { items1.append(try StowedAtticNumberListsByNameValueKeyedMap<Stowed>(swiftSchemaUnjudged: listed1.superDecoder(), swiftSchemaReaderOfStowed).storage) }; entries1[entry1.stringValue] = items1 }; self.numberListsByName = entries1;",
                "self.rungListsOrNoneByName = wireRungListsOrNoneByName.mapValues { $0.map { $0.map { $0.storage } } }\u{1}StowedAttic\u{3}var listed2 = try keyed2.nestedUnkeyedContainer(forKey: entry2); var items2: [[Rung: Stowed]?] = []; while !listed2.isAtEnd { if try !listed2.decodeNil() { items2.append(try StowedAtticRungListsOrNoneByNameValueKeyedMap<Stowed>(swiftSchemaUnjudged: listed2.superDecoder(), swiftSchemaReaderOfStowed).storage) } else { items2.append(nil) } }; entries2[entry2.stringValue] = items2 }; self.rungListsOrNoneByName = entries2 };",
            ],
        );
    }
}

/// A wrapper's storage throws nothing, so no conversion of a model of `listed_maps_models.rs`
/// writes a `try` ahead of the value it reads or of `$0`.
#[test]
fn no_try_is_written_ahead_of_a_conversion_that_throws_nothing() {
    for swift in every_model() {
        for tried in ["try wire", "try $0"] {
            assert!(!swift.contains(tried), "{tried} in: {swift}");
        }
    }
}

/// A list of maps of dates Swift keys natively parses each date in the conversion, which keeps
/// its `try`; a list of maps of dates under a key Swift wraps hands on each wrapper's storage,
/// the wrapper parsing the dates.
#[cfg(feature = "chrono")]
#[test]
fn a_conversion_that_parses_a_date_keeps_its_try() {
    let date = chrono::DateTime::from_timestamp_millis(1_704_164_645_678).unwrap();
    assert_eq!(
        read_back(&TimelineDraft {
            by_name_lists: vec![HashMap::from([("n".to_owned(), date)])],
            by_number_lists: vec![at_one(date)],
            maybe_by_name: None,
        }),
        r#"{"by_name_lists":[{"n":"2024-01-02T03:04:05.678Z"}],"by_number_lists":[{"1":"2024-01-02T03:04:05.678Z"}]}"#
    );
    holds(
        &timeline_swift::swift_definition(),
        "",
        "",
        &[
            "self.byNameLists = try wireByNameLists.map { try $0.mapValues { \
             try Self.swiftSchemaParseIso8601Date($0, codingPath: decoder.codingPath) } }; ",
            "self.byNumberLists = wireByNumberLists.map { $0.storage }; ",
            "self.maybeByName = try wireMaybeByName.map { try $0.mapValues { \
             try Self.swiftSchemaParseIso8601Date($0, codingPath: decoder.codingPath) } } }",
            "let wireValue = try container.decode(String.self, forKey: codingKey); \
             result[key] = try Self.swiftSchemaParseIso8601Date(wireValue, \
             codingPath: decoder.codingPath) }",
        ],
    );
}

/// A map whose value is an optional is stored through `updateValue`, as assigning a `nil` through
/// the subscript removes the entry: under a key Swift wraps, where the value is converted, and
/// under a key Swift keys natively, where `validate()` walks the value.
#[cfg(feature = "chrono")]
#[test]
fn an_optional_value_is_stored_with_update_value_so_a_null_entry_is_kept() {
    let date = chrono::DateTime::from_timestamp_millis(1_704_164_645_678).unwrap();
    assert_eq!(
        read_back(&DatelineDraft {
            maybe_by_name: HashMap::from([("n".to_owned(), None)]),
            maybe_by_number: HashMap::from([(1, Some(date))]),
        }),
        r#"{"maybe_by_name":{"n":null},"maybe_by_number":{"1":"2024-01-02T03:04:05.678Z"}}"#
    );
    assert_eq!(
        read_back(&DatelineDraft {
            maybe_by_name: HashMap::from([(
                "m".to_owned(),
                Some(
                    PlankDraft {
                        label: "a".to_owned()
                    }
                    .validate()
                    .unwrap()
                )
            )]),
            maybe_by_number: HashMap::new(),
        }),
        r#"{"maybe_by_name":{"m":{"label":"a"}},"maybe_by_number":{}}"#
    );
    holds(
        &dateline_swift::swift_definition(),
        "",
        "",
        &[
            "_ = result.updateValue(try wireValue.map { try Self.swiftSchemaParseIso8601Date($0, \
             codingPath: decoder.codingPath) }, forKey: key) }",
            "_ = entries.updateValue(nil, forKey: entry.stringValue) }",
        ],
    );
}
