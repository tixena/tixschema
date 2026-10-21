//! A `#[serde(transparent)]` tuple struct whose other slots serde skips is the brand a struct of one
//! slot is, over the one slot serde reads it as, wherever that slot sits.

/// The same brand declared with a skipped marker after its slot, before it, and alone. serde writes
/// all three as the string alone, so every surface describes the three the same way.
#[cfg(all(
    feature = "serde",
    any(
        feature = "typescript",
        feature = "jsonschema",
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    )
))]
mod beside_the_lone_slot {
    use core::marker::PhantomData;
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    #[cfg(any(
        feature = "typescript",
        feature = "zod",
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    ))]
    const AFTER: [&str; 3] = ["MarkedAfter", "marked_after", "markedAfter"];

    #[cfg(any(
        feature = "typescript",
        feature = "zod",
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    ))]
    const BEFORE: [&str; 3] = ["MarkedBefore", "marked_before", "markedBefore"];

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct MarkedAfter(pub String, #[serde(skip)] pub PhantomData<u8>);

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct MarkedBefore(#[serde(skip)] pub PhantomData<u8>, pub String);

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct MarkedHolder {
        pub after: MarkedAfter,
        pub before: MarkedBefore,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct SlotAlone(pub String);

    /// What is written for the struct of one slot, under the names of the brand `pascal`, whose
    /// snake case is `snake` and whose camel case is `camel`.
    #[cfg(any(
        feature = "typescript",
        feature = "zod",
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    ))]
    fn under_the_name(written: &str, [pascal, snake, camel]: [&str; 3]) -> String {
        written
            .replace("SlotAlone", pascal)
            .replace("slot_alone", snake)
            .replace("slotAlone", camel)
    }

    #[test]
    fn serde_writes_and_reads_the_slot_alone() {
        let after = MarkedAfter("abc".to_owned(), PhantomData);
        let before = MarkedBefore(PhantomData, "def".to_owned());
        assert_eq!(
            serde_json::to_value(&after).unwrap(),
            serde_json::json!("abc")
        );
        assert_eq!(
            serde_json::to_value(&before).unwrap(),
            serde_json::json!("def")
        );
        assert_eq!(
            serde_json::from_value::<MarkedAfter>(serde_json::json!("abc")).unwrap(),
            after
        );
        assert_eq!(
            serde_json::from_value::<MarkedBefore>(serde_json::json!("def")).unwrap(),
            before
        );
        let holder = MarkedHolder { after, before };
        let written = serde_json::to_value(&holder).unwrap();
        assert_eq!(
            written,
            serde_json::json!({ "after": "abc", "before": "def" })
        );
        assert_eq!(
            serde_json::from_value::<MarkedHolder>(written).unwrap(),
            holder
        );
    }

    #[cfg(feature = "typescript")]
    #[test]
    fn typescript_describes_each_as_the_struct_of_one_slot() {
        assert_eq!(
            MarkedAfter::ts_definition(),
            under_the_name(&SlotAlone::ts_definition(), AFTER)
        );
        assert_eq!(
            MarkedBefore::ts_definition(),
            under_the_name(&SlotAlone::ts_definition(), BEFORE)
        );
        let holder = MarkedHolder::ts_definition();
        assert!(holder.contains("  after: MarkedAfter;"), "got: {holder}");
        assert!(holder.contains("  before: MarkedBefore;"), "got: {holder}");
    }

    #[cfg(all(feature = "typescript", feature = "zod"))]
    #[test]
    fn typescript_brands_the_string() {
        assert!(
            MarkedBefore::ts_definition()
                .contains("export type MarkedBefore = string & $brand<\"MarkedBefore\">;"),
            "got: {}",
            MarkedBefore::ts_definition()
        );
    }

    #[cfg(feature = "zod")]
    #[test]
    fn zod_describes_each_as_the_struct_of_one_slot() {
        assert_eq!(
            MarkedAfter::zod_schema(),
            under_the_name(&SlotAlone::zod_schema(), AFTER)
        );
        assert_eq!(
            MarkedBefore::zod_schema(),
            under_the_name(&SlotAlone::zod_schema(), BEFORE)
        );
    }

    #[cfg(feature = "jsonschema")]
    #[test]
    fn json_schema_describes_the_slot_and_a_type_that_holds_it() {
        let string = serde_json::json!({ "type": "string" });
        assert_eq!(MarkedAfter::json_schema(), string);
        assert_eq!(MarkedBefore::json_schema(), string);
        assert_eq!(
            MarkedHolder::json_schema(),
            serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "after": { "type": "string" },
                    "before": { "type": "string" },
                },
                "required": ["after", "before"],
            })
        );
    }

    #[cfg(feature = "dart")]
    #[test]
    fn dart_describes_each_as_the_struct_of_one_slot() {
        assert_eq!(
            marked_after_dart::dart_definition(),
            under_the_name(&slot_alone_dart::dart_definition(), AFTER)
        );
        assert_eq!(
            marked_before_dart::dart_definition(),
            under_the_name(&slot_alone_dart::dart_definition(), BEFORE)
        );
    }

    #[cfg(feature = "swift")]
    #[test]
    fn swift_describes_each_as_the_struct_of_one_slot() {
        assert_eq!(
            marked_after_swift::swift_definition(),
            under_the_name(&slot_alone_swift::swift_definition(), AFTER)
        );
        assert_eq!(
            marked_before_swift::swift_definition(),
            under_the_name(&slot_alone_swift::swift_definition(), BEFORE)
        );
    }

    #[cfg(feature = "kotlin")]
    #[test]
    fn kotlin_describes_each_as_the_struct_of_one_slot() {
        assert_eq!(
            marked_after_kotlin::kotlin_definition(),
            under_the_name(&slot_alone_kotlin::kotlin_definition(), AFTER)
        );
        assert_eq!(
            marked_before_kotlin::kotlin_definition(),
            under_the_name(&slot_alone_kotlin::kotlin_definition(), BEFORE)
        );
    }

    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    #[test]
    fn each_is_displayed_as_the_slot_it_reads() {
        assert_eq!(MarkedAfter("x".to_owned(), PhantomData).to_string(), "x");
        assert_eq!(MarkedBefore(PhantomData, "y".to_owned()).to_string(), "y");
    }
}

