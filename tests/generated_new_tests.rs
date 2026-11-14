//! The `new` every struct's valid type has, which answers what `validate()` answers.

#![cfg(feature = "serde")]

#[cfg(test)]
mod tests {
    use core::marker::PhantomData;
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    pub struct TimeOfDay {
        #[model_schema_prop(maximum = 23)]
        pub hour: u8,
        #[model_schema_prop(maximum = 59)]
        pub minute: u8,
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct Point {
        pub x: i32,
        pub y: i32,
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    pub struct CalculatedFlag {
        pub advanced: bool,
        #[model_schema_prop(literal = true)]
        pub is_calculated: bool,
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    pub struct Versioned {
        #[model_schema_prop(literal = "kind")]
        pub kind: String,
        pub label: String,
        #[model_schema_prop(literal = 1.5)]
        pub ratio: f64,
        #[model_schema_prop(literal = 3)]
        pub version: u8,
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct OptionalMark {
        pub label: String,
        #[model_schema_prop(literal = "mark")]
        #[serde(skip_serializing_if = "Option::is_none", default)]
        pub mark: Option<String>,
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct Pair(pub u32, pub u32);

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct Wrapped(pub String);

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct Ping;

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct Tally<T> {
        #[model_schema_prop(minimum = 1)]
        pub count: u32,
        pub held: T,
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct Marked<T> {
        pub label: String,
        #[serde(skip)]
        pub marker: PhantomData<T>,
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct Reserved {
        pub new: u32,
        pub r#type: String,
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct Skipped {
        #[serde(skip)]
        pub cached: u32,
        pub label: String,
    }

    #[model_schema(new = true)]
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct Wide {
        pub a: u8,
        pub b: u8,
        pub c: u8,
        pub d: u8,
        pub e: u8,
        pub f: u8,
        pub g: u8,
        pub h: u8,
        pub i: u8,
        pub j: u8,
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct Seven {
        pub a: u8,
        pub b: u8,
        pub c: u8,
        pub d: u8,
        pub e: u8,
        pub f: u8,
        pub g: u8,
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct Eight {
        pub a: u8,
        pub b: u8,
        pub c: u8,
        pub d: u8,
        pub e: u8,
        pub f: u8,
        pub g: u8,
        pub h: u8,
    }

    impl Eight {
        pub fn new(sum: u8) -> Result<Self, Vec<String>> {
            EightDraft {
                a: sum,
                b: 0,
                c: 0,
                d: 0,
                e: 0,
                f: 0,
                g: 0,
                h: 0,
            }
            .validate()
        }
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct EightWithALiteral {
        pub a: u8,
        pub b: u8,
        pub c: u8,
        pub d: u8,
        pub e: u8,
        pub f: u8,
        #[model_schema_prop(literal = true)]
        pub fixed: bool,
        pub g: u8,
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
    pub enum Unit {
        Off,
        On,
    }

    #[model_schema(new = false)]
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct Handwritten {
        pub hour: u8,
        pub minute: u8,
    }

    impl Handwritten {
        pub fn new(hour: u8, minute: u8) -> Result<Self, Vec<String>> {
            HandwrittenDraft { hour, minute }.validate()
        }
    }

    #[model_schema(new = true)]
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct Written {
        pub hour: u8,
    }

    #[test]
    fn new_false_leaves_the_authors_own_new() {
        assert_eq!(Handwritten::new(1, 2).unwrap().minute, 2);
        assert_eq!(Written::new(1).unwrap().hour, 1);
    }

    #[test]
    fn a_struct_with_a_rule_builds_what_its_draft_validates() {
        let built = TimeOfDay::new(12, 0).unwrap();
        let validated = TimeOfDayDraft {
            hour: 12,
            minute: 0,
        }
        .validate()
        .unwrap();
        assert_eq!(built, validated);
    }

    #[test]
    fn a_broken_rule_answers_the_messages_validate_answers() {
        let refused = TimeOfDay::new(24, 60).unwrap_err();
        let validated = TimeOfDayDraft {
            hour: 24,
            minute: 60,
        }
        .validate()
        .unwrap_err();
        assert_eq!(refused, validated);
        assert_eq!(refused.len(), 2);
    }

    #[test]
    fn a_struct_with_no_rule_answers_a_result_too() {
        let built: Result<Point, Vec<String>> = Point::new(1, 2);
        assert_eq!(built.unwrap(), PointDraft { x: 1, y: 2 }.into());
    }

    #[test]
    fn a_boolean_literal_is_written_and_is_no_parameter() {
        let flag = CalculatedFlag::new(false).unwrap();
        assert!(flag.is_calculated);
        assert!(!flag.advanced);
    }

    #[test]
    fn integer_number_and_string_literals_are_written() {
        let value = Versioned::new("x".to_owned()).unwrap();
        assert_eq!(value.version, 3);
        assert!((value.ratio - 1.5).abs() < f64::EPSILON);
        assert_eq!(value.kind, "kind");
        assert_eq!(value.label, "x");
    }

    #[test]
    fn an_optional_literal_stays_a_parameter() {
        let value = OptionalMark::new("x".to_owned(), None).unwrap();
        assert_eq!(value.mark, None);
        OptionalMark::new("x".to_owned(), Some("other".to_owned())).unwrap_err();
        OptionalMark::new("x".to_owned(), Some("mark".to_owned())).unwrap();
    }

    #[test]
    fn a_tuple_struct_takes_one_value_per_slot() {
        assert_eq!(Pair::new(1, 2).unwrap().0, 1);
        assert_eq!(Wrapped::new("a".to_owned()).unwrap().0, "a");
    }

    #[test]
    fn a_unit_struct_takes_nothing() {
        assert_eq!(Ping::new().unwrap(), Ping);
    }

    #[test]
    fn a_generic_struct_takes_its_parameters() {
        let held = Tally::<String>::new(2, "a".to_owned()).unwrap();
        assert_eq!(held.count, 2);
        Tally::new(0, 1_u8).unwrap_err();
        Marked::<u8>::new("a".to_owned(), PhantomData).unwrap();
    }

    #[test]
    fn a_reserved_or_skipped_member_is_a_parameter() {
        assert_eq!(Reserved::new(4, "a".to_owned()).unwrap().r#type, "a");
        assert_eq!(Skipped::new(7, "a".to_owned()).unwrap().cached, 7);
    }

    #[test]
    fn a_wide_struct_takes_each_field() {
        assert_eq!(Wide::new(1, 2, 3, 4, 5, 6, 7, 8, 9, 10).unwrap().j, 10);
    }

    #[test]
    fn seven_parameters_are_generated() {
        assert_eq!(Seven::new(1, 2, 3, 4, 5, 6, 7).unwrap().g, 7);
    }

    #[test]
    fn eight_parameters_are_not_generated_by_default() {
        assert_eq!(Eight::new(3).unwrap().a, 3);
    }

    #[test]
    fn a_filled_literal_is_no_parameter_toward_the_width() {
        let value = EightWithALiteral::new(1, 2, 3, 4, 5, 6, 7).unwrap();
        assert!(value.fixed);
        assert_eq!(value.g, 7);
    }

    #[test]
    fn an_enum_keeps_its_constants_and_gets_no_constructor() {
        assert_eq!(Unit::On, Unit::On);
        assert_ne!(Unit::On, Unit::Off);
    }
}
