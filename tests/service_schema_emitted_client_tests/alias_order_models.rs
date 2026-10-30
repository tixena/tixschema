//! Untagged enums over an alias of a string, one with the alias declared above it and one with the
//! alias declared below it, and what serde answers of each payload: the first variant whose
//! payload reads.

use core::fmt::Debug;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// The payloads read as [`Tethering`] and as [`Binding`]: a text, a null, a number and a list.
pub const PAYLOADS: [&str; 4] = [r#""abc""#, "null", "7", r#"["a"]"#];

/// An alias of a string, declared above the enum that holds it.
#[model_schema()]
pub type Strap = String;

/// An untagged enum over an alias declared above it.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Tethering {
    Strapped(Strap),
    Tallied(u32),
}

/// An untagged enum over an alias declared below it.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Binding {
    Cinched(Cinch),
    Counted(u32),
}

/// An alias of a string, declared below the enum that holds it.
#[model_schema()]
pub type Cinch = String;

/// What the Rust read of `payload` as `Held` takes: the name of its variant, or `refused`.
pub fn rust_takes<Held>(payload: &str) -> String
where
    Held: DeserializeOwned + Debug,
{
    serde_json::from_str::<Held>(payload).map_or_else(
        |_| "refused".to_owned(),
        |held| {
            format!("{held:?}")
                .split(|written: char| !written.is_alphanumeric())
                .next()
                .unwrap_or_default()
                .to_owned()
        },
    )
}
