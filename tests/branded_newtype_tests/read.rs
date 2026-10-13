//! A brand is read as a model is: its own read, followed by its `validate()`. A type that holds
//! one reads it with no judgement of its own, and names it at its full path.

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
mod a_brand_is_read_as_a_model_is {
    use alloc::collections::BTreeMap;
    use core::fmt::Debug;
    use serde::de::DeserializeOwned;
    use serde::{Deserialize, Serialize};
    use std::path::PathBuf;
    use tixschema::model_schema;

    /// What a shelf with a broken label in every place it holds one is refused with.
    const SHELF_REFUSAL: &str = "\
'boxedLabel': too short: minimum length is 2, got 1; \
'boxedLabel': a label is upper case; \
'firstLabel': too short: minimum length is 2, got 1; \
'firstLabel': a label is upper case; \
'labelList.1': too short: minimum length is 2, got 1; \
'labelList.1': a label is upper case; \
'labelsByDock.north': too short: minimum length is 2, got 1; \
'labelsByDock.north': a label is upper case; \
'spareLabel': too short: minimum length is 2, got 1; \
'spareLabel': a label is upper case";

    /// A brand that names a check alone.
    #[model_schema(check = ["upperCase"])]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct Code(pub String);

    /// A brand held to a bound and to a check.
    #[model_schema(minLength = 2, check = ["labelInUpperCase"])]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct Label(pub String);

    /// A brand over a number.
    #[model_schema(check = ["aboveZero"])]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct Count(pub u32);

    /// A brand over a path, measured by the text serde writes for it.
    #[model_schema(no_display, minLength = 3)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct Dir(pub PathBuf);

    /// A brand over a list: its length counts the items, and its check reads them.
    #[model_schema(no_display, minLength = 2, check = ["withoutARepeat"])]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct Codes(pub Vec<String>);

