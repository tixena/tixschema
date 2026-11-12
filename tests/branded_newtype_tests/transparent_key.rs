//! A single-slot tuple struct is a brand wherever `transparent` is written among its serde keys.

/// The same brand declared twice: `transparent` written first, and written after a key that
/// carries a value. serde writes both the same way, so every surface describes both the same way.
#[cfg(feature = "serde")]
mod written_after_a_valued_key {
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    #[model_schema(enable = ["jsonschema"])]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent, rename = "Brand")]
    pub struct FirstBrand(String);

    #[model_schema(enable = ["jsonschema"])]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(rename = "Brand", transparent)]
    pub struct LastBrand(String);

    /// What is written for `LastBrand`, under the names `FirstBrand` is written with.
    fn under_the_first_name(written: &str) -> String {
        written
            .replace("LastBrand", "FirstBrand")
            .replace("last_brand", "first_brand")
            .replace("lastBrand", "firstBrand")
    }

    #[cfg(feature = "dart")]
    #[test]
    fn dart_describes_both_alike() {
        assert_eq!(
            under_the_first_name(&last_brand_dart::dart_definition()),
            first_brand_dart::dart_definition()
        );
    }

    #[test]
    fn json_schema_describes_both_alike() {
        assert_eq!(
            under_the_first_name(&LastBrand::json_schema().to_string()),
            FirstBrand::json_schema().to_string()
        );
    }

    #[cfg(feature = "kotlin")]
    #[test]
    fn kotlin_describes_both_alike() {
        assert_eq!(
            under_the_first_name(&last_brand_kotlin::kotlin_definition()),
            first_brand_kotlin::kotlin_definition()
        );
    }

    #[test]
    fn serde_writes_both_alike() {
        assert_eq!(
            serde_json::to_string(&FirstBrand::new("a".to_owned()).unwrap()).unwrap(),
            serde_json::to_string(&LastBrand::new("a".to_owned()).unwrap()).unwrap()
        );
    }

    #[cfg(feature = "swift")]
    #[test]
    fn swift_describes_both_alike() {
        assert_eq!(
            under_the_first_name(&last_brand_swift::swift_definition()),
            first_brand_swift::swift_definition()
        );
    }

    #[cfg(feature = "typescript")]
    #[test]
    fn typescript_brands_both() {
        let first = FirstBrand::ts_definition();
        assert!(first.contains("brand"), "got: {first}");
        assert_eq!(under_the_first_name(&LastBrand::ts_definition()), first);
    }
}
