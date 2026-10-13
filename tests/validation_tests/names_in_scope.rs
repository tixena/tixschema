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
        #[model_schema_prop(minLength = 1)]
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

        let named = Named {
            name: "ab".to_owned(),
        };
        assert_eq!(named.validate().unwrap_err().len(), 1);
        assert_eq!(
            serde_json::from_value::<Named>(serde_json::json!({ "name": "ab" }))
                .unwrap_err()
                .to_string(),
            named.validate().unwrap_err().join("; ")
        );
        serde_json::from_value::<Named>(serde_json::json!({ "name": "abc" })).unwrap();

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
/// hands the formatting to the value it holds. A brand is displayed where a schema surface is on.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
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

/// A module that declares a type named `Sized` beside a struct holding another model type, whose
/// expansion bounds a type parameter `?Sized`.
#[cfg(all(feature = "serde", feature = "typescript"))]
mod a_type_named_sized {
    /// The same two types in a module that declares no `Sized`, to read the surfaces against.
    mod unshadowed {
        use serde::{Deserialize, Serialize};
        use tixschema::model_schema;

        #[model_schema()]
        #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
        pub struct PlainHolder {
            pub inner: PlainInner,
            #[model_schema_prop(minimum = 1)]
            pub level: i32,
            #[model_schema_prop(maxLength = 2)]
            pub listed: Box<[i32]>,
        }

