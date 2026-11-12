//! One constrained model of each shape, each with a rule the tests break.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;
#[cfg(feature = "serde")]
use tixschema::service_schema;

/// A named struct, and the request of `AccountService::sign_up`.
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Signup {
    #[model_schema_prop(minLength = 3)]
    pub username: String,
}

/// A tuple struct with a rule on each slot.
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Pair(
    #[model_schema_prop(minLength = 1)] pub String,
    #[model_schema_prop(minimum = 1)] pub u32,
);

/// A brand over a string.
#[cfg(feature = "serde")]
#[model_schema(minLength = 3)]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Username(String);

/// An alias of the brand.
#[cfg(feature = "serde")]
#[model_schema()]
pub type Login = Username;

/// Holds the brand through its alias, under a key serde renames.
#[cfg(feature = "serde")]
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub display_login: Login,
}

/// An externally tagged enum.
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Contact {
    Email {
        #[model_schema_prop(minLength = 3)]
        address: String,
    },
}

/// An internally tagged enum.
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum TaggedContact {
    Email {
        #[model_schema_prop(minLength = 3)]
        address: String,
    },
}

/// An adjacently tagged enum.
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "contact")]
pub enum AdjacentContact {
    Email {
        #[model_schema_prop(minLength = 3)]
        address: String,
    },
}

/// An untagged enum.
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum UntaggedContact {
    Email {
        #[model_schema_prop(minLength = 3)]
        address: String,
    },
}

/// A generic struct, which the tests fill with `u32`, a filling other than its declared default.
#[model_schema(default_types(Entry = String))]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Roster<Entry> {
    pub entries: Vec<Entry>,
    #[model_schema_prop(minLength = 1)]
    pub title: String,
}

/// A struct the recovering decode reads.
#[cfg(feature = "serde")]
#[model_schema(enable = ["decode_with"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Handle {
    #[model_schema_prop(minLength = 3)]
    pub name: String,
}

/// The reply to [`AccountService::sign_up`].
#[cfg(feature = "serde")]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Done {
    pub welcomed: bool,
}

/// The error [`AccountService::sign_up`] declares.
#[cfg(feature = "serde")]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "errorCode")]
pub enum SignupError {
    Taken,
}

/// A service that names no transport, whose module publishes the request check a dispatcher runs.
#[cfg(feature = "serde")]
#[service_schema()]
pub trait AccountService<Ctx> {
    async fn sign_up(&self, ctx: &Ctx, req: Signup) -> Result<Done, SignupError>;
}
