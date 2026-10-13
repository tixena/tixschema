//! The models the emitted Swift read is run on, beside those of `swift_validate_models.rs`, and
//! the Rust function each check names.

use alloc::collections::BTreeMap;
use core::future::ready;

use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

use super::swift_validate_models::{
    Base, Package, Parcel, base_schema, package_schema, parcel_schema,
};

/// A parcel held in a field, a list, a map and an optional.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub first_parcel: Parcel,
    pub parcel_list: Vec<Parcel>,
    pub parcels_by_dock: BTreeMap<String, Parcel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare_parcel: Option<Parcel>,
}

/// An optional and a nullable member over a string and over a model, in a struct whose read Swift
/// synthesized before one was written.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mooring {
    #[model_schema_prop(nullable)]
    pub berth_note: Option<String>,
    #[model_schema_prop(nullable)]
    pub held_package: Option<Package>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare_note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare_package: Option<Package>,
}

/// The same nullable members in a struct whose read was written before: it flattens a member.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Anchorage {
    #[model_schema_prop(nullable)]
    pub anchor_note: Option<String>,
    #[model_schema_prop(nullable)]
    pub held_package: Option<Package>,
    #[serde(flatten)]
    pub starting_base: Base,
}

/// A model in a map under a key Swift reads as a number.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Decking {
    pub by_deck: BTreeMap<isize, Package>,
}

/// A brand with a bound of its own, which chooses a variant.
#[model_schema(pattern = "^[0-9]+$")]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Pin(pub String);

/// A brand that names a check, which chooses none.
#[model_schema(check = ["signetPressed"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Signet(pub String);

/// An untagged enum whose variants one payload fits several of: a bound on a slot, a brand with a
/// bound, a bound on a named member, a list and a map of models, and a model.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged, rename_all_fields = "camelCase")]
pub enum Sorted {
    Brief(#[model_schema_prop(maxLength = 3)] String),
    Coded(Pin),
    Counted {
        #[model_schema_prop(minimum = 1)]
        sorted_count: u32,
    },
    Listed(Vec<Package>),
    Mapped(BTreeMap<String, Package>),
    Whole(Package),
    Wider {
        item_label: String,
    },
    Wordy(String),
}

/// An untagged enum whose first variant holds a brand with a check.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Vetted {
    Signed(Signet),
    Worded(String),
}

/// An enum under a tag and a content key whose slot holds a model.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum Towed {
    Boxed(Package),
    Unladen,
}

/// A plain enum that names a check.
#[model_schema(check = ["withAnUrgency"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Urgency {
    High,
    Low,
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum TrackingError {
    NotFound,
}

/// A service whose reply is a model with a bound and a check.
#[service_schema(transports = [])]
pub trait TrackingClientService<Ctx> {
    async fn track(&self, ctx: &Ctx, parcel_label: String) -> Result<Parcel, TrackingError>;
}

/// A backend answering the contract, so the trait is implementable rather than merely declared.
pub struct TrackingBackEnd;

impl TrackingClientService<()> for TrackingBackEnd {
    async fn track(&self, _ctx: &(), parcel_label: String) -> Result<Parcel, TrackingError> {
        ready(()).await;
        if parcel_label.is_empty() {
            return Err(TrackingError::NotFound);
        }
        Ok(Parcel {
            fragile_item_codes: Vec::new(),
            item_code_list: Vec::new(),
            label: parcel_label,
        })
    }
}

fn signet_pressed(value: &Signet) -> Result<(), Vec<String>> {
    if value.0.chars().any(char::is_lowercase) {
        return Err(vec!["a signet is pressed in upper case".to_owned()]);
    }
    Ok(())
}

fn with_an_urgency(value: &Urgency) -> Result<(), Vec<String>> {
    if matches!(value, Urgency::Low) {
        return Err(vec!["a low urgency is not carried".to_owned()]);
    }
    Ok(())
}