        #[model_schema()]
        #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
        pub struct PlainInner {
            pub number: i32,
        }
    }

    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder {
        pub inner: Inner,
        #[model_schema_prop(minimum = 1)]
        pub level: i32,
        #[model_schema_prop(maxLength = 2)]
        pub listed: Box<[i32]>,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Inner {
        pub number: i32,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Sized {
        pub text: String,
    }

    /// `text` with the unshadowed pair's names put back to the shadowed pair's.
    fn renamed(text: &str) -> String {
        text.replace("PlainHolder", "Holder")
            .replace("PlainInner", "Inner")
    }

    #[test]
    fn a_struct_holding_a_model_type_describes_as_it_does_without_the_name() {
        assert_eq!(
            Holder::ts_definition(),
            renamed(&unshadowed::PlainHolder::ts_definition())
        );
        assert!(Sized::ts_definition().contains("text: string"));

        let refused = serde_json::from_value::<Holder>(
            serde_json::json!({ "inner": { "number": 1_i32 }, "level": 0_i32, "listed": [1_i32] }),
        )
        .unwrap_err()
        .to_string();
        assert_eq!(refused, "'level': too small: minimum is 1, got 0");
        let holder: Holder = serde_json::from_value(
            serde_json::json!({ "inner": { "number": 1_i32 }, "level": 1_i32, "listed": [1_i32] }),
        )
        .unwrap();
        holder.validate().unwrap();
    }

    #[cfg(feature = "zod")]
    #[test]
    fn its_zod_schema_is_what_it_is_without_the_name() {
        assert_eq!(
            Holder::zod_schema(),
            renamed(&unshadowed::PlainHolder::zod_schema())
        );
    }

    #[cfg(feature = "jsonschema")]
    #[test]
    fn its_json_schema_is_what_it_is_without_the_name() {
        assert_eq!(
            Holder::json_schema(),
            unshadowed::PlainHolder::json_schema()
        );
    }
}

/// A module that declares a model type named `Err` beside a struct whose expansion returns a `Result`.
#[cfg(all(feature = "serde", feature = "typescript"))]
mod a_type_named_err {
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct BesideErrHolder {
        pub inner: BesideErrInner,
        #[model_schema_prop(minLength = 3)]
        pub name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub note: Option<String>,
        #[model_schema_prop(minLength = 1)]
        pub tags: Vec<String>,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct BesideErrInner {
        pub number: i32,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Err;

    #[test]
    fn a_struct_declared_beside_it_builds_reads_and_validates() {
        let refused = serde_json::from_value::<BesideErrHolder>(serde_json::json!({
            "inner": { "number": 1_i32 },
            "name": "ab",
            "tags": ["t"],
        }))
        .unwrap_err()
        .to_string();
        assert_eq!(refused, "'name': too short: minimum length is 3, got 2");
        let read: BesideErrHolder = serde_json::from_value(serde_json::json!({
            "inner": { "number": 1_i32 },
            "name": "abc",
            "tags": ["t"],
        }))
        .unwrap();
        read.validate().unwrap();
        assert!(BesideErrHolder::ts_definition().contains("  inner: BesideErrInner;"));
        assert_eq!(serde_json::to_value(Err).unwrap(), serde_json::json!({}));
    }
}

/// A module that declares a model type named `None` beside a struct whose expansion matches on an `Option`.
#[cfg(all(feature = "serde", feature = "typescript"))]
mod a_type_named_none {
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct BesideNoneHolder {
        pub inner: BesideNoneInner,
        #[model_schema_prop(minLength = 3)]
        pub name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub note: Option<String>,
        #[model_schema_prop(minLength = 1)]
        pub tags: Vec<String>,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct BesideNoneInner {
        pub number: i32,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct None;

    #[test]
    fn a_struct_declared_beside_it_builds_reads_and_validates() {
        let refused = serde_json::from_value::<BesideNoneHolder>(serde_json::json!({
            "inner": { "number": 1_i32 },
            "name": "ab",
            "tags": ["t"],
        }))
        .unwrap_err()
        .to_string();
        assert_eq!(refused, "'name': too short: minimum length is 3, got 2");
        let read: BesideNoneHolder = serde_json::from_value(serde_json::json!({
            "inner": { "number": 1_i32 },
            "name": "abc",
            "tags": ["t"],
        }))
        .unwrap();
        read.validate().unwrap();
        assert!(BesideNoneHolder::ts_definition().contains("  inner: BesideNoneInner;"));
        assert_eq!(serde_json::to_value(None).unwrap(), serde_json::json!({}));
    }
}

/// A module that declares a model type named `Ok` beside a struct whose expansion returns a `Result`.
#[cfg(all(feature = "serde", feature = "typescript"))]
mod a_type_named_ok {
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct BesideOkHolder {
        pub inner: BesideOkInner,
        #[model_schema_prop(minLength = 3)]
        pub name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub note: Option<String>,
        #[model_schema_prop(minLength = 1)]
        pub tags: Vec<String>,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct BesideOkInner {
        pub number: i32,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Ok;

    #[test]
    fn a_struct_declared_beside_it_builds_reads_and_validates() {
        let refused = serde_json::from_value::<BesideOkHolder>(serde_json::json!({
            "inner": { "number": 1_i32 },
            "name": "ab",
            "tags": ["t"],
        }))
        .unwrap_err()
        .to_string();
        assert_eq!(refused, "'name': too short: minimum length is 3, got 2");
        let read: BesideOkHolder = serde_json::from_value(serde_json::json!({
            "inner": { "number": 1_i32 },
            "name": "abc",
            "tags": ["t"],
        }))
        .unwrap();
        read.validate().unwrap();
        assert!(BesideOkHolder::ts_definition().contains("  inner: BesideOkInner;"));
        assert_eq!(serde_json::to_value(Ok).unwrap(), serde_json::json!({}));
    }
}

/// A module that declares a model type named `Some` beside a struct whose expansion matches on an `Option`.
#[cfg(all(feature = "serde", feature = "typescript"))]
mod a_type_named_some {
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct BesideSomeHolder {
        pub inner: BesideSomeInner,
        #[model_schema_prop(minLength = 3)]
        pub name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub note: Option<String>,
        #[model_schema_prop(minLength = 1)]
        pub tags: Vec<String>,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct BesideSomeInner {
        pub number: i32,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Some;

    #[test]
    fn a_struct_declared_beside_it_builds_reads_and_validates() {
        let refused = serde_json::from_value::<BesideSomeHolder>(serde_json::json!({
            "inner": { "number": 1_i32 },
            "name": "ab",
            "tags": ["t"],
        }))
        .unwrap_err()
        .to_string();
        assert_eq!(refused, "'name': too short: minimum length is 3, got 2");
        let read: BesideSomeHolder = serde_json::from_value(serde_json::json!({
            "inner": { "number": 1_i32 },
            "name": "abc",
            "tags": ["t"],
        }))
        .unwrap();
        read.validate().unwrap();
        assert!(BesideSomeHolder::ts_definition().contains("  inner: BesideSomeInner;"));
        assert_eq!(serde_json::to_value(Some).unwrap(), serde_json::json!({}));
    }
}
