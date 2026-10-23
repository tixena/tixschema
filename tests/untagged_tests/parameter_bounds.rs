//! A bound on a member of an untagged enum whose type names the enum's own type parameter. The
//! member is held to it as it is read, as a member that names no parameter is, and the read asks
//! of the parameter what serde's derive asks of the enum as it is written.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A count on a list of pairs that names the enum's parameter, ahead of a variant no object reads.
#[model_schema(default_types(Key = String))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ProbeChosen<Key> {
    Listed {
        #[model_schema_prop(minLength = 1)]
        listed: Vec<(u32, Key)>,
    },
    Text(String),
}

/// The same member ahead of a variant that reads the list its count refuses.
#[model_schema(default_types(Key = String))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ProbeChosenOrOpen<Key> {
    Listed {
        #[model_schema_prop(minLength = 1)]
        listed: Vec<(u32, Key)>,
    },
    Open {
        listed: Vec<(u32, Key)>,
    },
}

/// The twin of [`ProbeChosenOrOpen`] whose member names no parameter.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ProbeChosenOrOpenText {
    Listed {
        #[model_schema_prop(minLength = 1)]
        listed: Vec<(u32, String)>,
    },
    Open {
        listed: Vec<(u32, String)>,
    },
}

/// A slot that names the parameter, under a count, ahead of a slot that reads what it refuses.
#[model_schema(default_types(Key = String))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ProbeSlotted<Key> {
    Listed(#[model_schema_prop(minLength = 1)] Vec<Key>),
    Open(Vec<Key>),
}

/// An optional member that names the parameter: its key is read as absent through the `default`
/// written beside its hook, which asks no `Default` of the parameter.
#[model_schema(default_types(Key = String))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ProbeMaybeListed<Key> {
    Listed {
        #[model_schema_prop(minLength = 1)]
        #[serde(skip_serializing_if = "Option::is_none")]
        listed: Option<Vec<Key>>,
        tag: u8,
    },
    Text(String),
}

/// A filling with no `Default`, which serde's derive asks of no parameter of the enums above.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProbeUndefaulted(pub u8);

/// What serde answers for `json` read as `T`, its refusal as text.
fn read<T>(json: &str) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_str(json).map_err(|refused: serde_json::Error| refused.to_string())
}

#[test]
fn a_counted_member_that_names_the_parameter_is_read_and_held_to_its_count() {
    assert_eq!(
        read::<ProbeChosen<String>>(r#"{"listed":[[7,"a"]]}"#),
        Ok(ProbeChosen::Listed {
            listed: vec![(7, "a".to_owned())]
        })
    );
    assert_eq!(
        read::<ProbeChosen<String>>(r#""a""#),
        Ok(ProbeChosen::Text("a".to_owned()))
    );
    // serde's untagged read drops each variant's own refusal, as it does for a member that names
    // no parameter; the hook and `validate()` answer the count's sentence.
    assert_eq!(
        read::<ProbeChosen<String>>(r#"{"listed":[]}"#),
        Err("data did not match any variant of untagged enum ProbeChosen".to_owned())
    );
    let count = "'listed': too short: minimum length is 1, got 0".to_owned();
    assert_eq!(
        probe_chosen_schema::deserialize_listed_listed::<_, Vec<(u32, String)>>(
            &mut serde_json::Deserializer::from_str("[]")
        )
        .map_err(|refused| refused.to_string()),
        Err(count.clone())
    );
    assert_eq!(
        ProbeChosen::<String>::Listed { listed: vec![] }.validate(),
        Err(vec![count])
    );
}

#[test]
fn a_list_the_count_refuses_is_read_as_the_next_variant_as_with_no_parameter_named() {
    assert_eq!(
        read::<ProbeChosenOrOpen<String>>(r#"{"listed":[[7,"a"]]}"#),
        Ok(ProbeChosenOrOpen::Listed {
            listed: vec![(7, "a".to_owned())]
        })
    );
    assert_eq!(
        read::<ProbeChosenOrOpen<String>>(r#"{"listed":[]}"#),
        Ok(ProbeChosenOrOpen::Open { listed: vec![] })
    );
    assert_eq!(
        read::<ProbeChosenOrOpenText>(r#"{"listed":[[7,"a"]]}"#),
        Ok(ProbeChosenOrOpenText::Listed {
            listed: vec![(7, "a".to_owned())]
        })
    );
    assert_eq!(
        read::<ProbeChosenOrOpenText>(r#"{"listed":[]}"#),
        Ok(ProbeChosenOrOpenText::Open { listed: vec![] })
    );
    assert_eq!(
        read::<ProbeSlotted<ProbeUndefaulted>>("[7]"),
        Ok(ProbeSlotted::Listed(vec![ProbeUndefaulted(7)]))
    );
    assert_eq!(
        read::<ProbeSlotted<ProbeUndefaulted>>("[]"),
        Ok(ProbeSlotted::Open(vec![]))
    );
}

#[test]
fn an_optional_counted_member_asks_no_default_of_the_parameter() {
    assert_eq!(
        read::<ProbeMaybeListed<ProbeUndefaulted>>(r#"{"listed":[7],"tag":1}"#),
        Ok(ProbeMaybeListed::Listed {
            listed: Some(vec![ProbeUndefaulted(7)]),
            tag: 1
        })
    );
    assert_eq!(
        read::<ProbeMaybeListed<ProbeUndefaulted>>(r#"{"tag":1}"#),
        Ok(ProbeMaybeListed::Listed {
            listed: None,
            tag: 1
        })
    );
    assert_eq!(
        read::<ProbeMaybeListed<ProbeUndefaulted>>(r#"{"listed":[],"tag":1}"#),
        Err("data did not match any variant of untagged enum ProbeMaybeListed".to_owned())
    );
}