    /// A brand written with a named field.
    #[model_schema(maxLength = 3)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct Slug {
        pub text: String,
    }

    /// A brand over a string that publishes no `Display`.
    #[model_schema(no_display, maxLength = 3, check = ["plainInLowerCase"])]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct Plain(pub String);

    /// A brand with a parameter: its bound is its declared default's, and its check is written
    /// once over every filling.
    #[model_schema(minLength = 2, default_types(CodeType = String), check = ["notEmpty"])]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct Tag<CodeType>(pub CodeType)
    where
        CodeType: Default + PartialEq;

    /// Holds a label in each place a holder reads a model unjudged.
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Shelf {
        pub boxed_label: Box<Label>,
        pub first_label: Label,
        pub label_list: Vec<Label>,
        pub labels_by_dock: BTreeMap<String, Label>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub spare_label: Option<Label>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Listing {
        pub listed: Vec<Dir>,
    }

    /// A brand that is read and never written.
    #[model_schema(maxLength = 5)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
    #[serde(transparent)]
    pub struct Inbound {
        pub text: String,
    }

    /// Holds an inbound text under a bound of the field's own, which is measured on the brand's
    /// text.
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
    pub struct Receipt {
        #[model_schema_prop(minLength = 2)]
        pub inbound: Inbound,
        pub spare: Inbound,
    }

    /// Holds a label in the one slot of a variant, and in a slot of a variant of several.
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub enum Crated {
        Lone(Label),
        Pair(Label, u32),
    }

    /// Holds a label in a slot of a tuple struct of several.
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Paired(pub Label, pub u32);

    /// An untagged enum whose first variant is chosen by its brand's bound, and by no check.
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(untagged)]
    pub enum Marked {
        Labelled(Label),
        Listed(Vec<Label>),
        Loose(String),
    }

    fn upper_case(value: &Code) -> Result<(), Vec<String>> {
        if value.0.chars().any(char::is_lowercase) {
            return Err(vec!["a code is upper case".to_owned()]);
        }
        Ok(())
    }

    fn label_in_upper_case(value: &Label) -> Result<(), Vec<String>> {
        if value.0.chars().any(char::is_lowercase) {
            return Err(vec!["a label is upper case".to_owned()]);
        }
        Ok(())
    }

    fn above_zero(value: &Count) -> Result<(), Vec<String>> {
        if value.0 == 0 {
            return Err(vec!["a count is above zero".to_owned()]);
        }
        Ok(())
    }

    fn without_a_repeat(value: &Codes) -> Result<(), Vec<String>> {
        let repeated: Vec<String> = value
            .0
            .iter()
            .enumerate()
            .filter(|(index, code)| value.0[..*index].contains(code))
            .map(|(index, code)| format!("'{index}': {code} is listed twice"))
            .collect();
        if repeated.is_empty() {
            Ok(())
        } else {
            Err(repeated)
        }
    }

    fn plain_in_lower_case(value: &Plain) -> Result<(), Vec<String>> {
        if value.0.chars().any(char::is_uppercase) {
            return Err(vec!["a plain label is lower case".to_owned()]);
        }
        Ok(())
    }

    fn not_empty<CodeType>(value: &Tag<CodeType>) -> Result<(), Vec<String>>
    where
        CodeType: Default + PartialEq,
    {
        if value.0 == CodeType::default() {
            return Err(vec!["a tag is not empty".to_owned()]);
        }
        Ok(())
    }

    fn label(text: &str) -> Label {
        Label(text.to_owned())
    }

    fn codes(written: &[&str]) -> Codes {
        Codes(written.iter().map(|&code| code.to_owned()).collect())
    }

    /// A shelf that holds `held` in every place it holds a label, after a sound one in its list.
    fn shelf_of(held: &Label) -> Shelf {
        Shelf {
            boxed_label: Box::new(held.clone()),
            first_label: held.clone(),
            label_list: vec![label("OK"), held.clone()],
            labels_by_dock: BTreeMap::from([("north".to_owned(), held.clone())]),
            spare_label: Some(held.clone()),
        }
    }

    /// The text a read is refused with for a value `validate()` answers `reported` of.
    fn refusal(reported: &[String]) -> String {
        reported.join("; ")
    }

    /// Holds the read of what serde writes for `value`, from a text and from a
    /// `serde_json::Value`, to what `validate()` answers of `value`: an error with every message
    /// where it reports any, and the value where it reports none.
    fn read_as_validated<T>(value: &T, validated: Result<(), Vec<String>>)
    where
        T: Debug + DeserializeOwned + PartialEq + Serialize,
    {
        let payload = serde_json::to_string(value).unwrap();
        let read = serde_json::from_str::<T>(&payload).map_err(|refused| refused.to_string());
        let from_value = serde_json::from_value::<T>(serde_json::to_value(value).unwrap())
            .map_err(|refused| refused.to_string());
        match validated {
            Ok(()) => {
                assert_eq!(read.as_ref(), Ok(value), "{payload} is not read");
                assert_eq!(from_value.as_ref(), Ok(value), "{payload} is not read");
            }
            Err(reported) => {
                let refused = refusal(&reported);
                assert_eq!(read, Err(refused.clone()), "{payload} is read");
                assert_eq!(from_value, Err(refused), "{payload} is read from a value");
            }
        }
    }

    #[test]
    fn a_brand_that_names_a_check_alone_is_refused_as_it_is_read() {
        let lower = Code("abc".to_owned());
        assert_eq!(
            lower.validate(),
            Err(vec!["a code is upper case".to_owned()])
        );
        read_as_validated(&lower, lower.validate());
        let upper = Code("ABC".to_owned());
        assert_eq!(upper.validate(), Ok(()));
        read_as_validated(&upper, upper.validate());
    }

    #[test]
    fn a_brand_with_a_bound_and_a_check_is_refused_with_every_message() {
        assert_eq!(
            refusal(&label("a").validate().unwrap_err()),
            "too short: minimum length is 2, got 1; a label is upper case"
        );
        for text in ["a", "A", "ab", "AB"] {
            let held = label(text);
            read_as_validated(&held, held.validate());
        }
        assert_eq!(label("AB").validate(), Ok(()));
    }

    #[test]
    fn a_brand_over_a_number_is_read_as_its_check_answers() {
        assert_eq!(
            Count(0).validate(),
            Err(vec!["a count is above zero".to_owned()])
        );
        for held in [Count(0), Count(7)] {
            read_as_validated(&held, held.validate());
        }
    }

    #[test]
    fn a_brand_over_a_path_is_read_as_its_bound_answers() {
        let short = Dir(PathBuf::from("/a"));
        assert_eq!(
            short.validate(),
            Err(vec!["too short: minimum length is 3, got 2".to_owned()])
        );
        read_as_validated(&short, short.validate());
        let long = Dir(PathBuf::from("/etc"));
        read_as_validated(&long, long.validate());
    }

    #[test]
    fn a_brand_over_a_list_counts_its_items_and_runs_its_check_on_the_read() {
        assert_eq!(
            codes(&["a"]).validate(),
            Err(vec!["too short: minimum length is 2, got 1".to_owned()])
        );
        assert_eq!(
            codes(&["a", "a"]).validate(),
            Err(vec!["'1': a is listed twice".to_owned()])
        );
        assert_eq!(
            refusal(&codes(&[]).validate().unwrap_err()),
            "too short: minimum length is 2, got 0"
        );
        for written in [&[][..], &["a"], &["a", "a"], &["a", "b"], &["a", "b", "a"]] {
            let held = codes(written);
            read_as_validated(&held, held.validate());
        }
        assert_eq!(codes(&["a", "b"]).validate(), Ok(()));
    }

    #[test]
    fn a_brand_with_a_named_field_is_read_as_its_bound_answers() {
        let slug = |text: &str| Slug {
            text: text.to_owned(),
        };
        assert_eq!(
            slug("abcd").validate(),
            Err(vec!["too long: maximum length is 3, got 4".to_owned()])
        );
        for text in ["abcd", "abc"] {
            let held = slug(text);
            read_as_validated(&held, held.validate());
        }
    }

    #[test]
    fn a_brand_that_publishes_no_display_is_read_as_validate_answers() {
        let plain = |text: &str| Plain(text.to_owned());
        assert_eq!(
            refusal(&plain("ABCD").validate().unwrap_err()),
            "too long: maximum length is 3, got 4; a plain label is lower case"
        );
        for text in ["ABCD", "abcd", "ABC", "abc"] {
            let held = plain(text);
            read_as_validated(&held, held.validate());
        }
    }

    #[test]
    fn a_generic_brand_is_judged_at_its_declared_default_and_read_at_any_other_filling() {
        assert_eq!(
            refusal(&Tag(String::new()).validate().unwrap_err()),
            "too short: minimum length is 2, got 0; a tag is not empty"
        );
        for text in ["", "a", "ab"] {
            let held = Tag(text.to_owned());
            read_as_validated(&held, held.validate());
        }
        // The bound is the declared default's, so one digit is read at another filling.
        let counted = Tag(7_u32);
        assert_eq!(counted.validate(), Ok(()));
        read_as_validated(&counted, counted.validate());
        assert_eq!(serde_json::from_str::<Tag<u32>>("7").ok(), Some(Tag(7)));
        // The check is written once over every filling.
        let zero = Tag(0_u32);
        assert_eq!(zero.validate(), Err(vec!["a tag is not empty".to_owned()]));
        read_as_validated(&zero, zero.validate());
        // A filling with no `Display` is read as well: nothing renders the value there.
        let listed = Tag(vec![7_u8]);
        assert_eq!(listed.validate(), Ok(()));
        read_as_validated(&listed, listed.validate());
    }

    #[test]
    fn a_holder_names_every_broken_brand_it_holds_at_its_full_path() {
        let held = shelf_of(&label("a"));
        assert_eq!(refusal(&held.validate().unwrap_err()), SHELF_REFUSAL);
        read_as_validated(&held, held.validate());

        let sound = shelf_of(&label("AB"));
        assert_eq!(sound.validate(), Ok(()));
        read_as_validated(&sound, sound.validate());
        let unspared: Shelf = serde_json::from_str(
            r#"{"boxedLabel":"AB","firstLabel":"AB","labelList":[],"labelsByDock":{}}"#,
        )
        .unwrap();
        assert_eq!(unspared.spare_label, None);
    }

    #[test]
    fn a_list_of_brands_names_each_broken_item_by_its_index() {
        let held = Listing {
            listed: vec![
                Dir(PathBuf::from("/a")),
                Dir(PathBuf::from("/etc")),
                Dir(PathBuf::from("/b")),
            ],
        };
        let refused = serde_json::from_str::<Listing>(r#"{"listed":["/a","/etc","/b"]}"#)
            .unwrap_err()
            .to_string();
        assert_eq!(
            refused,
            "'listed.0': too short: minimum length is 3, got 2; \
             'listed.2': too short: minimum length is 3, got 2"
        );
        assert_eq!(refused, refusal(&held.validate().unwrap_err()));
    }

    /// The read of a brand held in a slot answers what `validate()` answers of the value read,
    /// under the name `validate()` gives the slot.
    #[test]
    fn a_brand_held_in_a_slot_is_read_as_validate_answers() {
        for text in ["a", "AB"] {
            let lone = Crated::Lone(label(text));
            read_as_validated(&lone, lone.validate());
            let pair = Crated::Pair(label(text), 7);
            read_as_validated(&pair, pair.validate());
            let paired = Paired(label(text), 7);
            read_as_validated(&paired, paired.validate());
        }
        for broken in [
            Crated::Lone(label("a")).validate(),
            Crated::Pair(label("a"), 7).validate(),
            Paired(label("a"), 7).validate(),
        ] {
            assert_eq!(broken.map_err(|reported| reported.len()), Err(2));
        }
    }

    /// A brand that derives `Deserialize` alone keeps what a brand publishes beside its read: a
    /// bound written on a field typed with it is still measured on the brand's text.
    #[test]
    fn a_brand_that_is_only_read_is_judged_and_still_measured_by_a_field() {
        let inbound = |text: &str| Inbound {
            text: text.to_owned(),
        };
        assert_eq!(
            serde_json::from_str::<Inbound>(r#""abcdef""#).map_err(|refused| refused.to_string()),
            Err("too long: maximum length is 5, got 6".to_owned())
        );
        assert_eq!(
            serde_json::from_str::<Inbound>(r#""abc""#).ok(),
            Some(inbound("abc"))
        );
        let short = Receipt {
            inbound: inbound("a"),
            spare: inbound("abcdef"),
        };
        assert_eq!(
            serde_json::from_str::<Receipt>(r#"{"inbound":"a","spare":"abcdef"}"#)
                .map_err(|refused| refused.to_string()),
            Err(refusal(&short.validate().unwrap_err()))
        );
        assert_eq!(
            short.validate().unwrap_err(),
            [
                "'inbound': too short: minimum length is 2, got 1",
                "'spare': too long: maximum length is 5, got 6",
            ]
        );
        assert_eq!(
            serde_json::from_str::<Receipt>(r#"{"inbound":"ab","spare":"abc"}"#).ok(),
            Some(Receipt {
                inbound: inbound("ab"),
                spare: inbound("abc"),
            })
        );
    }

    #[test]
    fn the_readme_shows_what_the_read_of_a_brand_answers() {
        let readme = include_str!("../../README.md");
        let listing = r#"{"listed":["/a","/etc","/b"]}"#;
        for shown in [
            format!(
                "serde_json::from_str::<Label>(r#\"\"a\"\"#);\n// Err: {}\n",
                serde_json::from_str::<Label>(r#""a""#).unwrap_err()
            ),
            format!(
                "serde_json::from_str::<Listing>(r#\"{listing}\"#);\n// Err: {}\n",
                serde_json::from_str::<Listing>(listing).unwrap_err()
            ),
        ] {
            assert!(
                readme.contains(&shown),
                "the README no longer shows {shown}"
            );
        }
    }

    #[test]
    fn a_brand_read_alone_answers_the_sentences_of_its_own_rules() {
        assert_eq!(
            serde_json::from_str::<Dir>(r#""/a""#)
                .unwrap_err()
                .to_string(),
            "too short: minimum length is 3, got 2"
        );
        assert_eq!(
            serde_json::from_str::<Label>(r#""a""#)
                .unwrap_err()
                .to_string(),
            "too short: minimum length is 2, got 1; a label is upper case"
        );
        // A refusal serde raises itself is serde's, unchanged.
        assert_eq!(
            serde_json::from_str::<Label>("5").unwrap_err().to_string(),
            "invalid type: integer `5`, expected a string at line 1 column 1"
        );
    }

    /// The bound of a brand chooses the variant of an untagged enum, as it did where the brand
    /// refused in a read of its own. A check chooses none: the enum's read refuses with it.
    #[test]
    fn the_bound_of_a_brand_chooses_the_variant_of_an_untagged_enum_and_its_check_none() {
        assert_eq!(
            serde_json::from_str::<Marked>(r#""AB""#).ok(),
            Some(Marked::Labelled(label("AB")))
        );
        assert_eq!(
            serde_json::from_str::<Marked>(r#""a""#).ok(),
            Some(Marked::Loose("a".to_owned()))
        );
        assert_eq!(
            serde_json::from_str::<Marked>(r#"["AB","CD"]"#).ok(),
            Some(Marked::Listed(vec![label("AB"), label("CD")]))
        );
        assert_eq!(
            serde_json::from_str::<Marked>(r#"["AB","c"]"#)
                .unwrap_err()
                .to_string(),
            "data did not match any variant of untagged enum Marked"
        );
        for held in [
            Marked::Labelled(label("ab")),
            Marked::Listed(vec![label("AB"), label("cd")]),
        ] {
            assert!(held.validate().is_err(), "{held:?} is kept");
            read_as_validated(&held, held.validate());
        }
    }
}
