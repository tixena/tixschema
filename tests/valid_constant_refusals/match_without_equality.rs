//! A constant pattern needs structural equality: an enum that derives no `PartialEq` and `Eq`
//! has constants for expressions only.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SessionRole {
    Auditor,
    DntiTechnician,
}

fn code(role: &SessionRole) -> u8 {
    match *role {
        SessionRole::DntiTechnician => 1,
        SessionRole::Auditor => 2,
    }
}

fn main() {
    let _code = code(&SessionRole::Auditor);
}
