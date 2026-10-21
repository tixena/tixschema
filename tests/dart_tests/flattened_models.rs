//! The models of the flattened Dart reads `dart_tests/flattened.rs` pins and
//! `service_schema_emitted_client_tests/run_dart_flattened.rs` runs: a map flattened after a
//! struct, beside a member of its holder's own and in a variant under a tag key, a map that is the
//! one flattened member, a map flattened ahead of a struct, two flattened maps, and a map
//! flattened after a member that takes none of the keys it reads. serde reads the members in the
//! order declared: a struct takes the keys it reads, and a map is handed every key no member ahead
//! of it took.

use alloc::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::model_schema;

/// What every other struct is read from.
pub const PAYLOAD: &str = r#"{"label":"n","north":2}"#;

/// What [`Clamping`] is read from.
pub const CLAMPING: &str = r#"{"label":"n","north":2,"note":"a"}"#;

/// What [`Fastening`] is read from.
pub const FASTENING: &str = r#"{"north":2,"note":"a"}"#;

/// What [`Tethering`] is read from.
pub const TETHERING: &str = r#"{"kind":"Held","label":"n","north":2}"#;

/// What [`Stowing`] is read from.
pub const STOWING: &str = r#"{"north":2,"south":3}"#;

/// The member the structs below flatten.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Marking {
    pub label: String,
}

/// A map flattened after a struct, where serde hands it the keys the struct did not take.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Coupling {
    #[serde(flatten)]
    pub marking: Marking,
    #[serde(flatten)]
    pub rest: BTreeMap<String, u8>,
}

/// [`Coupling`] with a member of its own between the two flattened ones.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Clamping {
    #[serde(flatten)]
    pub marking: Marking,
    pub note: String,
    #[serde(flatten)]
    pub rest: BTreeMap<String, u8>,
}

/// A map, the one flattened member, beside a member of its holder's own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Fastening {
    pub note: String,
    #[serde(flatten)]
    pub rest: BTreeMap<String, u8>,
}

/// A map flattened ahead of a struct, where serde hands it every key, the struct's among them.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Hoisting {
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
    #[serde(flatten)]
    pub marking: Marking,
}

/// Two flattened maps, where serde hands each the same keys.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Stowing {
    #[serde(flatten)]
    pub first: BTreeMap<String, u8>,
    #[serde(flatten)]
    pub second: BTreeMap<String, u8>,
}

/// A struct that names a function on itself, which serde reads as a value.
#[model_schema(preprocess = ["asGiven"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Ticketing {
    pub label: String,
}

/// A struct with a flattened member of its own, which serde reads as a map.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Bracing {
    #[serde(flatten)]
    pub marking: Marking,
}

/// A map flattened after a struct that names a function on itself, where serde hands it every key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Lacing {
    #[serde(flatten)]
    pub issued: Ticketing,
    #[serde(flatten)]
    pub rest: BTreeMap<String, Value>,
}

/// A map flattened after a member that names a function, where serde hands it every key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Rigging {
    #[model_schema_prop(preprocess = ["asGiven"])]
    #[serde(flatten)]
    pub marking: Marking,
    #[serde(flatten)]
    pub rest: BTreeMap<String, Value>,
}

/// A map flattened after a struct with a flattened member of its own, where serde hands it every
/// key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Belaying {
    #[serde(flatten)]
    pub bracing: Bracing,
    #[serde(flatten)]
    pub rest: BTreeMap<String, Value>,
}

/// A map flattened after a struct in a variant under a tag key, where serde hands it the keys
/// neither the tag nor the struct took.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum Tethering {
    Held {
        #[serde(flatten)]
        marking: Marking,
        #[serde(flatten)]
        rest: BTreeMap<String, u8>,
    },
}

/// The payload as it was handed.
const fn as_given(payload: Value) -> Value {
    payload
}

/// The text of `value` as serde writes it.
fn written<T>(value: &T) -> String
where
    T: Serialize,
{
    serde_json::to_string(value).unwrap()
}

/// A map after a struct holds the keys the struct did not take, and a map ahead of a struct, and
/// after a member that takes no key, holds every key. serde writes a key two members hold twice.
#[test]
fn serde_hands_a_flattened_map_the_keys_no_member_ahead_of_it_took() {
    let coupling: Coupling = serde_json::from_str(PAYLOAD).unwrap();
    assert_eq!(written(&coupling.rest), r#"{"north":2}"#);
    let clamping: Clamping = serde_json::from_str(CLAMPING).unwrap();
    assert_eq!(written(&clamping.rest), r#"{"north":2}"#);
    let fastening: Fastening = serde_json::from_str(FASTENING).unwrap();
    assert_eq!(written(&fastening.rest), r#"{"north":2}"#);
    let Tethering::Held { marking, rest } = serde_json::from_str(TETHERING).unwrap();
    assert_eq!(marking.label, "n");
    assert_eq!(written(&rest), r#"{"north":2}"#);
    let hoisting: Hoisting = serde_json::from_str(PAYLOAD).unwrap();
    assert_eq!(written(&hoisting.extra), PAYLOAD);
    assert_eq!(written(&hoisting), r#"{"label":"n","north":2,"label":"n"}"#);
    let lacing: Lacing = serde_json::from_str(PAYLOAD).unwrap();
    assert_eq!(written(&lacing.rest), PAYLOAD);
    let rigging: Rigging = serde_json::from_str(PAYLOAD).unwrap();
    assert_eq!(written(&rigging.rest), PAYLOAD);
    let belaying: Belaying = serde_json::from_str(PAYLOAD).unwrap();
    assert_eq!(written(&belaying.rest), PAYLOAD);
    let stowing: Stowing = serde_json::from_str(STOWING).unwrap();
    assert_eq!(
        written(&stowing),
        r#"{"north":2,"south":3,"north":2,"south":3}"#
    );
}
