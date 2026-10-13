//! The models the emitted Swift `preprocess` is run on, beside those of
//! `pattern_preprocess_tests/models.rs`, and the Rust function each one names.

use alloc::collections::BTreeMap;
use core::cell::Cell;
use core::future::ready;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::{model_schema, service_schema};

use super::preprocess_models::{
    Parcel, Pinned, Tagged, or_unnamed, parcel_schema, pinned_schema, tagged_schema, trimmed,
    unwrapped, with_its_keys,
};

thread_local! {
    /// How often the function of a tallied label was called on this thread.
    static LABEL_CALLS: Cell<u32> = const { Cell::new(0) };
    /// How often the function of a tallied spare was called on this thread.
    static SPARE_CALLS: Cell<u32> = const { Cell::new(0) };
}

/// A label read from a number as its text.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Stamped {
    #[model_schema_prop(preprocess = ["textual"])]
    pub label: String,
}

/// A list whose function names the Swift value each of its items reaches a function as.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Sampled {
    #[model_schema_prop(preprocess = ["kindsOf"])]
    pub kinds: Vec<String>,
}

/// A required and an optional member whose functions count their calls.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Tallied {
    #[model_schema_prop(preprocess = ["countedLabel"])]
    pub label: String,
    #[model_schema_prop(preprocess = ["countedSpare"])]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare: Option<String>,
}

/// A nullable and an optional member that name a function.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Spared {
    #[model_schema_prop(nullable, preprocess = ["orUnnamed"])]
    pub note: Option<String>,
    #[model_schema_prop(preprocess = ["orUnnamed"])]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare: Option<String>,
}

/// A function on a member of each kind a struct reads: a map Swift reads through a wrapper, a
/// model, a list and a map of models, an optional model, and a text with two functions.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Loaded {
    #[model_schema_prop(preprocess = ["unwrapped"])]
    pub by_number: BTreeMap<u32, String>,
    #[model_schema_prop(preprocess = ["unwrapped"])]
    pub first_parcel: Parcel,
    #[model_schema_prop(preprocess = ["unwrapped"])]
    pub parcel_list: Vec<Parcel>,
    #[model_schema_prop(preprocess = ["unwrapped"])]
    pub parcels_by_dock: BTreeMap<String, Parcel>,
    #[model_schema_prop(preprocess = ["unwrapped"])]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare_parcel: Option<Parcel>,
    #[model_schema_prop(preprocess = ["unwrapped", "trimmed"])]
    pub title: String,
}

/// The keys a function lists, read by a type that names no function.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Listing {
    pub keys: Vec<String>,
    pub tag: String,
}

/// A flattened member that names a function of its own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Docked {
    pub dock: String,
    #[serde(flatten)]
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    pub listing: Listing,
}

/// An internally tagged enum whose slot names a function of its own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Marked {
    Held(#[model_schema_prop(preprocess = ["withItsKeys"])] Listing),
    Lost,
}

/// An internally tagged enum that holds a type whose function lists its keys.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Stowed {
    Held(Tagged),
    Lost,
}

/// A holder that flattens a holder that flattens the type whose function lists its keys.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Quayside {
    pub berth: String,
    #[serde(flatten)]
    pub pinned: Pinned,
}

/// A second type whose function lists its keys.
#[model_schema(preprocess = ["withItsKeys"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Sided {
    pub keys: Vec<String>,
    pub side: String,
}

/// Two flattened types, each of which names a function.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Twice {
    pub dock: String,
    #[serde(flatten)]
    pub first: Tagged,
    #[serde(flatten)]
    pub second: Sided,
}

/// An internally tagged enum whose variant flattens the type whose function lists its keys.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Boxed {
    Held {
        dock: String,
        #[serde(flatten)]
        tagged: Tagged,
    },
    Lost,
}

/// An untagged enum that holds the type whose function lists its keys.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Choice {
    Named { name: String },
    Tagged(Tagged),
}

/// A holder that flattens the untagged enum.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Chosen {
    pub choice_dock: String,
    #[serde(flatten)]
    pub taken: Choice,
}

/// A holder that flattens the internally tagged enum.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Lodged {
    pub dock: String,
    #[serde(flatten)]
    pub stowed: Stowed,
}

/// A brand over the type whose function lists its keys.
#[model_schema(no_display)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Banded(pub Tagged);

/// A holder that flattens the brand.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Strapped {
    #[serde(flatten)]
    pub banded: Banded,
    pub dock: String,
}

/// A holder declared above the type it flattens, whose function lists its keys.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Ahead {
    #[serde(flatten)]
    pub behind: Behind,
    pub dock: String,
}

/// A type whose function lists its keys, declared below its holder.
#[model_schema(preprocess = ["withItsKeys"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Behind {
    pub keys: Vec<String>,
    pub tag: String,
}

