//! Models of each shape, whose draft and valid type are held to the same rules.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "kind", content = "value")]
pub enum Adjacent {
    Card {
        #[model_schema_prop(minLength = 4)]
        number: String,
    },
    Cash,
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Audit {
    pub revision: u32,
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Colour {
    Green,
    Red,
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Crate<T> {
    pub items: Vec<T>,
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum External {
    Card {
        #[model_schema_prop(minLength = 4)]
        number: String,
    },
    Cash,
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "kind")]
pub enum Internal {
    Card {
        #[model_schema_prop(minLength = 4)]
        number: String,
    },
    Cash,
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum Loose {
    Card {
        #[model_schema_prop(minLength = 4)]
        number: String,
    },
    Count(u32),
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Marker;

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Note {
    #[serde(flatten)]
    pub audit: Audit,
    #[model_schema_prop(minLength = 1)]
    pub text: String,
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Pair(#[model_schema_prop(minimum = 1)] pub u32, pub u32);

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Tree {
    pub children: Vec<Tree>,
    #[model_schema_prop(minLength = 1)]
    pub name: String,
}
