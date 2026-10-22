//! The models a name serde reads beside the one it writes is read under, which the Kotlin and the
//! Dart clients are held to: a variant under its `alias`, under each tagging and in a plain enum,
//! and a member under its `alias`, in a struct the Kotlin plugin reads, in one the Kotlin emitter
//! reads by hand, flattened, and in a variant of each tagging. serde reads each name, writes the
//! one it writes, and refuses a member read under two of its names.

use alloc::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::model_schema;

/// What [`Surging`] is read from: each variant under its name, under each alias and as an entry
/// over `null`, a member under its alias, and a member under its name and its alias, which serde
/// refuses.
pub const SURGING: [&str; 10] = [
    r#"{"Berth":{"label":"n"}}"#,
    r#"{"Pier":{"label":"n"}}"#,
    r#"{"Pier":{"name":"n"}}"#,
    r#"{"Berth":{"label":"n","name":"m"}}"#,
    r#""Calm""#,
    r#""Still""#,
    r#""Slack""#,
    r#"{"Slack":null}"#,
    r#"{"Held":"x"}"#,
    r#"{"Tied":"x"}"#,
];

/// What [`Careening`] is read from: each variant under its name and its alias, a member under its
/// alias, and a member under its name and its alias, which serde refuses.
pub const CAREENING: [&str; 6] = [
    r#"{"kind":"Berth","label":"n"}"#,
    r#"{"kind":"Pier","label":"n"}"#,
    r#"{"kind":"Pier","name":"n"}"#,
    r#"{"kind":"Berth","label":"n","name":"m"}"#,
    r#"{"kind":"Calm"}"#,
    r#"{"kind":"Still"}"#,
];

/// What [`Scending`] is read from: each variant under its name and its alias, and a member under
/// its alias.
pub const SCENDING: [&str; 7] = [
    r#"{"kind":"Berth","data":{"label":"n"}}"#,
    r#"{"kind":"Pier","data":{"label":"n"}}"#,
    r#"{"kind":"Pier","data":{"name":"n"}}"#,
    r#"{"kind":"Calm"}"#,
    r#"{"kind":"Still"}"#,
    r#"{"kind":"Held","data":"x"}"#,
    r#"{"kind":"Tied","data":"x"}"#,
];

/// What [`Swell`] is read from: each variant under its name and each alias, and a name no variant
/// is read under.
pub const SWELL: [&str; 5] = [
    r#""Flood""#,
    r#""Ebb""#,
    r#""Low""#,
    r#""Neap""#,
    r#""High""#,
];

/// What [`Lulling`] is read from: a variant under its name and under its alias, each between
/// spaces its function takes off, and the other variant.
pub const LULLING: [&str; 3] = [r#"" Ebb ""#, r#"" Low ""#, r#""Flood""#];

/// What [`Tiding`] is read from: [`Swell`] under an alias in a list and as the key of a map.
pub const TIDING: &str = r#"{"by_swell":{"Neap":1,"Flood":2},"swells":["Low","Flood"]}"#;

/// What [`Veering`] is read from: a member under its name and under its alias, the two together,
/// which serde reads no variant of, and the other variant.
pub const VEERING: [&str; 4] = [
    r#"{"label":"n"}"#,
    r#"{"name":"n"}"#,
    r#"{"label":"n","name":"m"}"#,
    "3",
];

/// What [`Hawsing`] and [`Gybing`] are read from: a member under its name and under each alias,
/// an optional member under its alias, and a member under two of its names, which serde refuses.
pub const HAWSING: [&str; 5] = [
    r#"{"label":"n"}"#,
    r#"{"name":"n"}"#,
    r#"{"title":"n","remark":"r"}"#,
    r#"{"label":"n","name":"m"}"#,
    r#"{"name":"n","title":"m"}"#,
];

/// What [`Fairleading`] and [`Fothering`] are read from: a member under its alias beside a key no
/// member reads, and under its name and its alias.
pub const FAIRLEADING: [&str; 3] = [
    r#"{"name":"n","north":2}"#,
    r#"{"label":"n","north":2}"#,
    r#"{"label":"n","name":"m","north":2}"#,
];

/// An externally tagged enum with a variant under one alias and under two, and a member under an
/// alias.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Surging {
    #[serde(alias = "Pier")]
    Berth {
        #[serde(alias = "name")]
        label: String,
    },
    #[serde(alias = "Still", alias = "Slack")]
    Calm,
    #[serde(alias = "Tied")]
    Held(String),
}

/// [`Surging`] under a tag alone.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum Careening {
    #[serde(alias = "Pier")]
    Berth {
        #[serde(alias = "name")]
        label: String,
    },
    #[serde(alias = "Still")]
    Calm,
}

/// [`Surging`] under a tag and a content key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data")]
pub enum Scending {
    #[serde(alias = "Pier")]
    Berth {
        #[serde(alias = "name")]
        label: String,
    },
    #[serde(alias = "Still")]
    Calm,
    #[serde(alias = "Tied")]
    Held(String),
}

/// A plain enum with a variant under two aliases.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum Swell {
    #[serde(alias = "Low", alias = "Neap")]
    Ebb,
    Flood,
}

/// [`Swell`] under a function that takes the spaces off its payload: serde reads a variant under
/// its alias from what the function answers.
#[model_schema(preprocess = ["unpadded"])]
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Lulling {
    #[serde(alias = "Low")]
    Ebb,
    Flood,
}

/// [`Swell`] held in a list and as the key of a map.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Tiding {
    pub by_swell: BTreeMap<Swell, u8>,
    pub swells: Vec<Swell>,
}

