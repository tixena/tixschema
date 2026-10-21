//! A member serde reads and never writes, `#[serde(default, skip_serializing)]`: in a struct of
//! its own, and beside a flattened member. Each client reads the payload below and writes it back
//! as serde does, without the member.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// The payload each struct is read from: the member serde never writes, held.
pub const PAYLOAD: &str = r#"{"label":"north","spare":3}"#;

/// What serde writes back of [`PAYLOAD`], read as either struct.
pub const WRITTEN: &str = r#"{"label":"north"}"#;

/// A member serde reads and never writes.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Furling {
    pub label: String,
    #[serde(default, skip_serializing)]
    pub spare: Option<u32>,
}

/// The member [`Reefing`] flattens.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Braiding {
    pub label: String,
}

/// [`Furling`]'s member beside a flattened one.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Reefing {
    #[serde(flatten)]
    pub braiding: Braiding,
    #[serde(default, skip_serializing)]
    pub spare: Option<u32>,
}

/// serde reads the member from [`PAYLOAD`] and writes [`WRITTEN`] back, in either struct.
#[test]
fn serde_reads_the_member_and_never_writes_it() {
    let furling: Furling = serde_json::from_str(PAYLOAD).unwrap();
    let reefing: Reefing = serde_json::from_str(PAYLOAD).unwrap();
    assert_eq!(
        furling,
        Furling {
            label: "north".to_owned(),
            spare: Some(3),
        }
    );
    assert_eq!(
        reefing,
        Reefing {
            braiding: Braiding {
                label: "north".to_owned(),
            },
            spare: Some(3),
        }
    );
    assert_eq!(serde_json::to_string(&furling).unwrap(), WRITTEN);
    assert_eq!(serde_json::to_string(&reefing).unwrap(), WRITTEN);
}
