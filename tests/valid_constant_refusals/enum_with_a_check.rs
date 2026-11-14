//! An enum that names a check of its own has no constants: the check could refuse a unit variant.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(check = ["with_a_technician"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum SessionRole {
    Auditor,
    DntiTechnician,
}

fn with_a_technician(_value: &SessionRoleDraft) -> Result<(), Vec<String>> {
    Ok(())
}

fn main() {
    let _role: SessionRole = SessionRole::DntiTechnician;
}