/// An untagged enum whose variant has a member under an alias.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Veering {
    Berth {
        #[serde(alias = "name")]
        label: String,
    },
    Count(u8),
}

/// A struct the plugin reads, with a member under two aliases and an optional member under one.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Hawsing {
    #[serde(alias = "name", alias = "title")]
    pub label: String,
    #[serde(alias = "remark", default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// [`Hawsing`] with a parameter.
#[model_schema(default_types(LabelType = String))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Gybing<LabelType> {
    #[serde(alias = "name", alias = "title")]
    pub label: LabelType,
    #[serde(alias = "remark", default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// The keys a function was handed, which the members below that name one read.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Pennon {
    pub keys: Vec<String>,
}

/// A member under an alias beside a member that names a function flattened after it: serde hands
/// the function the keys the member did not take.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Fairleading {
    #[serde(alias = "name")]
    pub label: String,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub tally: Pennon,
}

/// [`Hawsing`] flattened ahead of a member that names a function: serde hands the function the
/// keys the struct did not take, an alias of its member among those it took.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Fothering {
    #[serde(flatten)]
    pub hawsing: Hawsing,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub tally: Pennon,
}

/// The keys of the object `payload`, under `keys`.
fn with_its_keys(payload: Value) -> Value {
    let Value::Object(object) = payload else {
        return payload;
    };
    let keys: Vec<Value> = object
        .into_iter()
        .map(|(key, _)| Value::String(key))
        .collect();
    serde_json::json!({ "keys": keys })
}

/// `payload`, a string, without the spaces around it.
fn unpadded(payload: Value) -> Value {
    if let Value::String(text) = payload {
        Value::String(text.trim().to_owned())
    } else {
        payload
    }
}

/// What serde reads of `payload` as `T`, written back, or `refused` where it refuses it.
pub fn written_back<T>(payload: &str) -> String
where
    T: for<'de> Deserialize<'de> + Serialize,
{
    serde_json::from_str::<T>(payload).map_or_else(
        |_| "refused".to_owned(),
        |read| serde_json::to_string(&read).unwrap(),
    )
}

/// What serde reads of each payload: a name it reads beside the one it writes is read as that
/// variant or member, written back under the name serde writes, and a member under two of its
/// names is refused.
#[test]
fn serde_reads_a_variant_and_a_member_under_each_alias() {
    let surging = SURGING.map(written_back::<Surging>);
    assert_eq!(
        surging,
        [
            r#"{"Berth":{"label":"n"}}"#,
            r#"{"Berth":{"label":"n"}}"#,
            r#"{"Berth":{"label":"n"}}"#,
            "refused",
            r#""Calm""#,
            r#""Calm""#,
            r#""Calm""#,
            r#""Calm""#,
            r#"{"Held":"x"}"#,
            r#"{"Held":"x"}"#,
        ]
    );
    let careening = CAREENING.map(written_back::<Careening>);
    assert_eq!(
        careening,
        [
            r#"{"kind":"Berth","label":"n"}"#,
            r#"{"kind":"Berth","label":"n"}"#,
            r#"{"kind":"Berth","label":"n"}"#,
            "refused",
            r#"{"kind":"Calm"}"#,
            r#"{"kind":"Calm"}"#,
        ]
    );
    let scending = SCENDING.map(written_back::<Scending>);
    assert_eq!(
        scending,
        [
            r#"{"kind":"Berth","data":{"label":"n"}}"#,
            r#"{"kind":"Berth","data":{"label":"n"}}"#,
            r#"{"kind":"Berth","data":{"label":"n"}}"#,
            r#"{"kind":"Calm"}"#,
            r#"{"kind":"Calm"}"#,
            r#"{"kind":"Held","data":"x"}"#,
            r#"{"kind":"Held","data":"x"}"#,
        ]
    );
    let swell = SWELL.map(written_back::<Swell>);
    assert_eq!(
        swell,
        [r#""Flood""#, r#""Ebb""#, r#""Ebb""#, r#""Ebb""#, "refused"]
    );
    assert_eq!(
        LULLING.map(written_back::<Lulling>),
        [r#""Ebb""#, r#""Ebb""#, r#""Flood""#]
    );
    assert_eq!(
        written_back::<Tiding>(TIDING),
        r#"{"by_swell":{"Ebb":1,"Flood":2},"swells":["Ebb","Flood"]}"#
    );
    let veering = VEERING.map(written_back::<Veering>);
    assert_eq!(
        veering,
        [r#"{"label":"n"}"#, r#"{"label":"n"}"#, "refused", "3"]
    );
    let hawsing = HAWSING.map(written_back::<Hawsing>);
    let held = [
        r#"{"label":"n"}"#,
        r#"{"label":"n"}"#,
        r#"{"label":"n","note":"r"}"#,
        "refused",
        "refused",
    ];
    assert_eq!(hawsing, held);
    assert_eq!(HAWSING.map(written_back::<Gybing<String>>), held);
    assert_eq!(
        serde_json::from_str::<Hawsing>(HAWSING[3])
            .unwrap_err()
            .to_string(),
        "duplicate field `label` at line 1 column 19"
    );
    let tallied = [
        r#"{"label":"n","keys":["north"]}"#,
        r#"{"label":"n","keys":["north"]}"#,
        "refused",
    ];
    assert_eq!(FAIRLEADING.map(written_back::<Fairleading>), tallied);
    assert_eq!(FAIRLEADING.map(written_back::<Fothering>), tallied);
}
