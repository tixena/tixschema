//! `minLength` and `maxLength` on a string count Unicode characters, the unit Zod and a JSON
//! Schema validator count, in a field, in a brand, in a read and in a unit variant's name.

#[cfg(feature = "serde")]
mod a_length_counts_characters {
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    const ACCENT: &str = "\u{e9}";
    const COMBINED: &str = "e\u{301}";
    const SMILE: &str = "\u{1f600}";
    const TWO_SMILES: &str = "\u{1f600}\u{1f600}";

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct AtMostOne {
        #[model_schema_prop(maxLength = 1)]
        pub name: String,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct AtLeastTwo {
        #[model_schema_prop(minLength = 2)]
        pub name: String,
    }

    #[model_schema(maxLength = 1)]
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(transparent)]
    pub struct OneMax(String);

    #[model_schema(minLength = 2)]
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(transparent)]
    pub struct TwoMin(String);

    #[model_schema(maxLength = 1)]
    #[derive(Debug, Deserialize, Serialize)]
    pub enum Marks {
        #[serde(rename = "\u{e9}")]
        Accent,
        #[serde(rename = "\u{1f600}")]
        Smile,
    }

    fn at_most_one(name: &str) -> Result<(), Vec<String>> {
        AtMostOneDraft {
            name: name.to_owned(),
        }
        .validate()
        .map(|_valid| ())
    }

    fn at_least_two(name: &str) -> Result<(), Vec<String>> {
        AtLeastTwoDraft {
            name: name.to_owned(),
        }
        .validate()
        .map(|_valid| ())
    }

    fn too_long(count: usize) -> Result<(), Vec<String>> {
        Err(vec![format!(
            "'name': too long: maximum length is 1, got {count}"
        )])
    }

    fn too_short(count: usize) -> Result<(), Vec<String>> {
        Err(vec![format!(
            "'name': too short: minimum length is 2, got {count}"
        )])
    }

    #[test]
    fn a_field_under_a_maximum_counts_characters() {
        assert_eq!(at_most_one(ACCENT), Ok(()));
        assert_eq!(at_most_one(SMILE), Ok(()));
        assert_eq!(at_most_one(COMBINED), too_long(2));
        assert_eq!(at_most_one(TWO_SMILES), too_long(2));
    }

    #[test]
    fn a_field_under_a_minimum_counts_characters() {
        assert_eq!(at_least_two(ACCENT), too_short(1));
        assert_eq!(at_least_two(SMILE), too_short(1));
        assert_eq!(at_least_two(COMBINED), Ok(()));
        assert_eq!(at_least_two(TWO_SMILES), Ok(()));
    }

    #[test]
    fn a_brand_under_a_maximum_counts_characters() {
        let held = |text: &str| OneMax::new(text.to_owned()).map(|_| ());
        let refused =
            |count: usize| Err(vec![format!("too long: maximum length is 1, got {count}")]);
        assert_eq!(held(ACCENT), Ok(()));
        assert_eq!(held(SMILE), Ok(()));
        assert_eq!(held(COMBINED), refused(2));
        assert_eq!(held(TWO_SMILES), refused(2));
    }

    #[test]
    fn a_brand_under_a_minimum_counts_characters() {
        let held = |text: &str| TwoMin::new(text.to_owned()).map(|_| ());
        let refused =
            |count: usize| Err(vec![format!("too short: minimum length is 2, got {count}")]);
        assert_eq!(held(ACCENT), refused(1));
        assert_eq!(held(SMILE), refused(1));
        assert_eq!(held(COMBINED), Ok(()));
        assert_eq!(held(TWO_SMILES), Ok(()));
    }

    #[test]
    fn the_read_of_a_brand_counts_characters() {
        let read = |text: &str| serde_json::from_value::<OneMax>(serde_json::json!(text));
        read(ACCENT).unwrap();
        read(SMILE).unwrap();
        for refused in [COMBINED, TWO_SMILES] {
            let message = read(refused).unwrap_err().to_string();
            assert!(
                message.contains("too long: maximum length is 1, got 2"),
                "{message}"
            );
        }
    }

    #[test]
    fn the_read_of_a_field_counts_characters() {
        let read =
            |text: &str| serde_json::from_value::<AtMostOne>(serde_json::json!({ "name": text }));
        read(ACCENT).unwrap();
        read(SMILE).unwrap();
        for refused in [COMBINED, TWO_SMILES] {
            let message = read(refused).unwrap_err().to_string();
            assert!(
                message.contains("'name': too long: maximum length is 1, got 2"),
                "{message}"
            );
        }
    }

    #[test]
    fn a_unit_variant_named_with_one_character_builds_under_a_maximum_of_one() {
        assert_eq!(
            serde_json::to_string(&MarksDraft::Accent).unwrap(),
            "\"\u{e9}\""
        );
        assert_eq!(
            serde_json::to_string(&MarksDraft::Smile).unwrap(),
            "\"\u{1f600}\""
        );
    }
}