/// A bound on such a brand, and on a field typed with it, measures the slot serde reads, after the
/// marker it skips.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
mod bounded_after_the_marker {
    use core::marker::PhantomData;
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    const TOO_SHORT: &str = "too short: minimum length is 2, got 1";

    #[model_schema(no_display, maxLength = 2)]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct CountedMarked(#[serde(skip)] pub PhantomData<u8>, pub Vec<String>);

    #[model_schema(minLength = 2, default_types(TextType = String))]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct FilledMarked<TextType>(#[serde(skip)] pub PhantomData<u8>, pub TextType);

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct LooseMarked(#[serde(skip)] pub PhantomData<u8>, pub String);

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct LooseMarkedHolder {
        #[model_schema_prop(maxLength = 3)]
        pub slug: LooseMarked,
    }

    #[model_schema(minLength = 2)]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct ShortMarked(#[serde(skip)] pub PhantomData<u8>, pub String);

    #[test]
    fn validate_measures_the_slot_serde_reads() {
        assert_eq!(
            ShortMarked(PhantomData, "a".to_owned()).validate(),
            Err(vec![TOO_SHORT.to_owned()])
        );
        assert_eq!(ShortMarked(PhantomData, "ab".to_owned()).validate(), Ok(()));
        assert_eq!(
            FilledMarked::<String>(PhantomData, "a".to_owned()).validate(),
            Err(vec![TOO_SHORT.to_owned()])
        );
        assert_eq!(
            CountedMarked(PhantomData, vec!["a".to_owned(); 3]).validate(),
            Err(vec!["too long: maximum length is 2, got 3".to_owned()])
        );
    }

    #[test]
    fn the_read_refuses_what_validate_refuses() {
        assert_eq!(
            serde_json::from_str::<ShortMarked>(r#""ab""#).unwrap(),
            ShortMarked(PhantomData, "ab".to_owned())
        );
        for refused in [
            serde_json::from_str::<ShortMarked>(r#""a""#).unwrap_err(),
            serde_json::from_str::<FilledMarked<String>>(r#""a""#).unwrap_err(),
        ] {
            assert!(refused.to_string().contains(TOO_SHORT), "got: {refused}");
        }
        let counted = serde_json::from_str::<CountedMarked>(r#"["a","b","c"]"#).unwrap_err();
        assert!(
            counted
                .to_string()
                .contains("too long: maximum length is 2, got 3"),
            "got: {counted}"
        );
    }

    #[test]
    fn a_bound_on_a_field_typed_with_it_measures_the_slot() {
        assert_eq!(
            LooseMarkedHolder {
                slug: LooseMarked(PhantomData, "abcd".to_owned()),
            }
            .validate(),
            Err(vec![
                "'slug': too long: maximum length is 3, got 4".to_owned()
            ])
        );
    }
}
