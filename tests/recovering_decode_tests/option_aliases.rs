//! `from_value_with` on a type that holds an alias of an `Option` where serde reads an absent key
//! as `None`: a field under `skip_serializing_if` alone, and the content of a variant under a tag
//! and a content key. The alias that declares a type parameter is read whole, as one that declares
//! none is walked as the type it names, and neither lists the absent key as missing.

use serde::{Deserialize, Serialize};
use serde_json::json;
use sounded_schema::DecodeWith as _;
use tixschema::model_schema;

/// An alias of an `Option`.
#[model_schema()]
pub type Fathoms = Option<u32>;

/// An alias of an `Option` that declares a type parameter.
#[model_schema(default_types(T = String))]
pub type Hapless<T> = Option<T>;

/// Fields of each alias, their keys left out for `None`.
#[model_schema(enable = ["decode_with"])]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Sounded {
    #[serde(skip_serializing_if = "Option::is_none")]
    depth: Fathoms,
    #[serde(skip_serializing_if = "Option::is_none")]
    mark: Hapless<String>,
}

/// The content of each variant an alias of an `Option`, under a tag and a content key.
#[model_schema(enable = ["decode_with"])]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", content = "body")]
enum Heave {
    Depth(Hapless<u32>),
    Line(Fathoms),
}

/// The decode reads each payload as serde reads it, with no issue for the callback.
#[test]
fn an_absent_key_of_an_alias_of_an_option_is_read_as_none_as_serde_reads_it() {
    for stored in [json!({}), json!({ "depth": 5_u32, "mark": "a" })] {
        assert_eq!(
            Sounded::from_value_with(stored.clone(), |_raw, _found| {
                sounded_schema::Verdict::Reject
            })
            .unwrap(),
            serde_json::from_value::<Sounded>(stored).unwrap()
        );
    }
    for stored in [
        json!({ "kind": "Depth" }),
        json!({ "kind": "Line" }),
        json!({ "kind": "Depth", "body": 4_u32 }),
    ] {
        assert_eq!(
            Heave::from_value_with(stored.clone(), |_raw, _found| {
                heave_schema::Verdict::Reject
            })
            .unwrap(),
            serde_json::from_value::<Heave>(stored).unwrap()
        );
    }
}
