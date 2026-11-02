//! A member a `#[cfg]` leaves out of the build is left out of what `validate()` and the read
//! write for it, so its type need not exist in that build.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A held model with a rule.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Tag {
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

/// A model no build declares.
#[cfg(any())]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Unbuilt {
    pub label: String,
}

/// A member under a `cfg` no build turns on, typed with a model no build declares, beside one
/// every build holds.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Shelved {
    pub tag: Tag,
    #[cfg(any())]
    pub unbuilt: Unbuilt,
}

#[test]
fn validate_holds_the_members_the_build_keeps() {
    assert_eq!(
        Shelved {
            tag: Tag {
                label: String::new()
            }
        }
        .validate(),
        Err(vec![
            "'tag.label': too short: minimum length is 1, got 0".to_owned()
        ])
    );
}

#[cfg(feature = "serde")]
#[test]
fn the_read_refuses_through_the_members_the_build_keeps() {
    assert_eq!(
        serde_json::from_str::<Shelved>(r#"{"tag":{"label":""}}"#)
            .unwrap_err()
            .to_string(),
        "'tag.label': too short: minimum length is 1, got 0"
    );
}
