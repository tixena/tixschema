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
    AdjacentGalley, AdjacentStowedGalley, Attic, Caddy, Cellar, Creel, Cupboard, Dresser, Galley,
    Hopper, Larder, Pantry, Rung, Sideboard, StowedAttic, StowedCaddy, StowedCellar, StowedCreel,
    StowedCupboard, StowedDresser, StowedGalley, StowedLarder, StowedPantry, StowedSideboard,
    TaggedGalley, TaggedStowedGalley, Trug, UntaggedGalley, UntaggedStowedGalley,
    adjacent_galley_swift, adjacent_stowed_galley_swift, attic_swift, caddy_swift, cellar_swift,
    creel_swift, cupboard_swift, dresser_swift, galley_swift, hopper_swift, larder_swift,
    pantry_swift, sideboard_swift, stowed_attic_swift, stowed_caddy_swift, stowed_cellar_swift,
    stowed_creel_swift, stowed_cupboard_swift, stowed_dresser_swift, stowed_galley_swift,
    stowed_larder_swift, stowed_pantry_swift, stowed_sideboard_swift, tagged_galley_swift,
    tagged_stowed_galley_swift, trug_swift, untagged_galley_swift, untagged_stowed_galley_swift,
};
#[cfg(feature = "chrono")]
use super::listed_maps_models::{Timeline, timeline_swift};

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

