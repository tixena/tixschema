//! `validate()` runs the `validate()` of a model held as the value of a map, in a slot and in a
//! tuple, and names it by the key serde writes for the entry, or by its position.

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
mod a_model_held_in_a_map_a_slot_or_a_tuple {
    use alloc::collections::BTreeMap;
    use alloc::sync::Arc;
    use core::fmt;
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use tixschema::model_schema;

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Parcel {
        #[model_schema_prop(minLength = 1)]
        pub item_label: String,
    }

    /// A plain enum serde writes in lower case, and prints under another name.
    #[model_schema()]
    #[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
    #[serde(rename_all = "lowercase")]
    pub enum Dock {
        North,
        South,
    }

    impl fmt::Display for Dock {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(match *self {
                Self::North => "the north dock",
                Self::South => "the south dock",
            })
        }
    }

    /// A brand over a string.
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
    #[serde(transparent)]
    pub struct Bay(pub String);

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Terminal {
        pub first_parcel: Parcel,
        pub parcels_by_dock: BTreeMap<String, Parcel>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Yard {
        pub by_bay: HashMap<String, Parcel>,
        pub by_brand: BTreeMap<Bay, Parcel>,
        pub by_dock: BTreeMap<Dock, Parcel>,
        pub by_flag: BTreeMap<bool, Parcel>,
        pub by_letter: BTreeMap<char, Parcel>,
        pub by_number: BTreeMap<u32, Parcel>,
        pub docks_in_order: Vec<BTreeMap<String, Parcel>>,
        pub gate_parcel: Parcel,
        pub lists_by_dock: BTreeMap<String, Vec<Parcel>>,
        pub rows_by_dock: BTreeMap<String, BTreeMap<u32, Parcel>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub spares_by_dock: Option<Arc<BTreeMap<String, Parcel>>>,
    }

    /// A struct whose one field is a map of models, and the struct that holds it.
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Racks {
        pub by_aisle: BTreeMap<String, Parcel>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Depot {
        pub depot_racks: Racks,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Consignment(#[model_schema_prop(minLength = 1)] pub String, pub Parcel);

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Pallet(
        #[model_schema_prop(minLength = 1)] pub String,
        pub Option<Parcel>,
        pub Vec<Parcel>,
        pub BTreeMap<String, Parcel>,
    );

    /// A struct of one slot, which serde writes as the slot's value, and the struct that holds it.
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Sealed(pub Parcel);

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Crane {
        pub sealed_load: Sealed,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(rename_all_fields = "camelCase")]
    pub enum Freight {
        Boxed(Parcel),
        Loose {
            #[model_schema_prop(minLength = 1)]
            freight_label: String,
        },
        Split(#[model_schema_prop(minLength = 1)] String, Parcel),
        Stacked(Vec<Parcel>),
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(tag = "kind", content = "held", rename_all_fields = "camelCase")]
    pub enum Haul {
        Boxed(Parcel),
        Loose {
            #[model_schema_prop(minLength = 1)]
            haul_label: String,
        },
        Split(String, Parcel),
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(tag = "kind", rename_all_fields = "camelCase")]
    pub enum Lift {
        Boxed(Parcel),
        Loose {
            #[model_schema_prop(minLength = 1)]
            lift_label: String,
        },
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Barge {
        pub deck_pair: (String, Parcel),
        pub deck_pairs: Vec<(Parcel, Parcel)>,
        pub hold_parcel: Parcel,
        pub stern_pair: (u32, (String, Option<Parcel>)),
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(untagged)]
    pub enum Stock {
        Keyed(BTreeMap<String, Parcel>),
        Listed(Vec<Parcel>),
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Annex {
        pub annex_parcel: Parcel,
        #[serde(flatten)]
        pub parcels_by_gate: BTreeMap<String, Parcel>,
    }

    /// A key typed with a parameter that declares no `Serialize`, beside a borrowed key. The
    /// lifetime and the parameter are named as the walk names its own.
    #[model_schema(default_types(K = String))]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Shelf<'held, K: Ord> {
        pub by_key: BTreeMap<K, Parcel>,
        #[serde(borrow)]
        pub by_name: BTreeMap<&'held str, Parcel>,
        pub top_parcel: Parcel,
    }

    /// A key typed with a parameter that declares `Serialize`, which the derive is told not to
    /// write a second time.
    #[model_schema(default_types(Key = String))]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(
        rename_all = "camelCase",
        bound(serialize = "", deserialize = "Key: Deserialize<'de>")
    )]
    pub struct Rack<Key: Ord + Serialize> {
        pub by_key: BTreeMap<Key, Parcel>,
        pub rack_parcel: Parcel,
    }

    #[cfg(feature = "indexmap")]
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Quay {
        pub in_order: indexmap::IndexMap<String, Parcel>,
        pub quay_parcel: Parcel,
    }

    #[cfg(feature = "chrono")]
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Schedule {
        pub by_arrival: BTreeMap<chrono::NaiveDateTime, Parcel>,
        pub schedule_parcel: Parcel,
    }

    fn sound() -> Parcel {
        Parcel {
            item_label: "A".to_owned(),
        }
    }

    fn unlabelled() -> Parcel {
        Parcel {
            item_label: String::new(),
        }
    }

    /// The message of an unlabelled parcel held at `path`.
    fn unlabelled_at(path: &str) -> String {
        format!("'{path}': too short: minimum length is 1, got 0")
    }

    /// One unlabelled parcel under the key `north`.
    fn at_north() -> BTreeMap<String, Parcel> {
        BTreeMap::from([("north".to_owned(), unlabelled())])
    }

    /// A yard with one unlabelled parcel under a key of each kind that is no string.
    fn keyed_yard() -> Yard {
        Yard {
            by_brand: BTreeMap::from([(Bay("b2".to_owned()), unlabelled())]),
            by_dock: BTreeMap::from([(Dock::North, unlabelled()), (Dock::South, sound())]),
            by_flag: BTreeMap::from([(true, unlabelled())]),
            by_letter: BTreeMap::from([('q', unlabelled())]),
            by_number: BTreeMap::from([(7, unlabelled())]),
            ..empty_yard()
        }
    }

    /// A yard with one unlabelled parcel in a map of lists and in a list of maps.
    fn nested_yard() -> Yard {
        Yard {
            docks_in_order: vec![BTreeMap::new(), at_north()],
            lists_by_dock: BTreeMap::from([("north".to_owned(), vec![sound(), unlabelled()])]),
            ..empty_yard()
        }
    }

    fn empty_yard() -> Yard {
        Yard {
            by_bay: HashMap::new(),
            by_brand: BTreeMap::new(),
            by_dock: BTreeMap::new(),
            by_flag: BTreeMap::new(),
            by_letter: BTreeMap::new(),
            by_number: BTreeMap::new(),
            docks_in_order: Vec::new(),
            gate_parcel: sound(),
            lists_by_dock: BTreeMap::new(),
            rows_by_dock: BTreeMap::new(),
            spares_by_dock: None,
        }
    }

    #[test]
    fn a_model_held_as_a_maps_value_is_reported_under_its_key() {
        let terminal = Terminal {
            first_parcel: unlabelled(),
            parcels_by_dock: BTreeMap::from([
                ("north".to_owned(), unlabelled()),
                ("south".to_owned(), sound()),
                ("west".to_owned(), unlabelled()),
            ]),
        };
        assert_eq!(
            terminal.validate(),
            Err(vec![
                unlabelled_at("firstParcel.itemLabel"),
                unlabelled_at("parcelsByDock.north.itemLabel"),
                unlabelled_at("parcelsByDock.west.itemLabel"),
            ])
        );
    }

    #[test]
    fn a_hash_map_is_walked_as_a_b_tree_map_is() {
        let yard = Yard {
            by_bay: HashMap::from([("b2".to_owned(), unlabelled())]),
            ..empty_yard()
        };
        assert_eq!(
            yard.validate(),
            Err(vec![unlabelled_at("byBay.b2.itemLabel")])
        );
    }

    #[cfg(feature = "indexmap")]
    #[test]
    fn an_index_map_is_walked_as_a_b_tree_map_is() {
        let quay = Quay {
            in_order: indexmap::IndexMap::from([
                ("west".to_owned(), unlabelled()),
                ("north".to_owned(), unlabelled()),
            ]),
            quay_parcel: sound(),
        };
        assert_eq!(
            quay.validate(),
            Err(vec![
                unlabelled_at("inOrder.west.itemLabel"),
                unlabelled_at("inOrder.north.itemLabel"),
            ])
        );
    }

    #[test]
    fn an_empty_map_and_a_map_of_sound_models_report_nothing() {
        assert_eq!(empty_yard().validate(), Ok(()));
        let terminal = Terminal {
            first_parcel: sound(),
            parcels_by_dock: BTreeMap::from([("north".to_owned(), sound())]),
        };
        assert_eq!(terminal.validate(), Ok(()));
    }

    #[test]
    fn a_key_and_an_index_are_written_in_the_order_the_payload_nests_them() {
        let yard = Yard {
            rows_by_dock: BTreeMap::from([(
                "north".to_owned(),
                BTreeMap::from([(4, unlabelled())]),
            )]),
            spares_by_dock: Some(Arc::new(at_north())),
            ..nested_yard()
        };
        assert_eq!(
            yard.validate(),
            Err(vec![
                unlabelled_at("docksInOrder.1.north.itemLabel"),
                unlabelled_at("listsByDock.north.1.itemLabel"),
                unlabelled_at("rowsByDock.north.4.itemLabel"),
                unlabelled_at("sparesByDock.north.itemLabel"),
            ])
        );
    }

    #[test]
    fn a_key_that_is_no_string_is_written_as_serde_writes_it() {
        let yard = keyed_yard();
        assert_eq!(
            yard.validate(),
            Err(vec![
                unlabelled_at("byBrand.b2.itemLabel"),
                unlabelled_at("byDock.north.itemLabel"),
                unlabelled_at("byFlag.true.itemLabel"),
                unlabelled_at("byLetter.q.itemLabel"),
                unlabelled_at("byNumber.7.itemLabel"),
            ])
        );
        assert_eq!(Dock::North.to_string(), "the north dock");
        let written = serde_json::to_value(&yard).unwrap();
        for (field, key) in [
            ("byBrand", "b2"),
            ("byDock", "north"),
            ("byFlag", "true"),
            ("byLetter", "q"),
            ("byNumber", "7"),
        ] {
            assert_eq!(
                written[field][key]["itemLabel"], "",
                "serde writes no `{key}` under `{field}`"
            );
        }
    }

    /// A date and time prints with a space where serde writes a `T`.
    #[cfg(feature = "chrono")]
    #[test]
    fn a_key_is_the_text_serde_writes_where_the_key_prints_another() {
        let arrival = chrono::NaiveDate::from_ymd_opt(2026, 10, 13)
            .unwrap()
            .and_hms_opt(4, 17, 0)
            .unwrap();
        assert_eq!(arrival.to_string(), "2026-10-13 04:17:00");
        let schedule = Schedule {
            by_arrival: BTreeMap::from([(arrival, unlabelled())]),
            schedule_parcel: sound(),
        };
        assert_eq!(
            schedule.validate(),
            Err(vec![unlabelled_at(
                "byArrival.2026-10-13T04:17:00.itemLabel"
            )])
        );
        assert_eq!(
            serde_json::to_value(&schedule).unwrap()["byArrival"]["2026-10-13T04:17:00"]["itemLabel"],
            ""
        );
        assert!(
            include_str!("../../README.md")
                .contains("`2026-10-13T04:17:00`, where the type prints `2026-10-13 04:17:00`"),
            "the README no longer shows the key of a date and time"
        );
    }

    #[test]
    fn a_struct_whose_one_field_is_a_map_of_models_reports_them() {
        let depot = Depot {
            depot_racks: Racks {
                by_aisle: at_north(),
            },
        };
        assert_eq!(
            depot.validate(),
            Err(vec![unlabelled_at("depotRacks.byAisle.north.itemLabel")])
        );
    }

    #[test]
    fn a_model_held_in_a_slot_of_a_tuple_struct_is_reported_under_its_position() {
        assert_eq!(
            Consignment(String::new(), unlabelled()).validate(),
            Err(vec![unlabelled_at("0"), unlabelled_at("1.itemLabel")])
        );
        assert_eq!(Consignment("a".to_owned(), sound()).validate(), Ok(()));
    }

    #[test]
    fn a_slot_is_walked_through_an_option_a_list_and_a_map() {
        let pallet = Pallet(
            "a".to_owned(),
            Some(unlabelled()),
            vec![sound(), unlabelled()],
            at_north(),
        );
        assert_eq!(
            pallet.validate(),
            Err(vec![
                unlabelled_at("1.itemLabel"),
                unlabelled_at("2.1.itemLabel"),
                unlabelled_at("3.north.itemLabel"),
            ])
        );
        assert_eq!(
            Pallet("a".to_owned(), None, Vec::new(), BTreeMap::new()).validate(),
            Ok(())
        );
    }

    #[test]
    fn the_one_slot_of_a_struct_is_reported_under_its_position() {
        let crane = Crane {
            sealed_load: Sealed(unlabelled()),
        };
        assert_eq!(
            crane.validate(),
            Err(vec![unlabelled_at("sealedLoad.0.itemLabel")])
        );
    }

    #[test]
    fn a_model_held_in_the_one_slot_of_a_tagged_variant_is_reported_under_its_position() {
        assert_eq!(
            [
                Freight::Boxed(unlabelled()).validate(),
                Freight::Stacked(vec![sound(), unlabelled()]).validate(),
                Freight::Boxed(sound()).validate(),
            ],
            [
                Err(vec![unlabelled_at("0.itemLabel")]),
                Err(vec![unlabelled_at("0.1.itemLabel")]),
                Ok(()),
            ]
        );
    }

    #[test]
    fn a_model_held_in_a_tagged_variant_of_several_slots_is_reported_under_its_position() {
        assert_eq!(
            Freight::Split(String::new(), unlabelled()).validate(),
            Err(vec![unlabelled_at("0"), unlabelled_at("1.itemLabel")])
        );
        let loose = Freight::Loose {
            freight_label: String::new(),
        };
        assert_eq!(loose.validate(), Err(vec![unlabelled_at("freightLabel")]));
    }

    #[test]
    fn a_slot_is_reported_under_its_position_under_a_content_key_and_beside_a_tag() {
        let loose = Haul::Loose {
            haul_label: "a".to_owned(),
        };
        let lifted = Lift::Loose {
            lift_label: "a".to_owned(),
        };
        assert_eq!(
            [
                Haul::Boxed(unlabelled()).validate(),
                Haul::Split("a".to_owned(), unlabelled()).validate(),
                Lift::Boxed(unlabelled()).validate(),
                loose.validate(),
                lifted.validate(),
            ],
            [
                Err(vec![unlabelled_at("0.itemLabel")]),
                Err(vec![unlabelled_at("1.itemLabel")]),
                Err(vec![unlabelled_at("0.itemLabel")]),
                Ok(()),
                Ok(()),
            ]
        );
    }

    #[test]
    fn a_model_held_in_a_tuple_written_as_a_fields_type_is_reported_under_its_position() {
        let barge = Barge {
            deck_pair: ("a".to_owned(), unlabelled()),
            deck_pairs: vec![(sound(), sound()), (unlabelled(), unlabelled())],
            hold_parcel: sound(),
            stern_pair: (1, ("a".to_owned(), Some(unlabelled()))),
        };
        assert_eq!(
            barge.validate(),
            Err(vec![
                unlabelled_at("deckPair.1.itemLabel"),
                unlabelled_at("deckPairs.1.0.itemLabel"),
                unlabelled_at("deckPairs.1.1.itemLabel"),
                unlabelled_at("sternPair.1.1.itemLabel"),
            ])
        );
    }

    #[test]
    fn a_map_written_as_the_whole_value_names_its_entry_by_the_key_alone() {
        let annex = Annex {
            annex_parcel: sound(),
            parcels_by_gate: at_north(),
        };
        assert_eq!(
            [Stock::Keyed(at_north()).validate(), annex.validate()],
            [
                Err(vec![unlabelled_at("north.itemLabel")]),
                Err(vec![unlabelled_at("north.itemLabel")]),
            ]
        );
        assert_eq!(
            serde_json::to_value(&annex).unwrap()["north"]["itemLabel"],
            ""
        );
    }

    #[test]
    fn a_key_typed_with_a_parameter_is_written_where_the_parameter_declares_serialize() {
        let rack = Rack {
            by_key: at_north(),
            rack_parcel: sound(),
        };
        let shelf = Shelf {
            by_key: at_north(),
            by_name: BTreeMap::from([("west", unlabelled())]),
            top_parcel: sound(),
        };
        assert_eq!(
            [rack.validate(), shelf.validate()],
            [
                Err(vec![unlabelled_at("byKey.north.itemLabel")]),
                Err(vec![
                    unlabelled_at("byKey.itemLabel"),
                    unlabelled_at("byName.west.itemLabel"),
                ]),
            ],
            "`validate()` is written under the bounds the type declares, and names no key it \
             cannot ask serde to write"
        );
    }

    /// Each message the README shows for a map, for a slot and for a tuple is one a run answers.
    #[test]
    fn the_readme_shows_what_validate_reports_for_a_map_a_slot_and_a_tuple() {
        let readme = include_str!("../../README.md");
        let terminal = Terminal {
            first_parcel: sound(),
            parcels_by_dock: at_north(),
        };
        let shown = [
            terminal.validate(),
            Consignment(String::new(), unlabelled()).validate(),
        ];
        assert_eq!(
            shown,
            [
                Err(vec![unlabelled_at("parcelsByDock.north.itemLabel")]),
                Err(vec![unlabelled_at("0"), unlabelled_at("1.itemLabel")]),
            ]
        );
        for reported in &shown {
            let written = format!("// {reported:?}");
            assert!(
                readme.contains(&written),
                "the README no longer shows {written}"
            );
        }
        let whole = unlabelled_at("parcelsByDock.north.itemLabel");
        assert!(
            readme.contains(&format!("`{whole}`")),
            "the README no longer shows `{whole}`"
        );

        let barge = Barge {
            deck_pair: ("a".to_owned(), unlabelled()),
            deck_pairs: Vec::new(),
            hold_parcel: sound(),
            stern_pair: (1, ("a".to_owned(), None)),
        };
        let rack = Rack {
            by_key: at_north(),
            rack_parcel: sound(),
        };
        let shelf = Shelf {
            by_key: at_north(),
            by_name: BTreeMap::new(),
            top_parcel: sound(),
        };
        let named: Vec<String> = [
            keyed_yard().validate(),
            nested_yard().validate(),
            barge.validate(),
            Stock::Keyed(at_north()).validate(),
            rack.validate(),
            shelf.validate(),
        ]
        .into_iter()
        .flat_map(Result::unwrap_err)
        .map(|reported| reported.replace(": too short: minimum length is 1, got 0", ""))
        .collect();
        assert_eq!(
            named,
            [
                "'byBrand.b2.itemLabel'",
                "'byDock.north.itemLabel'",
                "'byFlag.true.itemLabel'",
                "'byLetter.q.itemLabel'",
                "'byNumber.7.itemLabel'",
                "'docksInOrder.1.north.itemLabel'",
                "'listsByDock.north.1.itemLabel'",
                "'deckPair.1.itemLabel'",
                "'north.itemLabel'",
                "'byKey.north.itemLabel'",
                "'byKey.itemLabel'",
            ]
        );
        // The README names a brand's key and a `char`'s in words, and every other path as it is.
        for path in named
            .iter()
            .filter(|path| !path.starts_with("'byBrand") && !path.starts_with("'byLetter"))
        {
            assert!(
                readme.contains(&format!("`{path}")),
                "the README no longer shows {path}"
            );
        }
    }
}
