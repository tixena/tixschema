//! A check at a generic holder's parameter on a variant member or a tuple slot, and aliases of a
//! list, an `Option` or a tuple over a type declared below them.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(default_types(IdType = String), check = ["withRecords"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Vault<IdType> {
    pub owner: IdType,
    pub records: Vec<String>,
}

#[model_schema(default_types(IdType = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Safe<IdType> {
    Named {
        #[model_schema_prop(check = ["withRecords"])]
        vault: Vault<IdType>,
    },
    Single(#[model_schema_prop(check = ["withRecords"])] Vault<IdType>),
}

#[model_schema(default_types(IdType = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Open<IdType> {
    Named {
        #[model_schema_prop(check = ["withRecords"])]
        vault: Vault<IdType>,
    },
    Single(#[model_schema_prop(check = ["withRecords"])] Vault<IdType>),
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Hasp {
    pub label: String,
    #[serde(flatten)]
    pub open: Open<String>,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Latch {
    pub label: String,
    #[serde(flatten)]
    pub open: Open<u32>,
}

#[model_schema(default_types(IdType = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Tagged<IdType> {
    Named {
        #[model_schema_prop(check = ["withRecords"])]
        vault: Vault<IdType>,
    },
}

#[model_schema(default_types(IdType = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "body")]
pub enum Adjacent<IdType> {
    Single(#[model_schema_prop(check = ["withRecords"])] Vault<IdType>),
}

#[model_schema(default_types(IdType = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Locker<IdType>(
    #[model_schema_prop(check = ["withRecords"])] pub Vault<IdType>,
    pub String,
);

#[model_schema()]
pub type Rows = Vec<Later>;

#[model_schema()]
pub type Maybe = Option<Later>;

#[model_schema()]
pub type Pair = (Later, u8);

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Later {
    pub name: String,
}

fn with_records<IdType>(value: &Vault<IdType>) -> Result<(), Vec<String>> {
    if value.records.is_empty() {
        return Err(vec!["'records': a vault holds a record".to_owned()]);
    }
    Ok(())
}
