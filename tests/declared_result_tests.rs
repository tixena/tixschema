//! A `#[model_schema()]` type of the crate's own named `Result`, declared above the type that holds
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

    /// The crate's own type named `Result`.
    #[model_schema(default_types(Held = u32, Refused = String))]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Result<Held, Refused> {
        pub held: Held,
        pub refused: Refused,
    }

    /// A field written with the crate's own `Result`.
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Shelved {
        pub outcome: Result<u32, String>,
    }

    #[test]
    fn the_crates_own_result_keeps_its_description_on_the_web_surfaces() {
        let typescript = Shelved::ts_definition();
        assert!(
            typescript.contains("outcome: Result<number, string>;"),
            "{typescript}"
        );
        let zod = Shelved::zod_schema();
        assert!(
            zod.contains("outcome: Result$SchemaFactory(z.number().int(), z.string()),"),
            "{zod}"
        );
        let document = Shelved::json_schema();
        assert_eq!(
            document["properties"]["outcome"],
            Result::<u32, String>::json_schema(),
            "{document}"
        );
        let value = Shelved {
            outcome: Result {
                held: 13_u32,
                refused: "late".to_owned(),
            },
        };
        assert_eq!(
            serde_json::to_value(&value).unwrap(),
            json!({ "outcome": { "held": 13_u32, "refused": "late" } })
        );
    }

    #[cfg(feature = "dart")]
    #[test]
    fn the_crates_own_result_keeps_its_description_in_dart() {
        let dart = shelved_dart::dart_definition();
        assert!(
            dart.contains("final Result<int, String> outcome;"),
            "{dart}"
        );
    }

    #[cfg(feature = "swift")]
    #[test]
    fn the_crates_own_result_keeps_its_description_in_swift() {
        let swift = shelved_swift::swift_definition();
        assert!(
            swift.contains("public let outcome: Result<UInt32, String>;"),
            "{swift}"
        );
        assert!(!swift.contains("ShelvedOutcomeResult"), "{swift}");
    }

    #[cfg(feature = "kotlin")]
    #[test]
    fn the_crates_own_result_keeps_its_description_in_kotlin() {
        let kotlin = shelved_kotlin::kotlin_definition();
        assert!(
            kotlin.contains("val outcome: Result<UInt, String>"),
            "{kotlin}"
        );
    }
}
