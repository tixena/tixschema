//! A serde read of a type that publishes `validate()` answers an error for a payload whose value
//! breaks a bound or a check, and the error holds what `validate()` answers of the value read.

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
mod a_read_refuses_what_validate_refuses {
    use alloc::borrow::Cow;
    use alloc::collections::BTreeMap;
    use alloc::sync::Arc;
    use core::cell::Cell;
    use core::fmt::Debug;
    use core::marker::PhantomData;
    use serde::de::{DeserializeOwned, IgnoredAny};
    use serde::{Deserialize, Deserializer, Serialize};
    use serde_json::json;
    use tixschema::model_schema;

    /// What a manifest with a broken parcel in every place it holds one is refused with.
    const MANIFEST_REFUSAL: &str = "\
'boxedParcel.itemLabel': too short: minimum length is 1, got 0; \
'boxedParcel.fragileItemCodes.1': y is outside of itemCodeList; \
'firstParcel.itemLabel': too short: minimum length is 1, got 0; \
'firstParcel.fragileItemCodes.1': y is outside of itemCodeList; \
'parcelList.1.itemLabel': too short: minimum length is 1, got 0; \
'parcelList.1.fragileItemCodes.1': y is outside of itemCodeList; \
'parcelsByDock.north.itemLabel': too short: minimum length is 1, got 0; \
'parcelsByDock.north.fragileItemCodes.1': y is outside of itemCodeList; \
'spareParcel.itemLabel': too short: minimum length is 1, got 0; \
'spareParcel.fragileItemCodes.1': y is outside of itemCodeList";

    thread_local! {
        /// Whether the dock of the test that is running takes parcels.
        static DOCK_IS_OPEN: Cell<bool> = const { Cell::new(false) };
    }

    /// What a parcel with no label and a fragile code of no item is refused with.
    const PARCEL_REFUSAL: &str = "'itemLabel': too short: minimum length is 1, got 0; \
'fragileItemCodes.1': y is outside of itemCodeList";

