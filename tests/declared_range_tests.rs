//! A `#[model_schema()]` type of the crate's own named `Range`, declared above the type that holds
//! it, keeps its own description on every surface: a field written with the name is that type. A
//! binary of its own: the name is read off a registry the whole crate shares.

#![cfg(all(
    feature = "serde",
    feature = "typescript",
    feature = "zod",
    feature = "jsonschema"
))]

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_json::json;
    use tixschema::model_schema;

    /// The crate's own type named `Range`.
    #[model_schema(default_types(Bound = u32))]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Range<Bound> {
        pub high: Bound,
        pub low: Bound,
    }

    /// A field written with the crate's own `Range`.
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Shelved {
        pub span: Range<u32>,
    }

    #[test]
    fn the_crates_own_range_keeps_its_description_on_the_web_surfaces() {
        let typescript = Shelved::ts_definition();
        assert!(typescript.contains("span: Range<number>;"), "{typescript}");
        let zod = Shelved::zod_schema();
        assert!(
            zod.contains("span: Range$SchemaFactory(z.number().int()),"),
            "{zod}"
        );
        let document = Shelved::json_schema();
        assert_eq!(
            document["properties"]["span"],
            Range::<u32>::json_schema(),
            "{document}"
        );
        let value = Shelved {
            span: Range {
                high: 13_u32,
                low: 8_u32,
            },
        };
        assert_eq!(
            serde_json::to_value(&value).unwrap(),
            json!({ "span": { "high": 13_u32, "low": 8_u32 } })
        );
    }

    #[cfg(feature = "dart")]
    #[test]
    fn the_crates_own_range_keeps_its_description_in_dart() {
        let dart = shelved_dart::dart_definition();
        assert!(dart.contains("final Range<int> span;"), "{dart}");
    }

    #[cfg(feature = "swift")]
    #[test]
    fn the_crates_own_range_keeps_its_description_in_swift() {
        let swift = shelved_swift::swift_definition();
        assert!(swift.contains("public let span: Range<UInt32>;"), "{swift}");
        assert!(!swift.contains("ShelvedSpanRange"), "{swift}");
    }

    #[cfg(feature = "kotlin")]
    #[test]
    fn the_crates_own_range_keeps_its_description_in_kotlin() {
        let kotlin = shelved_kotlin::kotlin_definition();
        assert!(kotlin.contains("val span: Range<UInt>"), "{kotlin}");
        assert!(!kotlin.contains("KotlinRange"), "{kotlin}");
    }
}
