//! A `#[serde(transparent)]` struct that derives one of serde's two derives is the brand over the
//! field that derive takes: the one serde writes under `Serialize` alone, and the one it reads under
//! `Deserialize` alone.

/// Two structs written with the same attributes, each beside the brand of one slot over the type of
/// the field its derive takes: serde writes `Written` as `count`, `text` being skipped when it
/// serializes, and reads `Received` as `text`, `count` having a `default`.
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
mod by_the_derive {
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    #[cfg(any(
        feature = "typescript",
        feature = "zod",
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    ))]
    const RECEIVED: [[&str; 2]; 3] = [
        ["TextAlone", "Received"],
        ["text_alone", "received"],
        ["textAlone", "received"],
    ];

    #[cfg(any(
        feature = "typescript",
        feature = "zod",
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    ))]
    const WRITTEN: [[&str; 2]; 3] = [
        ["CountAlone", "Written"],
        ["count_alone", "written"],
        ["countAlone", "written"],
    ];

    #[model_schema()]
    #[derive(Debug, Serialize)]
    #[serde(transparent)]
    pub struct CountAlone(pub u32);

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq)]
    #[serde(transparent)]
    pub struct Received {
        #[serde(default)]
        pub count: u32,
        #[serde(skip_serializing)]
        pub text: String,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize)]
    #[serde(transparent)]
    pub struct TextAlone(pub String);

    #[model_schema()]
    #[derive(Debug, Serialize)]
    #[serde(transparent)]
    pub struct Written {
        #[serde(default)]
        pub count: u32,
        #[serde(skip_serializing)]
        pub text: String,
    }

    /// What is written for a brand of one slot, under the names of the brand `names` maps it to.
    #[cfg(any(
        feature = "typescript",
        feature = "zod",
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    ))]
    fn under_the_name(written: &str, names: [[&str; 2]; 3]) -> String {
        names.iter().fold(written.to_owned(), |text, [from, to]| {
            text.replace(from, to)
        })
    }

    #[test]
    fn serde_writes_one_field_and_reads_the_other() {
        let written = Written {
            count: 7,
            text: "abc".to_owned(),
        };
        let renamed = Written {
            count: 7,
            text: "def".to_owned(),
        };
        assert_ne!(written.text, renamed.text);
        assert_eq!(
            serde_json::to_value(&written).unwrap(),
            serde_json::json!(7_u32)
        );
        assert_eq!(
            serde_json::to_value(&renamed).unwrap(),
            serde_json::to_value(&written).unwrap()
        );
        assert_eq!(
            serde_json::from_value::<Received>(serde_json::json!("abc")).unwrap(),
            Received {
                count: 0,
                text: "abc".to_owned(),
            }
        );
    }

    #[cfg(feature = "typescript")]
    #[test]
    fn typescript_brands_the_field_the_derive_takes() {
        assert_eq!(
            Written::ts_definition(),
            under_the_name(&CountAlone::ts_definition(), WRITTEN)
        );
        assert_eq!(
            Received::ts_definition(),
            under_the_name(&TextAlone::ts_definition(), RECEIVED)
        );
    }

    #[cfg(all(feature = "typescript", feature = "zod"))]
    #[test]
    fn typescript_brands_the_number_serde_writes() {
        let written = Written::ts_definition();
        assert!(
            written.contains("export type Written = number & $brand<\"Written\">;"),
            "got: {written}"
        );
    }

    #[cfg(feature = "zod")]
    #[test]
    fn zod_brands_the_field_the_derive_takes() {
        assert_eq!(
            Written::zod_schema(),
            under_the_name(&CountAlone::zod_schema(), WRITTEN)
        );
        assert_eq!(
            Received::zod_schema(),
            under_the_name(&TextAlone::zod_schema(), RECEIVED)
        );
    }

    #[cfg(feature = "jsonschema")]
    #[test]
    fn json_schema_describes_the_field_the_derive_takes() {
        assert_eq!(Written::json_schema(), CountAlone::json_schema());
        assert_eq!(
            Received::json_schema(),
            serde_json::json!({ "type": "string" })
        );
    }

    #[cfg(feature = "dart")]
    #[test]
    fn dart_brands_the_field_the_derive_takes() {
        assert_eq!(
            written_dart::dart_definition(),
            under_the_name(&count_alone_dart::dart_definition(), WRITTEN)
        );
        assert_eq!(
            received_dart::dart_definition(),
            under_the_name(&text_alone_dart::dart_definition(), RECEIVED)
        );
    }

    #[cfg(feature = "swift")]
    #[test]
    fn swift_brands_the_field_the_derive_takes() {
        assert_eq!(
            written_swift::swift_definition(),
            under_the_name(&count_alone_swift::swift_definition(), WRITTEN)
        );
        assert_eq!(
            received_swift::swift_definition(),
            under_the_name(&text_alone_swift::swift_definition(), RECEIVED)
        );
    }

    #[cfg(feature = "kotlin")]
    #[test]
    fn kotlin_brands_the_field_the_derive_takes() {
        assert_eq!(
            written_kotlin::kotlin_definition(),
            under_the_name(&count_alone_kotlin::kotlin_definition(), WRITTEN)
        );
        assert_eq!(
            received_kotlin::kotlin_definition(),
            under_the_name(&text_alone_kotlin::kotlin_definition(), RECEIVED)
        );
    }

    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    #[test]
    fn each_is_displayed_as_the_field_its_derive_takes() {
        let written = Written {
            count: 7,
            text: "abc".to_owned(),
        };
        assert_eq!(written.to_string(), "7");
        let received = Received {
            count: 7,
            text: "abc".to_owned(),
        };
        assert_eq!(received.to_string(), "abc");
    }
}
