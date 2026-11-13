//! A generic brand judges the value it holds where Rust can measure it: at every filling when its
//! inner names no parameter, and at the declared default when the inner is the parameter.

#[cfg(feature = "serde")]
mod a_marker_parameter {
    use core::marker::PhantomData;
    use serde::{Deserialize, Serialize};
    use std::path::PathBuf;
    use tixschema::model_schema;

    const TOO_SHORT: &str = "too short: minimum length is 3, got 2";

    #[model_schema(no_display, minLength = 3, default_types(Mark = String))]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct MarkedPath<Mark> {
        #[serde(skip)]
        mark: PhantomData<Mark>,
        path: PathBuf,
    }

    #[model_schema(minLength = 3, default_types(Mark = String))]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct MarkedText<Mark> {
        #[serde(skip)]
        mark: PhantomData<Mark>,
        text: String,
    }

    /// A marker that implements no `Display`.
    #[derive(Debug)]
    pub struct Opaque;

    #[test]
    fn a_path_inner_is_judged_at_every_filling() {
        let short = MarkedPath::<u32>::model_schema_read_unjudged(
            serde_json::to_value(PathBuf::from("ab")).unwrap(),
        )
        .unwrap();
        assert_eq!(short.validation_issues(), vec![TOO_SHORT.to_owned()]);
        let counted = serde_json::from_str::<MarkedPath<u32>>(r#""ab""#).unwrap_err();
        assert!(counted.to_string().contains(TOO_SHORT), "got: {counted}");
        let opaque = serde_json::from_str::<MarkedPath<Opaque>>(r#""ab""#).unwrap_err();
        assert!(opaque.to_string().contains(TOO_SHORT), "got: {opaque}");
        let sound = serde_json::from_str::<MarkedPath<Opaque>>(r#""abc""#).unwrap();
        assert_eq!(sound.validation_issues(), Vec::<String>::new());
    }

    #[test]
    fn a_string_inner_is_judged_at_every_filling() {
        let short = MarkedText::<u32>::model_schema_read_unjudged(
            serde_json::to_value("ab".to_owned()).unwrap(),
        )
        .unwrap();
        assert_eq!(short.validation_issues(), vec![TOO_SHORT.to_owned()]);
        let refused = serde_json::from_str::<MarkedText<Opaque>>(r#""ab""#).unwrap_err();
        assert!(refused.to_string().contains(TOO_SHORT), "got: {refused}");
        let sound = serde_json::from_str::<MarkedText<Opaque>>(r#""abc""#).unwrap();
        assert_eq!(sound.validation_issues(), Vec::<String>::new());
    }
}

#[cfg(feature = "serde")]
mod a_path_default {
    use serde::{Deserialize, Serialize};
    use std::path::PathBuf;
    use tixschema::model_schema;

    const TOO_SHORT: &str = "too short: minimum length is 3, got 2";

    #[model_schema(no_display, minLength = 3, default_types(Held = PathBuf))]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct Located<Held>(Held);

    #[test]
    fn the_default_is_measured_as_a_path_and_no_other_filling() {
        assert_eq!(
            Located::new(PathBuf::from("ab")).map(|_| ()),
            Err(vec![TOO_SHORT.to_owned()])
        );
        let refused = serde_json::from_str::<Located<PathBuf>>(r#""ab""#).unwrap_err();
        assert!(refused.to_string().contains(TOO_SHORT), "got: {refused}");
        assert_eq!(Located::new(7_u32).map(|_| ()), Ok(()));
        assert_eq!(
            serde_json::from_str::<Located<u32>>("7").unwrap(),
            Located::new(7_u32).unwrap()
        );
        assert_eq!(Located::new(vec![7_u8]).map(|_| ()), Ok(()));
    }
}

#[cfg(feature = "serde")]
mod beside_a_type_named_string {
    use alloc::string::String as StdString;
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    pub struct String;

    #[model_schema(minLength = 2)]
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(transparent)]
    pub struct Code<T>(T);

    #[test]
    fn the_checks_run_at_the_standard_string() {
        assert_eq!(
            Code::new("a".to_owned()).map(|_| ()),
            Err(vec!["too short: minimum length is 2, got 1".to_owned()])
        );
        let _: String = String;
        assert_eq!(Code::new(7_u32).map(|_| ()), Ok(()));
        let refused = serde_json::from_str::<Code<StdString>>(r#""a""#).unwrap_err();
        assert!(
            refused
                .to_string()
                .contains("too short: minimum length is 2, got 1"),
            "got: {refused}"
        );
        assert_eq!(
            *serde_json::from_str::<Code<u32>>("7").unwrap().as_inner(),
            7
        );
    }
}