    #[model_schema(check = ["withinParcel"])]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Parcel {
        pub fragile_item_codes: Vec<String>,
        #[model_schema_prop(minLength = 1)]
        pub item_label: String,
    }

    /// A model with no rule of its own, which publishes no `validate()`.
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Note {
        pub text: String,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Manifest {
        pub boxed_parcel: Box<Parcel>,
        pub first_parcel: Parcel,
        pub note: Note,
        pub parcel_list: Vec<Parcel>,
        pub parcels_by_dock: BTreeMap<String, Parcel>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub spare_parcel: Option<Parcel>,
    }

    /// A holder of the author's own with no `model_schema`.
    #[derive(Debug, Deserialize)]
    pub struct Plain {
        pub parcels: Vec<Parcel>,
    }

    /// Holds a parcel in each of three places its own read reads through no hook.
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Yard {
        pub arced_parcel: Arc<Parcel>,
        pub fixed_parcels: [Parcel; 1],
        pub paired_parcel: (Parcel, u32),
    }

    #[model_schema(minLength = 2)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct Code(pub String);

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Crane {
        pub load_code: Code,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Consignment(#[model_schema_prop(minLength = 1)] pub String, pub Parcel);

    /// A struct of one slot, which serde writes as the slot's value.
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Sealed(pub Parcel);

    /// A unit struct, which has one value: its check answers by whether the dock is open.
    #[model_schema(check = ["takingParcels"])]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Dock;

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "kind", rename_all_fields = "camelCase")]
    pub enum Lift {
        Boxed(Parcel),
        Loose {
            #[model_schema_prop(minLength = 1)]
            lift_label: String,
        },
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(untagged)]
    pub enum Stock {
        Keyed(BTreeMap<String, Parcel>),
        Listed(Vec<Parcel>),
        Single(Parcel),
    }

    /// A bounded brand held by a named member of a variant, under each form serde writes a
    /// variant's name or a content key ahead of the member.
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(rename_all_fields = "camelCase")]
    pub enum Sorted {
        Coded { parcel_code: Code },
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "kind", content = "held", rename_all_fields = "camelCase")]
    pub enum SortedUnderAKey {
        Coded { parcel_code: Code },
    }

    /// An untagged enum whose first variant is chosen by its member's bound.
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(untagged)]
    pub enum Reading {
        Short(#[model_schema_prop(maxLength = 3)] String),
        Text(String),
    }

    /// An enum no variant of which holds a value.
    #[model_schema(check = ["rated"])]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub enum Grade {
        Express,
        Unrated,
    }

    #[model_schema(default_types(CodeType = String), check = ["withACode"])]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Crate<CodeType> {
        pub codes: Vec<CodeType>,
        #[model_schema_prop(minLength = 1)]
        pub label: String,
    }

    /// A filling serde neither reads nor builds a default of.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct Unread;

    /// A filling serde builds a default of and never reads.
    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    pub struct Unwritten;

    /// A parameter a marker alone names.
    #[model_schema(default_types(Marker = String))]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Tagged<Marker> {
        #[model_schema_prop(minLength = 1)]
        pub label: String,
        #[serde(skip)]
        pub marker: PhantomData<Marker>,
    }

    /// A parameter a skipped field alone names.
    #[model_schema(default_types(Scratch = String))]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Cached<Scratch> {
        #[model_schema_prop(minLength = 1)]
        pub label: String,
        #[serde(skip)]
        pub scratch: Vec<Scratch>,
    }

    /// A bound of the container's own, in place of the one serde's derive writes.
    #[model_schema(default_types(Key = String))]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(bound(serialize = "", deserialize = "Key: Deserialize<'de>"))]
    pub struct Keyed<Key: Ord + Serialize> {
        pub by_key: BTreeMap<Key, u32>,
        #[model_schema_prop(minLength = 1)]
        pub label: String,
    }

    /// A parameter named by a field its author's own function reads.
    #[model_schema(default_types(Stamp = String))]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Stamped<Stamp> {
        #[model_schema_prop(minLength = 1)]
        pub label: String,
        #[serde(deserialize_with = "unstamped")]
        pub stamps: Vec<Stamp>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Borrowed<'text> {
        #[model_schema_prop(minLength = 1)]
        pub label: &'text str,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Lent<'text> {
        #[model_schema_prop(minLength = 1)]
        #[serde(borrow)]
        pub label: Cow<'text, str>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Owned<'text> {
        #[model_schema_prop(minLength = 1)]
        pub label: Cow<'text, str>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Fixed {
        #[model_schema_prop(minLength = 1)]
        pub label: &'static str,
    }

    fn within_parcel(value: &Parcel) -> Result<(), Vec<String>> {
        let outside: Vec<String> = value
            .fragile_item_codes
            .iter()
            .enumerate()
            .filter(|(_, code)| code.as_str() != "x")
            .map(|(index, code)| {
                format!("'fragileItemCodes.{index}': {code} is outside of itemCodeList")
            })
            .collect();
        if outside.is_empty() {
            Ok(())
        } else {
            Err(outside)
        }
    }

    fn taking_parcels(_value: &Dock) -> Result<(), Vec<String>> {
        if DOCK_IS_OPEN.with(Cell::get) {
            return Ok(());
        }
        Err(vec!["a closed dock takes no parcel".to_owned()])
    }

    fn rated(value: &Grade) -> Result<(), Vec<String>> {
        if matches!(value, Grade::Unrated) {
            return Err(vec!["a parcel is rated".to_owned()]);
        }
        Ok(())
    }

    fn with_a_code<CodeType>(value: &Crate<CodeType>) -> Result<(), Vec<String>> {
        if value.codes.is_empty() {
            return Err(vec!["'codes': a crate holds at least one code".to_owned()]);
        }
        Ok(())
    }

    /// Reads a stamp list as empty, whatever it holds.
    fn unstamped<'de, D, Stamp>(deserializer: D) -> Result<Vec<Stamp>, D::Error>
    where
        D: Deserializer<'de>,
    {
        IgnoredAny::deserialize(deserializer).map(|_| Vec::new())
    }

    fn sound() -> Parcel {
        Parcel {
            fragile_item_codes: vec!["x".to_owned()],
            item_label: "A".to_owned(),
        }
    }

    fn broken() -> Parcel {
        Parcel {
            fragile_item_codes: vec!["x".to_owned(), "y".to_owned()],
            item_label: String::new(),
        }
    }

    /// A manifest that holds `held` in every place it holds a parcel, after a sound one in its
    /// list.
    fn manifest_of(held: &Parcel) -> Manifest {
        Manifest {
            boxed_parcel: Box::new(held.clone()),
            first_parcel: held.clone(),
            note: Note {
                text: "n".to_owned(),
            },
            parcel_list: vec![sound(), held.clone()],
            parcels_by_dock: BTreeMap::from([("north".to_owned(), held.clone())]),
            spare_parcel: Some(held.clone()),
        }
    }

    /// The text a read is refused with for a value `validate()` answers `reported` of.
    fn refusal(reported: &[String]) -> String {
        reported.join("; ")
    }

    /// Holds the read of what serde writes for `value` to what `validate()` answers of `value`:
    /// an error with every message where it reports any, and the value where it reports none.
    fn read_as_validated<T>(value: &T, validated: Result<(), Vec<String>>)
    where
        T: Debug + DeserializeOwned + PartialEq + Serialize,
    {
        let payload = serde_json::to_string(value).unwrap();
        let read = serde_json::from_str::<T>(&payload);
        let from_value = serde_json::from_value::<T>(serde_json::to_value(value).unwrap());
        match validated {
            Ok(()) => {
                assert_eq!(read.as_ref().ok(), Some(value), "{payload} is not read");
                assert_eq!(
                    from_value.ok().as_ref(),
                    Some(value),
                    "{payload} is not read"
                );
                assert_eq!(serde_json::to_string(&read.unwrap()).unwrap(), payload);
            }
            Err(reported) => {
                let refused = refusal(&reported);
                assert_eq!(
                    read.map_err(|error| error.to_string()),
                    Err(refused.clone()),
                    "{payload} is read"
                );
                assert_eq!(
                    from_value.map_err(|error| error.to_string()),
                    Err(refused),
                    "{payload} is read from a value"
                );
            }
        }
    }

    #[test]
    fn a_payload_that_breaks_a_bound_is_refused_with_what_validate_answers() {
        let unlabelled = Parcel {
            fragile_item_codes: Vec::new(),
            item_label: String::new(),
        };
        let refused = serde_json::from_str::<Parcel>(r#"{"itemLabel":"","fragileItemCodes":[]}"#)
            .unwrap_err();
        assert_eq!(
            refused.to_string(),
            "'itemLabel': too short: minimum length is 1, got 0"
        );
        assert_eq!(
            refused.to_string(),
            refusal(&unlabelled.validate().unwrap_err())
        );
    }

    #[test]
    fn a_payload_that_breaks_a_check_is_refused_with_what_validate_answers() {
        let outside = Parcel {
            fragile_item_codes: vec!["x".to_owned(), "y".to_owned()],
            item_label: "A".to_owned(),
        };
        let refused =
            serde_json::from_str::<Parcel>(r#"{"itemLabel":"A","fragileItemCodes":["x","y"]}"#)
                .unwrap_err();
        assert_eq!(
            refused.to_string(),
            "'fragileItemCodes.1': y is outside of itemCodeList"
        );
        assert_eq!(
            refused.to_string(),
            refusal(&outside.validate().unwrap_err())
        );
    }

    #[test]
    fn a_model_read_alone_is_refused_with_every_message() {
        let payload = serde_json::to_string(&broken()).unwrap();
        assert_eq!(
            serde_json::from_str::<Parcel>(&payload)
                .unwrap_err()
                .to_string(),
            PARCEL_REFUSAL
        );
        assert_eq!(refusal(&broken().validate().unwrap_err()), PARCEL_REFUSAL);
    }

    #[test]
    fn a_holder_is_refused_with_every_message_of_every_model_it_holds_at_its_full_path() {
        let held = manifest_of(&broken());
        assert_eq!(refusal(&held.validate().unwrap_err()), MANIFEST_REFUSAL);
        let payload = serde_json::to_string(&held).unwrap();
        assert_eq!(
            serde_json::from_str::<Manifest>(&payload)
                .unwrap_err()
                .to_string(),
            MANIFEST_REFUSAL
        );
    }

    #[test]
    fn a_value_is_refused_with_the_text_a_json_text_is_refused_with() {
        let written = serde_json::to_value(manifest_of(&broken())).unwrap();
        assert_eq!(
            serde_json::from_value::<Manifest>(written)
                .unwrap_err()
                .to_string(),
            MANIFEST_REFUSAL
        );
    }

    #[test]
    fn a_payload_that_keeps_every_rule_is_read_and_written_back_as_it_was() {
        let held = manifest_of(&sound());
        let payload = serde_json::to_string(&held).unwrap();
        let read: Manifest = serde_json::from_str(&payload).unwrap();
        assert_eq!(read, held);
        assert_eq!(serde_json::to_string(&read).unwrap(), payload);
        assert_eq!(
            payload,
            "{\"boxedParcel\":{\"fragileItemCodes\":[\"x\"],\"itemLabel\":\"A\"},\
             \"firstParcel\":{\"fragileItemCodes\":[\"x\"],\"itemLabel\":\"A\"},\
             \"note\":{\"text\":\"n\"},\
             \"parcelList\":[{\"fragileItemCodes\":[\"x\"],\"itemLabel\":\"A\"},\
             {\"fragileItemCodes\":[\"x\"],\"itemLabel\":\"A\"}],\
             \"parcelsByDock\":{\"north\":{\"fragileItemCodes\":[\"x\"],\"itemLabel\":\"A\"}},\
             \"spareParcel\":{\"fragileItemCodes\":[\"x\"],\"itemLabel\":\"A\"}}"
        );
        let unspared: Manifest = serde_json::from_value(json!({
            "boxedParcel": {"fragileItemCodes": [], "itemLabel": "A"},
            "firstParcel": {"fragileItemCodes": [], "itemLabel": "A"},
            "note": {"text": "n"},
            "parcelList": [],
            "parcelsByDock": {},
        }))
        .unwrap();
        assert_eq!(unspared.spare_parcel, None);
    }

    #[test]
    fn a_refusal_serde_raises_inside_a_held_model_is_serdes_own_text() {
        let mut written = serde_json::to_value(manifest_of(&sound())).unwrap();
        written["firstParcel"]["itemLabel"] = json!(5_i32);
        let payload = serde_json::to_string(&written).unwrap();
        let refused = serde_json::from_str::<Manifest>(&payload)
            .unwrap_err()
            .to_string();
        let within = serde_json::from_str::<Parcel>(r#"{"fragileItemCodes":["x"],"itemLabel":5}"#)
            .unwrap_err()
            .to_string();
        assert_eq!(
            within,
            "invalid type: integer `5`, expected a string at line 1 column 39"
        );
        assert_eq!(
            refused,
            "invalid type: integer `5`, expected a string at line 1 column 111"
        );
    }

    #[test]
    fn a_model_held_by_a_type_with_no_model_schema_refuses_with_its_own_messages() {
        let payload = format!(
            r#"{{"parcels":[{0},{0}]}}"#,
            serde_json::to_string(&broken()).unwrap()
        );
        assert_eq!(
            serde_json::from_str::<Plain>(&payload)
                .unwrap_err()
                .to_string(),
            format!("{PARCEL_REFUSAL} at line 1 column 58")
        );
        let read: Plain = serde_json::from_str(r#"{"parcels":[]}"#).unwrap();
        assert_eq!(read.parcels, Vec::<Parcel>::new());
    }

    #[test]
    fn a_model_held_where_its_holder_reads_through_no_hook_judges_itself_as_it_is_read() {
        let yard_of = |arced: &Parcel, fixed: &Parcel, paired: &Parcel| Yard {
            arced_parcel: Arc::new(arced.clone()),
            fixed_parcels: [fixed.clone()],
            paired_parcel: (paired.clone(), 1),
        };
        let kept = yard_of(&sound(), &sound(), &sound());
        let written = serde_json::to_string(&kept).unwrap();
        assert_eq!(serde_json::from_str::<Yard>(&written).unwrap(), kept);

        // `validate()` of the holder names what each place holds at its full path.
        assert_eq!(
            yard_of(&broken(), &broken(), &broken())
                .validate()
                .unwrap_err(),
            [
                "'arcedParcel.itemLabel': too short: minimum length is 1, got 0",
                "'arcedParcel.fragileItemCodes.1': y is outside of itemCodeList",
                "'fixedParcels.0.itemLabel': too short: minimum length is 1, got 0",
                "'fixedParcels.0.fragileItemCodes.1': y is outside of itemCodeList",
                "'pairedParcel.0.itemLabel': too short: minimum length is 1, got 0",
                "'pairedParcel.0.fragileItemCodes.1': y is outside of itemCodeList",
            ]
        );
        for held in [
            yard_of(&broken(), &sound(), &sound()),
            yard_of(&sound(), &broken(), &sound()),
            yard_of(&sound(), &sound(), &broken()),
            yard_of(&broken(), &broken(), &broken()),
        ] {
            let refused = serde_json::from_str::<Yard>(&serde_json::to_string(&held).unwrap())
                .unwrap_err()
                .to_string();
            // Each parcel judges itself as it is read, so the first broken one ends the read,
            // with its own two messages.
            for message in [
                "too short: minimum length is 1, got 0",
                "'fragileItemCodes.1': y is outside of itemCodeList",
            ] {
                assert_eq!(refused.matches(message).count(), 1, "got: {refused}");
            }
        }
    }

    #[test]
    fn the_readme_shows_what_a_read_answers() {
        let readme = include_str!("../../README.md");
        let mut changed = sound();
        changed.fragile_item_codes.push("y".to_owned());
        for shown in [
            format!(
                "serde_json::from_str::<Parcel>(r#\"{}\"#);\n// Err: {PARCEL_REFUSAL}\n",
                serde_json::to_string(&broken()).unwrap()
            ),
            format!("```text\n{}\n```", MANIFEST_REFUSAL.replace("; ", ";\n")),
            "`invalid type: integer `5`, expected a string at line 1 column 39`".to_owned(),
            "`... at line 1 column 58`".to_owned(),
            format!("parcel.validate();\n// {:?}\n", changed.validate()),
        ] {
            assert!(
                readme.contains(&shown),
                "the README no longer shows {shown}"
            );
        }
    }

    #[test]
    fn a_value_built_in_code_is_judged_by_validate_and_written_as_it_is() {
        let held = manifest_of(&broken());
        assert_eq!(refusal(&held.validate().unwrap_err()), MANIFEST_REFUSAL);
        assert_eq!(
            serde_json::to_string(&broken()).unwrap(),
            r#"{"fragileItemCodes":["x","y"],"itemLabel":""}"#
        );
        assert_eq!(
            serde_json::to_value(&held).unwrap()["firstParcel"]["itemLabel"],
            ""
        );
    }

    #[test]
    fn a_value_changed_after_it_was_read_is_judged_by_validate() {
        let mut parcel: Parcel =
            serde_json::from_str(r#"{"itemLabel":"A","fragileItemCodes":["x"]}"#).unwrap();
        assert_eq!(parcel.validate(), Ok(()));
        parcel.fragile_item_codes.push("y".to_owned());
        assert_eq!(
            parcel.validate(),
            Err(vec![
                "'fragileItemCodes.1': y is outside of itemCodeList".to_owned()
            ])
        );
    }

    #[test]
    fn a_tuple_struct_is_read_as_validate_answers() {
        for held in [sound(), broken()] {
            let labelled = Consignment("a".to_owned(), held.clone());
            let unlabelled = Consignment(String::new(), held.clone());
            let sealed = Sealed(held);
            read_as_validated(&labelled, labelled.validate());
            read_as_validated(&unlabelled, unlabelled.validate());
            read_as_validated(&sealed, sealed.validate());
        }
        assert!(Consignment(String::new(), broken()).validate().is_err());
        assert!(Sealed(broken()).validate().is_err());
    }

    #[test]
    fn a_unit_struct_is_read_as_its_check_answers() {
        read_as_validated(&Dock, Dock.validate());
        assert_eq!(
            serde_json::from_str::<Dock>("{}").unwrap_err().to_string(),
            "a closed dock takes no parcel"
        );
        DOCK_IS_OPEN.with(|open| open.set(true));
        read_as_validated(&Dock, Dock.validate());
        assert_eq!(serde_json::from_str::<Dock>("{}").ok(), Some(Dock));
    }

    #[test]
    fn an_enum_under_each_tagging_is_read_as_validate_answers() {
        for held in [sound(), broken()] {
            for freight in [
                Freight::Boxed(held.clone()),
                Freight::Loose {
                    freight_label: held.item_label.clone(),
                },
                Freight::Split(held.item_label.clone(), held.clone()),
                Freight::Stacked(vec![sound(), held.clone()]),
            ] {
                read_as_validated(&freight, freight.validate());
            }
            for haul in [
                Haul::Boxed(held.clone()),
                Haul::Loose {
                    haul_label: held.item_label.clone(),
                },
                Haul::Split("a".to_owned(), held.clone()),
            ] {
                read_as_validated(&haul, haul.validate());
            }
            for lift in [
                Lift::Boxed(held.clone()),
                Lift::Loose {
                    lift_label: held.item_label.clone(),
                },
            ] {
                read_as_validated(&lift, lift.validate());
            }
            for stock in [
                Stock::Keyed(BTreeMap::from([("north".to_owned(), held.clone())])),
                Stock::Listed(vec![sound(), held.clone()]),
                Stock::Single(held.clone()),
            ] {
                read_as_validated(&stock, stock.validate());
            }
        }
        for broken_whole in [
            Freight::Boxed(broken()).validate(),
            Haul::Boxed(broken()).validate(),
            Lift::Boxed(broken()).validate(),
            Stock::Single(broken()).validate(),
        ] {
            assert!(broken_whole.is_err());
        }
        read_as_validated(&Grade::Express, Grade::Express.validate());
        read_as_validated(&Grade::Unrated, Grade::Unrated.validate());
        assert_eq!(Grade::Unrated.validate().map_err(|said| said.len()), Err(1));
    }

    #[test]
    fn a_member_of_an_untagged_enum_chooses_its_variant_by_its_bound() {
        assert_eq!(
            serde_json::from_str::<Reading>(r#""abc""#).ok(),
            Some(Reading::Short("abc".to_owned()))
        );
        assert_eq!(
            serde_json::from_str::<Reading>(r#""abcd""#).ok(),
            Some(Reading::Text("abcd".to_owned()))
        );
        // A model an untagged variant holds is read as that variant, and judged with the whole.
        let listed = serde_json::to_string(&Stock::Listed(vec![broken()])).unwrap();
        assert_eq!(
            serde_json::from_str::<Stock>(&listed)
                .unwrap_err()
                .to_string(),
            refusal(&Stock::Listed(vec![broken()]).validate().unwrap_err())
        );
    }

    #[test]
    fn a_bounded_brand_refuses_as_it_is_read_under_the_key_of_the_field_that_holds_it() {
        assert_eq!(
            serde_json::from_str::<Code>(r#""a""#)
                .unwrap_err()
                .to_string(),
            "too short: minimum length is 2, got 1"
        );
        assert_eq!(
            serde_json::from_str::<Crane>(r#"{"loadCode":"a"}"#)
                .unwrap_err()
                .to_string(),
            "'loadCode': too short: minimum length is 2, got 1"
        );
        let read: Crane = serde_json::from_str(r#"{"loadCode":"ab"}"#).unwrap();
        assert_eq!(read.load_code, Code("ab".to_owned()));
    }

    /// The read names the brand by the path `validate()` names it by: below the variant's name,
    /// and below the content key.
    #[test]
    fn a_bounded_brand_in_a_variants_member_refuses_under_the_path_validate_names() {
        let coded = Sorted::Coded {
            parcel_code: Code("a".to_owned()),
        };
        let under_a_key = SortedUnderAKey::Coded {
            parcel_code: Code("a".to_owned()),
        };
        assert_eq!(
            [coded.validate(), under_a_key.validate()],
            [
                Err(vec![
                    "'Coded.parcelCode': too short: minimum length is 2, got 1".to_owned()
                ]),
                Err(vec![
                    "'held.parcelCode': too short: minimum length is 2, got 1".to_owned()
                ]),
            ]
        );
        assert_eq!(
            serde_json::from_str::<Sorted>(&serde_json::to_string(&coded).unwrap())
                .unwrap_err()
                .to_string(),
            "'Coded.parcelCode': too short: minimum length is 2, got 1"
        );
        assert_eq!(
            serde_json::from_str::<SortedUnderAKey>(&serde_json::to_string(&under_a_key).unwrap())
                .unwrap_err()
                .to_string(),
            "'held.parcelCode': too short: minimum length is 2, got 1"
        );
        let kept = Sorted::Coded {
            parcel_code: Code("ab".to_owned()),
        };
        read_as_validated(&kept, kept.validate());
    }

    #[test]
    fn a_generic_type_is_read_as_validate_answers_at_every_filling() {
        let texts = |codes: &[&str], label: &str| Crate {
            codes: codes.iter().map(|&code| code.to_owned()).collect(),
            label: label.to_owned(),
        };
        let numbers = |codes: &[u32], label: &str| Crate {
            codes: codes.to_vec(),
            label: label.to_owned(),
        };
        for (codes, label) in [
            (vec!["a"], "c"),
            (vec![], "c"),
            (vec!["a"], ""),
            (vec![], ""),
        ] {
            let held = texts(&codes, label);
            read_as_validated(&held, held.validate());
        }
        for (codes, label) in [(vec![7], "c"), (vec![], "c"), (vec![7], ""), (vec![], "")] {
            let held = numbers(&codes, label);
            read_as_validated(&held, held.validate());
        }
        assert_eq!(
            refusal(&numbers(&[], "").validate().unwrap_err()),
            "'label': too short: minimum length is 1, got 0; \
             'codes': a crate holds at least one code"
        );
    }

    /// Each type is read at a filling a bound wider than serde's would refuse.
    #[test]
    fn the_written_read_asks_of_a_parameter_what_serdes_derive_asks() {
        let unlabelled = "'label': too short: minimum length is 1, got 0";

        let tagged: Tagged<Unread> = serde_json::from_str(r#"{"label":"a"}"#).unwrap();
        assert_eq!(tagged.label, "a");
        assert_eq!(
            serde_json::from_str::<Tagged<Unread>>(r#"{"label":""}"#)
                .unwrap_err()
                .to_string(),
            unlabelled
        );

        let cached: Cached<Unwritten> = serde_json::from_str(r#"{"label":"a"}"#).unwrap();
        assert_eq!(cached.scratch, Vec::new());
        assert_eq!(
            serde_json::from_str::<Cached<Unwritten>>(r#"{"label":""}"#)
                .unwrap_err()
                .to_string(),
            unlabelled
        );

        let keyed: Keyed<String> =
            serde_json::from_str(r#"{"by_key":{"north":1},"label":"a"}"#).unwrap();
        assert_eq!(keyed.by_key.get("north"), Some(&1));
        assert_eq!(
            serde_json::from_str::<Keyed<String>>(r#"{"by_key":{},"label":""}"#)
                .unwrap_err()
                .to_string(),
            unlabelled
        );

        let stamped: Stamped<Unread> =
            serde_json::from_str(r#"{"label":"a","stamps":[1,2]}"#).unwrap();
        assert_eq!(stamped.stamps, Vec::new());
        assert_eq!(
            serde_json::from_str::<Stamped<Unread>>(r#"{"label":"","stamps":[]}"#)
                .unwrap_err()
                .to_string(),
            unlabelled
        );
    }

    /// A text read into an owned value outlives the text it was read from.
    fn owned_from(text: &str) -> Owned<'static> {
        serde_json::from_str(text).unwrap()
    }

    #[test]
    fn the_written_read_borrows_for_the_lifetimes_serdes_derive_borrows_for() {
        let unlabelled = "'label': too short: minimum length is 1, got 0";
        let text = String::from(r#"{"label":"a"}"#);
        let empty = String::from(r#"{"label":""}"#);

        let borrowed: Borrowed<'_> = serde_json::from_str(&text).unwrap();
        assert_eq!(borrowed.label, "a");
        assert_eq!(
            serde_json::from_str::<Borrowed<'_>>(&empty)
                .unwrap_err()
                .to_string(),
            unlabelled
        );

        let lent: Lent<'_> = serde_json::from_str(&text).unwrap();
        assert!(matches!(lent.label, Cow::Borrowed("a")));
        assert_eq!(
            serde_json::from_str::<Lent<'_>>(&empty)
                .unwrap_err()
                .to_string(),
            unlabelled
        );

        let owned = owned_from(&text);
        assert!(matches!(owned.label, Cow::Owned(_)));
        assert_eq!(
            serde_json::from_str::<Owned<'_>>(&empty)
                .unwrap_err()
                .to_string(),
            unlabelled
        );

        let fixed: Fixed = serde_json::from_str(r#"{"label":"a"}"#).unwrap();
        assert_eq!(fixed.label, "a");
        assert_eq!(
            serde_json::from_str::<Fixed>(r#"{"label":""}"#)
                .unwrap_err()
                .to_string(),
            unlabelled
        );
    }
}
