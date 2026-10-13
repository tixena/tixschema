//! A type that declares no bound and no check publishes no `validate()`: one written by hand
//! beside it builds, and a holder runs it.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Loose {
    pub open: bool,
}

impl Loose {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        if self.open {
            return Err(vec!["a loose parcel is closed".to_owned()]);
        }
        Ok(())
    }
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Shelf {
    pub loose: Loose,
}

#[test]
fn a_validate_written_by_hand_builds_beside_a_type_with_no_check() {
    let held = Shelf {
        loose: Loose { open: true },
    };
    assert_eq!(
        held.loose.validate().unwrap_err(),
        ["a loose parcel is closed"]
    );
    assert_eq!(Loose { open: false }.validate(), Ok(()));
    #[cfg(all(
        feature = "serde",
        any(feature = "typescript", feature = "zod", feature = "jsonschema")
    ))]
    assert_eq!(
        held.validate().unwrap_err(),
        ["'loose': a loose parcel is closed"]
    );
}
