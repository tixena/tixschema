//! A rule of a model a member of an untagged enum holds chooses no variant: serde takes the first
//! variant whose shape reads the payload, and the enum's read then refuses the value, where the
//! JSON Schema's `anyOf` admits it by a later branch. These are the README's `Picker` and
//! `Chooser`, under names of their own.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[cfg(feature = "jsonschema")]
use super::documents::admits;

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProbeBale {
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProbeLoose {
    pub label: String,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ProbePicker {
    A(ProbeBale),
    B(ProbeLoose),
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProbeCount(#[model_schema_prop(minimum = 1)] pub u32);

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ProbeChooser {
    A(ProbeCount),
    B(u32),
}

/// What serde answers for `json` read as `T`, its refusal as text.
fn read<T>(json: &str) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_str(json).map_err(|refused: serde_json::Error| refused.to_string())
}

#[test]
fn a_held_models_rule_chooses_no_variant() {
    assert_eq!(
        read::<ProbePicker>(r#"{"label":"x"}"#),
        Ok(ProbePicker::A(ProbeBale {
            label: "x".to_owned()
        }))
    );
    let empty = ProbePicker::B(ProbeLoose {
        label: String::new(),
    });
    assert_eq!(serde_json::to_string(&empty).unwrap(), r#"{"label":""}"#);
    assert_eq!(
        read::<ProbePicker>(r#"{"label":""}"#),
        Err("'label': too short: minimum length is 1, got 0".to_owned())
    );

    assert_eq!(
        read::<ProbeChooser>("1"),
        Ok(ProbeChooser::A(ProbeCount(1)))
    );
    assert_eq!(serde_json::to_string(&ProbeChooser::B(0)).unwrap(), "0");
    assert_eq!(
        read::<ProbeChooser>("0"),
        Err("too small: minimum is 1, got 0".to_owned())
    );
}

/// The JSON Schema admits by its second branch what serde refuses.
#[cfg(feature = "jsonschema")]
#[test]
fn the_json_schema_admits_by_a_later_branch_what_serde_refuses() {
    let picker = ProbePicker::json_schema();
    let empty = serde_json::json!({ "label": "" });
    assert!(!admits(&picker["anyOf"][0], &empty), "{picker}");
    assert!(admits(&picker["anyOf"][1], &empty), "{picker}");
    assert!(admits(&picker, &empty), "{picker}");
    assert!(
        admits(&ProbeChooser::json_schema(), &serde_json::json!(0_u32)),
        "{}",
        ProbeChooser::json_schema()
    );
}

#[test]
fn the_readme_states_the_answers_of_a_held_models_rule() {
    let readme = include_str!("../../README.md");
    for shown in [
        "`{\"label\":\"\"}` read as `Picker` is `Picker::A`, the first variant whose shape reads \
         it, and serde refuses it with `'label': too short: minimum length is 1, got 0`.",
        "`Picker$Schema.safeParse({ label: \"\" })` answers `{\"success\":true,\"data\":{\"label\":\"\"}}`",
        "is refused with `too small: minimum is 1, got 0`, where Zod reads it as `B`.",
    ] {
        assert!(readme.contains(shown), "the README no longer shows {shown}");
    }
}