/// Each of `written` in `swift`, with `@held` filled with `held` and `@p` with `parameter`.
fn holds(swift: &str, held: &str, parameter: &str, written: &[&str]) {
    for template in written {
        let text = template.replace("@held", held).replace("@p", parameter);
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
            read_back(&Pantry {
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
            read_back(&StowedPantry {
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
            read_back(&Larder {
                flag_lists_or_none: (vec![None], 1),
                maybe_numbers: (Some(at_one(text())), 2),
                maybe_rung_lists: (None, 3),
                number_lists: (vec![at_one(text())], 4),
            }),
            larder,
        ),
        (
            read_back(&StowedLarder {
                flag_lists_or_none: (vec![None], 1),
                maybe_numbers: (Some(at_one(text())), 2),
                maybe_rung_lists: (None, 3),
                number_lists: (vec![at_one(text())], 4),
            }),
            larder,
        ),
        (
            read_back(&Cellar(
                vec![at_one(text())],
                vec![None],
                Some(vec![at_low(text())]),
                None,
            )),
            cellar,
        ),
        (
            read_back(&StowedCellar(
                vec![at_one(text())],
                vec![None],
                Some(vec![at_low(text())]),
                None,
            )),
            cellar,
        ),
        (
            read_back(&Attic {
                flag_lists_by_number: at_one(vec![at_true(text())]),
                maybe_number_lists_by_name: HashMap::from([("n".to_owned(), None)]),
                maybe_rungs_by_rung: at_low(Some(at_low(text()))),
                number_lists_by_name: HashMap::from([("n".to_owned(), vec![at_one(text())])]),
                rung_lists_or_none_by_name: HashMap::from([("r".to_owned(), vec![None])]),
            }),
            attic,
        ),
        (
            read_back(&StowedAttic {
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
        (read_back(&Cupboard(vec![at_one(text())])), r#"[{"1":"a"}]"#),
        (
            read_back(&StowedCupboard(vec![at_one(text())])),
            r#"[{"1":"a"}]"#,
        ),
        (
            read_back(&Sideboard(vec![Some(at_true(text())), None])),
            r#"[{"true":"a"},null]"#,
        ),
        (
            read_back(&StowedSideboard(vec![Some(at_true(text())), None])),
            r#"[{"true":"a"},null]"#,
        ),
        (
            read_back(&Dresser(Some(vec![at_low(text())]))),
            r#"[{"Low":"a"}]"#,
        ),
        (read_back(&StowedDresser::<String>(None)), "null"),
        (read_back(&Caddy(None)), "null"),
        (
            read_back(&StowedCaddy(Some(at_one(text())))),
            r#"{"1":"a"}"#,
        ),
        (read_back(&Creel(vec![at_one(text())])), r#"[{"1":"a"}]"#),
        (read_back(&Trug(vec![None])), "[null]"),
        (
            read_back(&Hopper {
                rung_lists: vec![at_low(text())],
            }),
            r#"[{"Low":"a"}]"#,
        ),
        (
            read_back(&StowedCreel(vec![at_low(text())])),
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
            read_back(&Galley::Paired(vec![at_true(text())], None)),
            r#"{"Paired":[[{"true":"a"}],null]}"#.to_owned(),
        ),
        (
            read_back(&StowedGalley::<String>::Maybe(None)),
            r#"{"Maybe":null}"#.to_owned(),
        ),
        (
            read_back(&AdjacentGalley::MaybeListed(Some(vec![at_one(text())]))),
            r#"{"kind":"MaybeListed","held":[{"1":"a"}]}"#.to_owned(),
        ),
        (
            read_back(&AdjacentStowedGalley::<String>::Stocked {
                flag_lists: Vec::new(),
                maybe_number_lists: None,
                maybe_rungs: None,
                number_lists_or_none: vec![None],
            }),
            format!(r#"{{"kind":"Stocked","held":{stocked}}}"#),
        ),
        (
            read_back(&TaggedGalley::Stocked {
                maybe_rung_lists: None,
                maybe_rungs: None,
                rung_lists: vec![at_low(text())],
                rung_lists_or_none: vec![None],
            }),
            format!(r#"{{"kind":"Stocked",{}"#, &rungs[1..]),
        ),
        (
            read_back(&TaggedStowedGalley::Stocked {
                maybe_rung_lists: None,
                maybe_rungs: None,
                rung_lists: vec![at_low(text())],
                rung_lists_or_none: vec![None],
            }),
            format!(r#"{{"kind":"Stocked",{}"#, &rungs[1..]),
        ),
        (
            read_back(&UntaggedGalley::ListedOrNone(vec![
                Some(at_low(text())),
                None,
            ])),
            r#"[{"Low":"a"},null]"#.to_owned(),
        ),
        (
            read_back(&UntaggedStowedGalley::Stocked {
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
                "let wireFlagLists = try container.decode([@heldFlagListsKeyedMap@p].self, \
                 forKey: .flagLists); self.flagLists = wireFlagLists.map { $0.storage }; ",
                "let wireFlagListsOrNone = try \
                 container.decode([@heldFlagListsOrNoneKeyedMap@p?].self, \
                 forKey: .flagListsOrNone); \
                 self.flagListsOrNone = wireFlagListsOrNone.map { $0.map { $0.storage } }; ",
                "let wireMaybeFlagLists = try \
                 container.decodeIfPresent([@heldMaybeFlagListsKeyedMap@p].self, \
                 forKey: .maybeFlagLists); \
                 self.maybeFlagLists = wireMaybeFlagLists.map { $0.map { $0.storage } }; ",
                "let wireMaybeFlags = try container.decodeIfPresent(@heldMaybeFlagsKeyedMap@p.self, \
                 forKey: .maybeFlags); self.maybeFlags = wireMaybeFlags.map { $0.storage }; ",
                "let wireMaybeNumberLists = try \
                 container.decodeIfPresent([@heldMaybeNumberListsKeyedMap@p].self, \
                 forKey: .maybeNumberLists); \
                 self.maybeNumberLists = wireMaybeNumberLists.map { $0.map { $0.storage } }; ",
                "let wireMaybeNumbers = try \
                 container.decodeIfPresent(@heldMaybeNumbersKeyedMap@p.self, \
                 forKey: .maybeNumbers); self.maybeNumbers = wireMaybeNumbers.map { $0.storage }; ",
                "let wireMaybeRungLists = try \
                 container.decodeIfPresent([@heldMaybeRungListsKeyedMap@p].self, \
                 forKey: .maybeRungLists); \
                 self.maybeRungLists = wireMaybeRungLists.map { $0.map { $0.storage } }; ",
                "let wireMaybeRungs = try container.decodeIfPresent(@heldMaybeRungsKeyedMap@p.self, \
                 forKey: .maybeRungs); self.maybeRungs = wireMaybeRungs.map { $0.storage }; ",
                "let wireNumberLists = try container.decode([@heldNumberListsKeyedMap@p].self, \
                 forKey: .numberLists); self.numberLists = wireNumberLists.map { $0.storage }; ",
                "let wireNumberListsOrNone = try \
                 container.decode([@heldNumberListsOrNoneKeyedMap@p?].self, \
                 forKey: .numberListsOrNone); \
                 self.numberListsOrNone = wireNumberListsOrNone.map { $0.map { $0.storage } }; ",
                "let wireRungLists = try container.decode([@heldRungListsKeyedMap@p].self, \
                 forKey: .rungLists); self.rungLists = wireRungLists.map { $0.storage }; ",
                "let wireRungListsOrNone = try \
                 container.decode([@heldRungListsOrNoneKeyedMap@p?].self, \
                 forKey: .rungListsOrNone); \
                 self.rungListsOrNone = wireRungListsOrNone.map { $0.map { $0.storage } }",
                "if container.contains(.defaulted) { let wireDefaulted = try \
                 container.decode([@heldDefaultedKeyedMap@p].self, forKey: .defaulted); \
                 self.defaulted = wireDefaulted.map { $0.storage } } else { ",
                "let wireDefaultedDefault = try \
                 SwiftSchemaPayload.read([@heldDefaultedKeyedMap@p].self, \
                 from: defaultedDefaultPayload, at: container.codingPath + \
                 [CodingKeys.defaulted]); self.defaulted = wireDefaultedDefault.map { $0.storage } }",
                "let wireVerbatimLists = try \
                 SwiftSchemaPayload.read([@heldVerbatimListsKeyedMap@p].self, \
                 from: verbatim(verbatimListsPayload), at: container.codingPath + \
                 [CodingKeys.verbatimLists]); \
                 self.verbatimLists = wireVerbatimLists.map { $0.storage } }",
                "let wireFlagListsOrNone = flagListsOrNone.map { $0.map { \
                 @heldFlagListsOrNoneKeyedMap@p(storage: $0) } }; ",
                "let wireMaybeFlags = maybeFlags.map { @heldMaybeFlagsKeyedMap@p(storage: $0) }; ",
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
                "self.flagLists = wireFlagLists.map { $0.storage }; ",
                "let wireMaybeNumberLists = try \
                 container.decodeIfPresent([@heldStockedPayloadMaybeNumberListsKeyedMap@p].self, \
                 forKey: .maybeNumberLists); \
                 self.maybeNumberLists = wireMaybeNumberLists.map { $0.map { $0.storage } }; ",
                "self.maybeRungs = wireMaybeRungs.map { $0.storage }; ",
                "self.numberListsOrNone = wireNumberListsOrNone.map { $0.map { $0.storage } }",
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
                "let wireMaybeRungLists = try \
                 container.decodeIfPresent([@heldStockedPayloadMaybeRungListsKeyedMap@p].self, \
                 forKey: .maybeRungLists); \
                 self.maybeRungLists = wireMaybeRungLists.map { $0.map { $0.storage } }; ",
                "self.maybeRungs = wireMaybeRungs.map { $0.storage }; ",
                "self.rungLists = wireRungLists.map { $0.storage }; ",
                "self.rungListsOrNone = wireRungListsOrNone.map { $0.map { $0.storage } }",
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
                     forKey: @key); self = .listed(wireListed.map { $0.storage }); ",
                ),
                &at_key(
                    "let wireListedOrNone = try \
                     container.decode([@heldListedOrNoneValueKeyedMap@p?].self, forKey: @key); \
                     self = .listedOrNone(wireListedOrNone.map { $0.map { $0.storage } }); ",
                ),
                &at_key(
                    "let wireMaybe = try container.decode(@heldMaybeValueKeyedMap@p?.self, \
                     forKey: @key); self = .maybe(wireMaybe.map { $0.storage }); ",
                ),
                &at_key(
                    "let wireMaybeListed = try \
                     container.decode([@heldMaybeListedValueKeyedMap@p]?.self, forKey: @key); \
                     self = .maybeListed(wireMaybeListed.map { $0.map { $0.storage } }); ",
                ),
                "let wireSlot0 = try container.decode([@heldPairedTupleSlot0KeyedMap@p].self); \
                 slot0 = wireSlot0.map { $0.storage }; \
                 let wireSlot1 = try container.decode(@heldPairedTupleSlot1KeyedMap@p?.self); \
                 slot1 = wireSlot1.map { $0.storage }; ",
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
                "let wireListed = try [@heldListedValueKeyedMap@p](from: decoder); \
                 return wireListed.map { $0.storage } }",
                "let wireListedOrNone = try [@heldListedOrNoneValueKeyedMap@p?](from: decoder); \
                 return wireListedOrNone.map { $0.map { $0.storage } } }",
                "let wireMaybe = try @heldMaybeValueKeyedMap@p?(from: decoder); \
                 return wireMaybe.map { $0.storage } }",
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
            "self.value = try { let wireValue = try \
             container.decode([StowedCupboardKeyedMap<Stowed>].self); \
             return wireValue.map { $0.storage } }() }; ",
        ),
        (
            stowed_sideboard_swift::swift_definition(),
            "self.value = try { let wireValue = try \
             container.decode([StowedSideboardKeyedMap<Stowed>?].self); \
             return wireValue.map { $0.map { $0.storage } } }() }; ",
        ),
        (
            stowed_dresser_swift::swift_definition(),
            "self.value = try { let wireValue = try \
             container.decode([StowedDresserKeyedMap<Stowed>]?.self); \
             return wireValue.map { $0.map { $0.storage } } }() }; ",
        ),
        (
            stowed_caddy_swift::swift_definition(),
            "self.value = try { let wireValue = try \
             container.decode(StowedCaddyKeyedMap<Stowed>?.self); \
             return wireValue.map { $0.storage } }() }; ",
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
                "let wireSlot0 = try \
                 container.decode([@heldFlagListsOrNoneTupleSlot0KeyedMap@p?].self); \
                 slot0 = wireSlot0.map { $0.map { $0.storage } }; ",
                "let wireSlot0 = try \
                 container.decode(@heldMaybeNumbersTupleSlot0KeyedMap@p?.self); \
                 slot0 = wireSlot0.map { $0.storage }; ",
                "let wireSlot0 = try \
                 container.decode([@heldMaybeRungListsTupleSlot0KeyedMap@p]?.self); \
                 slot0 = wireSlot0.map { $0.map { $0.storage } }; ",
                "let wireSlot0 = try \
                 container.decode([@heldNumberListsTupleSlot0KeyedMap@p].self); \
                 slot0 = wireSlot0.map { $0.storage }; ",
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
                "let wireSlot0 = try container.decode([@heldTupleSlot0KeyedMap@p].self); \
               slot0 = wireSlot0.map { $0.storage }; \
               let wireSlot1 = try container.decode([@heldTupleSlot1KeyedMap@p?].self); \
               slot1 = wireSlot1.map { $0.map { $0.storage } }; \
               let wireSlot2 = try container.decode([@heldTupleSlot2KeyedMap@p]?.self); \
               slot2 = wireSlot2.map { $0.map { $0.storage } }; \
               let wireSlot3 = try container.decode(@heldTupleSlot3KeyedMap@p?.self); \
               slot3 = wireSlot3.map { $0.storage }; ",
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
                "self.maybeNumberListsByName = \
                 wireMaybeNumberListsByName.mapValues { $0.map { $0.map { $0.storage } } }; ",
                "let wireValue = try \
                 container.decode(@heldMaybeRungsByRungValueKeyedMap@p?.self, \
                 forKey: codingKey); result[key] = wireValue.map { $0.storage } }; ",
                "self.numberListsByName = \
                 wireNumberListsByName.mapValues { $0.map { $0.storage } }; ",
                "self.rungListsOrNoneByName = \
                 wireRungListsOrNoneByName.mapValues { $0.map { $0.map { $0.storage } } }",
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
        read_back(&Timeline {
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
