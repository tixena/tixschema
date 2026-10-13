//! A message of `validate()` names a value by its path in the payload: the key serde writes for
//! each member, and the index of each item of a list.

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
mod a_message_names_the_path_in_the_payload {
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    /// What `validate()` answers for [`broken_manifest`], which the README prints as it is here.
    const MANIFEST_REPORT: &str = "\
'firstParcel.itemLabel': too short: minimum length is 1, got 0
'firstParcel.codes': too short: minimum length is 1, got 0
'firstParcel.parcelCode': too short: minimum length is 2, got 1
'parcelList.0.itemLabel': too short: minimum length is 1, got 0
'parcelList.0.codes': too short: minimum length is 1, got 0
'parcelList.0.parcelCode': too short: minimum length is 2, got 1
'parcelList.1.itemLabel': too short: minimum length is 1, got 0
'parcelList.1.codes': too short: minimum length is 1, got 0
'parcelList.1.parcelCode': too short: minimum length is 2, got 1
'spareParcel.itemLabel': too short: minimum length is 1, got 0
'spareParcel.codes': too short: minimum length is 1, got 0
'spareParcel.parcelCode': too short: minimum length is 2, got 1
'baseName': too short: minimum length is 1, got 0";

    #[model_schema(minLength = 2)]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(transparent)]
    pub struct Code(pub String);

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Package {
        #[model_schema_prop(minLength = 1)]
        pub item_label: String,
        #[serde(rename = "codes")]
        #[model_schema_prop(minLength = 1)]
        pub listed_item_codes: Vec<String>,
        pub parcel_code: Code,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Base {
        #[model_schema_prop(minLength = 1)]
        pub base_name: String,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Manifest {
        pub first_parcel: Package,
        pub parcel_list: Vec<Package>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub spare_parcel: Option<Package>,
        #[serde(flatten)]
        pub starting_base: Base,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(
        tag = "kind",
        rename_all = "camelCase",
        rename_all_fields = "camelCase"
    )]
    pub enum Dispatch {
        ByAir {
            #[model_schema_prop(minLength = 1)]
            flight_number: String,
        },
        ByRoad {
            road_parcel: Package,
        },
    }

    /// A variant under a casing of its own, with one field renamed outright.
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(tag = "kind", rename_all_fields = "camelCase")]
    pub enum Vessel {
        #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
        Docked {
            #[serde(rename = "berth")]
            #[model_schema_prop(minimum = 1)]
            berth_number: u32,
            #[model_schema_prop(minLength = 1)]
            vessel_name: String,
        },
        Sailing {
            #[model_schema_prop(minLength = 1)]
            vessel_name: String,
        },
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Hold {
        pub code_list: Vec<Code>,
        pub grid: Vec<Vec<Package>>,
        #[model_schema_prop(minLength = 1)]
        pub parcel_list: Vec<Package>,
        pub parcel_pair: [Package; 2],
        #[serde(skip_serializing_if = "Option::is_none")]
        pub spare_list: Option<Vec<Package>>,
        pub spares: Vec<Option<Package>>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Customs {
        pub r#box: Package,
        #[model_schema_prop(minLength = 1)]
        pub r#type: String,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Docket(
        #[model_schema_prop(minLength = 1)] pub String,
        #[model_schema_prop(minimum = 1)] pub u32,
    );

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(rename_all_fields = "camelCase")]
    pub enum Seal {
        Signed {
            #[model_schema_prop(minLength = 1)]
            signed_by: String,
        },
        Stamped(#[model_schema_prop(minLength = 1)] String),
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(untagged, rename_all_fields = "camelCase")]
    pub enum Load {
        Labelled {
            #[model_schema_prop(minLength = 1)]
            load_label: String,
            load_parcels: Vec<Package>,
        },
        Loose(Vec<Package>),
        Text(#[model_schema_prop(minLength = 1)] String),
        Whole(Package),
    }

    fn sound() -> Package {
        Package {
            item_label: "A".to_owned(),
            listed_item_codes: vec!["x".to_owned()],
            parcel_code: Code("ab".to_owned()),
        }
    }

    /// A package that breaks one bound, so its report is one line.
    fn unlabelled() -> Package {
        Package {
            item_label: String::new(),
            ..sound()
        }
    }

    fn broken() -> Package {
        Package {
            item_label: String::new(),
            listed_item_codes: Vec::new(),
            parcel_code: Code("a".to_owned()),
        }
    }

    fn sound_manifest() -> Manifest {
        Manifest {
            first_parcel: sound(),
            parcel_list: Vec::new(),
            spare_parcel: None,
            starting_base: Base {
                base_name: "north".to_owned(),
            },
        }
    }

    fn broken_manifest() -> Manifest {
        Manifest {
            first_parcel: broken(),
            parcel_list: vec![broken(), broken()],
            spare_parcel: Some(broken()),
            starting_base: Base {
                base_name: String::new(),
            },
        }
    }

    fn sound_hold() -> Hold {
        Hold {
            code_list: Vec::new(),
            grid: Vec::new(),
            parcel_list: vec![sound()],
            parcel_pair: [sound(), sound()],
            spare_list: None,
            spares: Vec::new(),
        }
    }

    #[test]
    fn a_manifest_reports_each_broken_value_at_its_path_in_the_payload() {
        assert_eq!(
            broken_manifest().validate().unwrap_err().join("\n"),
            MANIFEST_REPORT
        );
        assert_eq!(sound_manifest().validate(), Ok(()));
    }

    #[test]
    fn a_fields_own_rename_wins_over_the_casing_of_its_struct() {
        assert_eq!(
            broken().validate().unwrap_err(),
            vec![
                "'itemLabel': too short: minimum length is 1, got 0",
                "'codes': too short: minimum length is 1, got 0",
                "'parcelCode': too short: minimum length is 2, got 1",
            ]
        );
    }

    #[test]
    fn a_variants_field_is_named_as_the_enum_cases_its_fields() {
        let by_air = Dispatch::ByAir {
            flight_number: String::new(),
        };
        assert_eq!(
            by_air.validate(),
            Err(vec![
                "'flightNumber': too short: minimum length is 1, got 0".to_owned()
            ])
        );
        let by_road = Dispatch::ByRoad {
            road_parcel: unlabelled(),
        };
        assert_eq!(
            by_road.validate().unwrap_err(),
            vec!["'roadParcel.itemLabel': too short: minimum length is 1, got 0"]
        );
    }

    #[test]
    fn a_variants_own_casing_wins_over_the_enums_and_a_fields_own_rename_over_both() {
        let docked = Vessel::Docked {
            berth_number: 0,
            vessel_name: String::new(),
        };
        assert_eq!(
            docked.validate().unwrap_err(),
            vec![
                "'berth': too small: minimum is 1, got 0",
                "'VESSEL_NAME': too short: minimum length is 1, got 0",
            ]
        );
        let sailing = Vessel::Sailing {
            vessel_name: String::new(),
        };
        assert_eq!(
            sailing.validate().unwrap_err(),
            vec!["'vesselName': too short: minimum length is 1, got 0"]
        );
    }

    #[test]
    fn an_item_of_a_list_is_named_by_its_index_at_each_level_of_the_list() {
        let hold = Hold {
            code_list: vec![Code("ab".to_owned()), Code("a".to_owned())],
            grid: vec![vec![sound(), unlabelled()], vec![unlabelled()]],
            parcel_pair: [sound(), unlabelled()],
            spare_list: Some(vec![unlabelled()]),
            spares: vec![None, None, Some(unlabelled())],
            ..sound_hold()
        };
        assert_eq!(
            hold.validate().unwrap_err(),
            vec![
                "'codeList.1': too short: minimum length is 2, got 1",
                "'grid.0.1.itemLabel': too short: minimum length is 1, got 0",
                "'grid.1.0.itemLabel': too short: minimum length is 1, got 0",
                "'parcelPair.1.itemLabel': too short: minimum length is 1, got 0",
                "'spareList.0.itemLabel': too short: minimum length is 1, got 0",
                "'spares.2.itemLabel': too short: minimum length is 1, got 0",
            ]
        );
        assert_eq!(sound_hold().validate(), Ok(()));
    }

    #[test]
    fn a_lists_own_count_is_reported_at_the_list_and_its_items_under_their_indexes() {
        let empty = Hold {
            parcel_list: Vec::new(),
            ..sound_hold()
        };
        assert_eq!(
            empty.validate().unwrap_err(),
            vec!["'parcelList': too short: minimum length is 1, got 0"]
        );
        let counted = Hold {
            parcel_list: vec![sound(), unlabelled()],
            ..sound_hold()
        };
        assert_eq!(
            counted.validate().unwrap_err(),
            vec!["'parcelList.1.itemLabel': too short: minimum length is 1, got 0"]
        );
    }

    #[test]
    fn a_raw_identifier_is_named_without_its_prefix() {
        let customs = Customs {
            r#box: unlabelled(),
            r#type: String::new(),
        };
        assert_eq!(
            customs.validate().unwrap_err(),
            vec![
                "'box.itemLabel': too short: minimum length is 1, got 0",
                "'type': too short: minimum length is 1, got 0",
            ]
        );
    }

    #[test]
    fn a_slot_is_named_by_its_position() {
        assert_eq!(
            Docket(String::new(), 0).validate().unwrap_err(),
            vec![
                "'0': too short: minimum length is 1, got 0",
                "'1': too small: minimum is 1, got 0",
            ]
        );
        assert_eq!(
            Seal::Stamped(String::new()).validate().unwrap_err(),
            vec!["'0': too short: minimum length is 1, got 0"]
        );
        let signed = Seal::Signed {
            signed_by: String::new(),
        };
        assert_eq!(
            signed.validate().unwrap_err(),
            vec!["'signedBy': too short: minimum length is 1, got 0"]
        );
    }

    #[test]
    fn a_flattened_member_and_an_option_add_no_segment() {
        let unnamed = Manifest {
            starting_base: Base {
                base_name: String::new(),
            },
            ..sound_manifest()
        };
        assert_eq!(
            unnamed.validate().unwrap_err(),
            vec!["'baseName': too short: minimum length is 1, got 0"]
        );
        let spare = Manifest {
            spare_parcel: Some(unlabelled()),
            ..sound_manifest()
        };
        assert_eq!(
            spare.validate().unwrap_err(),
            vec!["'spareParcel.itemLabel': too short: minimum length is 1, got 0"]
        );
    }

    #[test]
    fn the_lone_slot_of_an_untagged_variant_is_reported_under_no_name() {
        assert_eq!(
            Load::Text(String::new()).validate().unwrap_err(),
            vec!["too short: minimum length is 1, got 0"]
        );
        assert_eq!(
            Load::Whole(unlabelled()).validate().unwrap_err(),
            vec!["'itemLabel': too short: minimum length is 1, got 0"]
        );
        assert_eq!(
            Load::Loose(vec![sound(), unlabelled()])
                .validate()
                .unwrap_err(),
            vec!["'1.itemLabel': too short: minimum length is 1, got 0"],
            "a list written as the variant's whole value names its item by the index alone"
        );
    }

    #[test]
    fn a_field_of_an_untagged_variant_is_named_by_the_key_serde_writes() {
        let labelled = Load::Labelled {
            load_label: String::new(),
            load_parcels: vec![sound(), unlabelled()],
        };
        assert_eq!(
            labelled.validate().unwrap_err(),
            vec![
                "'loadLabel': too short: minimum length is 1, got 0",
                "'loadParcels.1.itemLabel': too short: minimum length is 1, got 0",
            ]
        );
    }

    /// The read of a bounded brand names the key of the payload, and `validate()` names the same
    /// one.
    #[test]
    fn the_read_of_a_bounded_brand_and_validate_name_one_key() {
        let parcel =
            serde_json::from_str::<Package>(r#"{"itemLabel":"A","codes":["x"],"parcelCode":"a"}"#);
        assert_eq!(
            parcel.unwrap_err().to_string(),
            "'parcelCode': too short: minimum length is 2, got 1"
        );
        let manifest = serde_json::from_str::<Manifest>(
            r#"{"firstParcel":{"itemLabel":"A","codes":["x"],"parcelCode":"a"},"parcelList":[],"baseName":"north"}"#,
        );
        assert_eq!(
            manifest.unwrap_err().to_string(),
            "'firstParcel.parcelCode': too short: minimum length is 2, got 1"
        );
        let held = Manifest {
            first_parcel: Package {
                parcel_code: Code("a".to_owned()),
                ..sound()
            },
            ..sound_manifest()
        };
        assert_eq!(
            held.validate().unwrap_err(),
            vec!["'firstParcel.parcelCode': too short: minimum length is 2, got 1"]
        );
    }

    /// Each message the README and the crate's rustdoc show for a path is one a run answers.
    #[test]
    fn the_documents_show_the_paths_validate_reports() {
        let readme = include_str!("../../README.md");
        assert!(
            readme.contains(&format!("```text\n{MANIFEST_REPORT}\n```")),
            "the README no longer shows the manifest's report"
        );
        let reported: Vec<String> = [
            Hold {
                grid: vec![vec![sound(), unlabelled()]],
                ..sound_hold()
            },
            Hold {
                parcel_list: Vec::new(),
                ..sound_hold()
            },
            Hold {
                code_list: vec![Code("ab".to_owned()), Code("a".to_owned())],
                ..sound_hold()
            },
        ]
        .iter()
        .flat_map(|hold| hold.validate().unwrap_err())
        .collect();
        assert_eq!(
            reported,
            [
                "'grid.0.1.itemLabel': too short: minimum length is 1, got 0",
                "'parcelList': too short: minimum length is 1, got 0",
                "'codeList.1': too short: minimum length is 2, got 1",
            ]
        );
        for shown in &reported {
            assert!(
                readme.contains(&format!("`{shown}`")),
                "the README no longer shows `{shown}`"
            );
        }

        let counted = Hold {
            parcel_list: vec![sound(), unlabelled()],
            ..sound_hold()
        };
        let in_the_rustdoc = counted.validate().unwrap_err().concat();
        assert_eq!(
            in_the_rustdoc,
            "'parcelList.1.itemLabel': too short: minimum length is 1, got 0"
        );
        assert!(
            include_str!("../../src/lib.rs").contains(&format!("`{in_the_rustdoc}`")),
            "the rustdoc no longer shows `{in_the_rustdoc}`"
        );
    }
}
