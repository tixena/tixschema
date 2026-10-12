//! A slot of a tuple struct takes the five constraints, checked where a named field of a struct is.

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
mod a_check_on_a_slot {
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Pair(
        #[model_schema_prop(minLength = 1)] pub String,
        #[model_schema_prop(minimum = 1)] pub u32,
    );

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct EachAlone(
        #[model_schema_prop(minLength = 2)] pub String,
        #[model_schema_prop(maxLength = 2)] pub String,
        #[model_schema_prop(pattern = "^[a-z]+$")] pub String,
        #[model_schema_prop(minimum = 2)] pub u32,
        #[model_schema_prop(maximum = 2)] pub u32,
    );

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct Together(
        #[model_schema_prop(minLength = 2, maxLength = 3, pattern = "^[a-z]+$")] pub String,
        #[model_schema_prop(minimum = 1, maximum = 9)] pub u32,
    );

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Lone(#[model_schema_prop(minLength = 1)] pub String);

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct Wrapped(
        #[model_schema_prop(minLength = 2)] pub Option<String>,
        #[model_schema_prop(minLength = 2)] pub Vec<String>,
        pub bool,
    );

    #[model_schema(default_types(T = String))]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct Labelled<T>(#[model_schema_prop(minLength = 1)] pub String, pub T);

    fn each_alone(
        least: &str,
        most: &str,
        matched: &str,
        low: u32,
        high: u32,
    ) -> Result<(), Vec<String>> {
        EachAlone(
            least.to_owned(),
            most.to_owned(),
            matched.to_owned(),
            low,
            high,
        )
        .validate()
    }

    #[test]
    fn validate_reports_each_slot_under_its_position() {
        let both = vec![
            "'0': too short: minimum length is 1, got 0".to_owned(),
            "'1': too small: minimum is 1, got 0".to_owned(),
        ];
        assert_eq!(Pair(String::new(), 0).validate().unwrap_err(), both);
        assert_eq!(Pair(String::new(), 1).validate().unwrap_err(), both[..1]);
        assert_eq!(Pair("a".to_owned(), 0).validate().unwrap_err(), both[1..]);
        assert_eq!(Pair("a".to_owned(), 1).validate(), Ok(()));
    }

    #[test]
    fn each_key_is_enforced_alone() {
        assert_eq!(each_alone("ab", "ab", "ab", 2, 2), Ok(()));
        for (reported, violation) in [
            (
                each_alone("a", "ab", "ab", 2, 2),
                "'0': too short: minimum length is 2, got 1",
            ),
            (
                each_alone("ab", "abc", "ab", 2, 2),
                "'1': too long: maximum length is 2, got 3",
            ),
            (
                each_alone("ab", "ab", "AB", 2, 2),
                "'2': does not match pattern '^[a-z]+$'",
            ),
            (
                each_alone("ab", "ab", "ab", 1, 2),
                "'3': too small: minimum is 2, got 1",
            ),
            (
                each_alone("ab", "ab", "ab", 2, 3),
                "'4': too large: maximum is 2, got 3",
            ),
        ] {
            assert_eq!(reported.unwrap_err(), vec![violation.to_owned()]);
        }
    }

    #[test]
    fn the_five_keys_are_enforced_together() {
        assert_eq!(Together("abc".to_owned(), 9).validate(), Ok(()));
        assert_eq!(
            Together("A".to_owned(), 0).validate().unwrap_err(),
            vec![
                "'0': too short: minimum length is 2, got 1".to_owned(),
                "'0': does not match pattern '^[a-z]+$'".to_owned(),
                "'1': too small: minimum is 1, got 0".to_owned(),
            ]
        );
        assert_eq!(
            Together("abcd".to_owned(), 10).validate().unwrap_err(),
            vec![
                "'0': too long: maximum length is 3, got 4".to_owned(),
                "'1': too large: maximum is 9, got 10".to_owned(),
            ]
        );
    }

    #[test]
    fn serde_reads_what_a_slots_bound_refuses_and_the_wire_is_unchanged() {
        let read: Pair = serde_json::from_str(r#"["",0]"#).unwrap();
        assert_eq!(read, Pair(String::new(), 0));
        assert_eq!(
            serde_json::to_string(&Pair("a".to_owned(), 1)).unwrap(),
            r#"["a",1]"#
        );
    }

    #[test]
    fn the_one_slot_of_a_struct_is_checked_under_its_position() {
        assert_eq!(
            Lone(String::new()).validate().unwrap_err(),
            vec!["'0': too short: minimum length is 1, got 0".to_owned()]
        );
        assert_eq!(Lone("a".to_owned()).validate(), Ok(()));
        let read: Lone = serde_json::from_str(r#""""#).unwrap();
        assert_eq!(read, Lone(String::new()));
        assert_eq!(
            serde_json::to_string(&Lone("a".to_owned())).unwrap(),
            r#""a""#
        );
    }

    #[test]
    fn a_slot_is_reached_through_an_option_and_a_list_is_counted() {
        assert_eq!(
            Wrapped(Some("a".to_owned()), vec!["long enough".to_owned()], true)
                .validate()
                .unwrap_err(),
            vec![
                "'0': too short: minimum length is 2, got 1".to_owned(),
                "'1': too short: minimum length is 2, got 1".to_owned(),
            ]
        );
        assert_eq!(
            Wrapped(None, vec!["a".to_owned(), "b".to_owned()], true).validate(),
            Ok(())
        );
    }

    #[test]
    fn a_generic_tuple_struct_validates_at_its_declared_default() {
        assert_eq!(
            Labelled::<String>(String::new(), "x".to_owned())
                .validate()
                .unwrap_err(),
            vec!["'0': too short: minimum length is 1, got 0".to_owned()]
        );
    }

    #[test]
    fn the_helpers_are_named_by_the_slots_position() {
        assert_eq!(
            pair_schema::validate_0_value("").unwrap_err(),
            vec!["'0': too short: minimum length is 1, got 0".to_owned()]
        );
        assert_eq!(pair_schema::validate_1_value(&1), Ok(()));
        let refused = pair_schema::deserialize_1(serde_json::json!(0_u32)).unwrap_err();
        assert_eq!(refused.to_string(), "'1': too small: minimum is 1, got 0");
        assert_eq!(
            pair_schema::deserialize_0(serde_json::json!("a")).unwrap(),
            "a"
        );
    }

    #[test]
    fn the_readme_shows_what_validate_reports_for_a_slot() {
        let readme = include_str!("../../README.md");
        let reported = Pair(String::new(), 0).validate();
        reported.as_ref().unwrap_err();
        let shown = format!("// {reported:?}");
        assert!(
            readme.contains(&shown),
            "the README no longer shows {shown}"
        );
        let wire = serde_json::to_string(&Pair("a".to_owned(), 1)).unwrap();
        assert!(readme.contains(&wire), "the README no longer shows {wire}");
    }

    #[cfg(feature = "zod")]
    #[test]
    fn the_zod_schema_writes_each_check_on_the_slots_member_of_the_tuple() {
        let pair = Pair::zod_schema();
        assert!(
            pair.contains(
                "z.tuple([z.string().min(1, { error: (issue) => `too short: minimum length is 1, \
                 got ${String(issue.input).length}` }), z.number().int().min(1, { error: (issue) \
                 => `too small: minimum is 1, got ${String(issue.input)}` })])"
            ),
            "Got:\n{pair}"
        );
        // The statement closes on the bound: the raw `const` beside TypeScript, and the exported
        // one, after the examples it holds, without it.
        let lone = Lone::zod_schema();
        #[cfg(feature = "typescript")]
        let closed = ";";
        #[cfg(not(feature = "typescript"))]
        let closed = ".meta({\n  examples: []\n});";
        assert!(
            lone.contains(&format!(
                " = z.string().min(1, {{ error: (issue) => `too short: minimum length is 1, got \
                 ${{String(issue.input).length}}` }}){closed}"
            )),
            "Got:\n{lone}"
        );
        let wrapped = Wrapped::zod_schema();
        assert!(
            wrapped.contains(
                "z.tuple([z.nullable(z.string().min(2, { error: (issue) => `too short: minimum \
                 length is 2, got ${String(issue.input).length}` })), z.array(z.string()).min(2, \
                 { error: (issue) => `too short: minimum length is 2, got \
                 ${Array.isArray(issue.input) ? issue.input.length : 0}` }), z.boolean()])"
            ),
            "Got:\n{wrapped}"
        );
    }

    #[cfg(feature = "jsonschema")]
    #[test]
    fn the_json_schema_writes_each_check_on_the_slots_member_of_the_tuple() {
        assert_eq!(
            Pair::json_schema(),
            serde_json::json!({
                "type": "array",
                "prefixItems": [
                    { "type": "string", "minLength": 1_u32 },
                    { "type": "integer", "minimum": 1.0_f64 }
                ],
                "items": false,
                "minItems": 2_u32,
                "maxItems": 2_u32
            })
        );
        assert_eq!(
            Lone::json_schema(),
            serde_json::json!({ "type": "string", "minLength": 1_u32 })
        );
        assert_eq!(
            Together::json_schema()["prefixItems"],
            serde_json::json!([
                { "type": "string", "minLength": 2_u32, "maxLength": 3_u32, "pattern": "^[a-z]+$" },
                { "type": "integer", "minimum": 1.0_f64, "maximum": 9.0_f64 }
            ])
        );
        assert_eq!(
            Wrapped::json_schema()["prefixItems"],
            serde_json::json!([
                { "anyOf": [{ "type": "string", "minLength": 2_u32 }, { "type": "null" }] },
                { "type": "array", "items": { "type": "string" }, "minItems": 2_u32 },
                { "type": "boolean" }
            ])
        );
    }

    #[cfg(feature = "typescript")]
    #[test]
    fn the_typescript_type_is_the_tuple_it_was() {
        let pair = Pair::ts_definition();
        assert!(
            pair.contains("export type Pair = [string, number];"),
            "Got:\n{pair}"
        );
        let lone = Lone::ts_definition();
        assert!(lone.contains("export type Lone = string;"), "Got:\n{lone}");
    }
}
