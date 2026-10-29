//! A length written where a list is named, not written: a field typed with a brand over a list or
//! with an alias of one, a brand over a brand over a list, and a generic brand whose declared
//! default is a list.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(no_display)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Slogans(pub Vec<String>);

#[model_schema()]
pub type Mottos = Vec<String>;

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Poster {
    #[model_schema_prop(minLength = 1)]
    pub later: LaterSlogans,
    #[model_schema_prop(minLength = 1)]
    pub mottos: Mottos,
    #[model_schema_prop(minLength = 1, maxLength = 2)]
    pub slogans: Slogans,
}

#[model_schema(no_display)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct LaterSlogans(pub Vec<String>);

#[model_schema(no_display, minLength = 1)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct SomeSlogans(pub Slogans);

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Billboard {
    #[model_schema_prop(maxLength = 2)]
    pub some: SomeSlogans,
}

#[model_schema(no_display, minLength = 1, default_types(T = Vec<String>))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Carried<T>(pub T);

#[cfg(feature = "indexmap")]
#[model_schema()]
pub type Ordered = indexmap::IndexSet<String>;

#[cfg(feature = "indexmap")]
#[model_schema(no_display, minLength = 1)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct SomeOrdered(pub Ordered);

#[cfg(feature = "indexmap")]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Lineup {
    #[model_schema_prop(minLength = 1, maxLength = 2)]
    pub ordered: Ordered,
    #[model_schema_prop(maxLength = 2)]
    pub some: SomeOrdered,
}
