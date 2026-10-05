//! Generated code builds whatever names its author's module has in scope.

/// A module that imports a `Result` taking no type argument, beside each shape whose expansion
/// writes a `Result` of its own: a read hook for a sibling type, `validate()`, a constrained
/// brand's reader, and a unit struct's serde impls.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
mod a_one_parameter_result {
    use core::fmt::{Display, Formatter, Result};
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Named {
        #[model_schema_prop(minLength = 3)]
        pub name: String,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Ping;

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Record {
        pub versions: Vec<Version>,
    }

    #[model_schema(minLength = 3)]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct Slug(pub String);

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Version {
        pub number: i32,
    }

    impl Display for Record {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            write!(f, "{} versions", self.versions.len())
        }
    }

    #[test]
    fn every_shape_that_writes_a_result_builds_and_reads() {
        let record: Record =
            serde_json::from_value(serde_json::json!({ "versions": [{ "number": 1_i32 }] }))
                .unwrap();
        assert_eq!(record.to_string(), "1 versions");

        let named: Named = serde_json::from_value(serde_json::json!({ "name": "ab" })).unwrap();
        assert_eq!(named.validate().unwrap_err().len(), 1);

        serde_json::from_value::<Slug>(serde_json::json!("ab")).unwrap_err();
        assert_eq!(
            serde_json::from_value::<Slug>(serde_json::json!("abc")).unwrap(),
            Slug("abc".to_owned())
        );

        assert_eq!(
            serde_json::from_value::<Ping>(serde_json::json!({})).unwrap(),
            Ping
        );
    }
}

/// A module that imports a second trait with a `fmt` method beside a brand, whose `Display`
/// hands the formatting to the value it holds.
#[cfg(feature = "serde")]
mod a_second_trait_with_fmt {
    use core::fmt::Debug;
    #[cfg(feature = "mongodb")]
    use mongodb::bson::oid::ObjectId;
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    #[cfg(feature = "mongodb")]
    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct RecordId(pub ObjectId);

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct Slug(pub String);

    /// What `value` prints as under `Debug`, the trait the module imports by name.
    fn debugged<T>(value: &T) -> String
    where
        T: Debug,
    {
        format!("{value:?}")
    }

    #[test]
    fn a_brand_is_displayed_as_the_value_it_holds() {
        let slug = Slug("abc".to_owned());
        assert_eq!(slug.to_string(), "abc");
        assert_eq!(debugged(&slug), "Slug(\"abc\")");
    }

    #[cfg(feature = "mongodb")]
    #[test]
    fn a_brand_over_an_id_is_displayed_as_the_id() {
        let id = ObjectId::parse_str("6a7cc592ca0574e6efdfe217").unwrap();
        assert_eq!(RecordId(id).to_string(), "6a7cc592ca0574e6efdfe217");
    }
}