/// A tuple struct whose slots name a function: a text, and a model.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Paired(
    #[model_schema_prop(preprocess = ["trimmed"])] pub String,
    #[model_schema_prop(preprocess = ["unwrapped"])] pub Parcel,
);

/// A struct serde writes as the value of its one slot, which names a function.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Lone(#[model_schema_prop(preprocess = ["trimmed"])] pub String);

/// An enum with no tag attribute whose slot, member and slots name a function.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Moved {
    Rail(#[model_schema_prop(preprocess = ["trimmed"])] String),
    Road {
        #[model_schema_prop(preprocess = ["trimmed"])]
        plate: String,
    },
    Split(#[model_schema_prop(preprocess = ["trimmed"])] String, u32),
}

/// The same under a tag and a content key, with a model in a slot.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum Shifted {
    Held(#[model_schema_prop(preprocess = ["unwrapped"])] Parcel),
    Rail(#[model_schema_prop(preprocess = ["trimmed"])] String),
    Split(#[model_schema_prop(preprocess = ["trimmed"])] String, u32),
}

/// The same in an untagged enum.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Loosed {
    Rail(#[model_schema_prop(preprocess = ["trimmed"])] String),
    Road {
        #[model_schema_prop(preprocess = ["trimmed"])]
        plate: String,
    },
}

/// A brand with a parameter that names a function.
#[model_schema(default_types(IdType = String), preprocess = ["unwrapped"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Sealing<IdType>(pub IdType);

/// A type with a parameter whose members name a function: a list of the parameter, and a model.
#[model_schema(default_types(CodeType = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Binned<CodeType> {
    #[model_schema_prop(preprocess = ["unwrapped"])]
    pub codes: Vec<CodeType>,
    #[model_schema_prop(preprocess = ["unwrapped"])]
    pub first_parcel: Parcel,
}

/// A date that names a function.
#[cfg(feature = "chrono")]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Timed {
    #[model_schema_prop(preprocess = ["unwrapped"])]
    pub at: chrono::DateTime<chrono::Utc>,
}

/// A type that reaches itself through a member that names a function.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Branch {
    #[model_schema_prop(preprocess = ["orNoBranches"])]
    pub branches: Vec<Self>,
    pub id: String,
}

/// A text whose Swift function answers a value that is no JSON.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Dated {
    #[model_schema_prop(preprocess = ["asADate"])]
    pub stamp: String,
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum ShippingError {
    NotFound,
}

/// A service whose reply names a function on itself and on a member.
#[service_schema(transports = [])]
pub trait ShippingClientService<Ctx> {
    async fn shipped(&self, ctx: &Ctx, parcel_label: String) -> Result<Parcel, ShippingError>;
}

/// A backend answering the contract, so the trait is implementable rather than merely declared.
pub struct ShippingBackEnd;

impl ShippingClientService<()> for ShippingBackEnd {
    async fn shipped(&self, _ctx: &(), parcel_label: String) -> Result<Parcel, ShippingError> {
        ready(()).await;
        if parcel_label.is_empty() {
            return Err(ShippingError::NotFound);
        }
        Ok(Parcel {
            fragile_item_codes: Vec::new(),
            item_code_list: Vec::new(),
            label: parcel_label,
        })
    }
}

/// How often the two functions of a tallied value were called on this thread: its label's, then
/// its spare's.
pub fn tallied_calls() -> (u32, u32) {
    (LABEL_CALLS.get(), SPARE_CALLS.get())
}

const fn as_a_date(payload: Value) -> Value {
    payload
}

fn counted_label(payload: Value) -> Value {
    LABEL_CALLS.set(LABEL_CALLS.get() + 1);
    payload
}

fn counted_spare(payload: Value) -> Value {
    SPARE_CALLS.set(SPARE_CALLS.get() + 1);
    payload
}

/// A list of the names of the Swift values its items reach a function as, and any other payload
/// as it is.
fn kinds_of(payload: Value) -> Value {
    let Value::Array(items) = payload else {
        return payload;
    };
    let named = items.iter().map(|item| match item {
        Value::Null => "NSNull",
        Value::Bool(_) => "Bool",
        Value::Number(number) if number.is_f64() => "Double",
        Value::Number(_) => "Int64",
        Value::String(_) => "String",
        Value::Array(_) => "[Any]",
        Value::Object(_) => "[String: Any]",
    });
    Value::Array(named.map(|name| Value::String(name.to_owned())).collect())
}

/// An empty list for a payload that holds nothing, and any other payload as it is.
fn or_no_branches(payload: Value) -> Value {
    if payload.is_null() {
        Value::Array(Vec::new())
    } else {
        payload
    }
}

/// The text of a number, and any other payload as it is.
fn textual(payload: Value) -> Value {
    if let Value::Number(number) = &payload {
        Value::String(number.to_string())
    } else {
        payload
    }
}
