//! The lone slot of a newtype variant under a tag alone, in each shape serde writes beside the
//! tag: a struct, one under its own tag, a brand and an alias over one, a generic struct, the
//! enum's own type parameter, an enum of each tagging, a unit struct, a struct with a flattened
//! member, one declared below the enum, and one a function reads. serde writes the slot's members
//! beside the tag, and reads the slot from the object without the tag.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::model_schema;

/// Each payload serde writes, and the type it is read as, as Kotlin and Swift name it. The row of
/// an externally tagged enum's unit variant is [`UNIT_ENTRY`].
pub const WRITTEN: [(&str, &str); 19] = [
    ("Docketed", r#"{"kind":"Weighed","cost":"c"}"#),
    (
        "Docketed",
        r#"{"kind":"Sorted","sort":"Ballast","weight":3}"#,
    ),
    ("Docketed", r#"{"kind":"Branded","cost":"c"}"#),
    ("Docketed", r#"{"kind":"Aliased","cost":"c"}"#),
    ("Docketed", r#"{"kind":"Filled","load":"l"}"#),
    ("Docketed", r#"{"kind":"Inner","sort":"Daily","days":2}"#),
    ("Docketed", r#"{"kind":"Inner","sort":"Waived"}"#),
    (
        "Docketed",
        r#"{"kind":"Adjacent","sort":"Fuel","body":{"tons":4}}"#,
    ),
    ("Docketed", r#"{"kind":"Adjacent","sort":"Empty"}"#),
    ("Docketed", r#"{"kind":"External","Stacked":{"layers":5}}"#),
    ("Docketed", UNIT_ENTRY),
    ("Docketed", r#"{"kind":"Either","miles":6}"#),
    ("Docketed", r#"{"kind":"Either","bay":"b"}"#),
    ("Docketed", r#"{"kind":"Hired"}"#),
    ("Docketed", r#"{"kind":"Merged","note":"n","cost":"c"}"#),
    ("Docketed", r#"{"kind":"Below","barge":"g"}"#),
    ("Docketed", r#"{"kind":"Trimmed","cost":"c"}"#),
    ("Docketed", r#"{"kind":"Idle"}"#),
    ("Waybilled<Tonnage>", r#"{"kind":"Held","cost":"c"}"#),
];

/// What serde writes of an externally tagged enum's unit variant beside the tag: its name over
/// `null`. TypeScript and the generated Zod schema describe no payload for it.
pub const UNIT_ENTRY: &str = r#"{"kind":"External","Loose":null}"#;

/// A struct of one member.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Tonnage {
    pub cost: String,
}

/// A struct under its own tag.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "sort")]
pub struct Ballast {
    pub weight: u32,
}

/// A brand over [`Tonnage`].
#[model_schema(no_display)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Wharfage(pub Tonnage);

/// An alias of [`Tonnage`].
#[model_schema()]
pub type Drayage = Tonnage;

/// A struct that declares a parameter.
#[model_schema(default_types(Load = String))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Lighter<Load> {
    pub load: Load,
}

/// An enum under a tag alone.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "sort")]
pub enum Demurrage {
    Daily { days: u32 },
    Waived,
}

/// An enum under a tag and a content key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "sort", content = "body")]
pub enum Bunkered {
    Empty,
    Fuel { tons: u32 },
}

/// An enum with no tag attribute, of a variant of members and a unit variant.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Palleted {
    Loose,
    Stacked { layers: u32 },
}

/// An untagged enum of two structs' shapes.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Dray {
    Hauled { miles: u32 },
    Parked { bay: String },
}

/// A unit struct, which every surface writes as `{}`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Stevedore;

/// A struct with a flattened member.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Freighted {
    pub note: String,
    #[serde(flatten)]
    pub tonnage: Tonnage,
}

/// An enum under a tag alone whose newtype variants hold each shape above, one declared below it
/// and one a function reads.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum Docketed {
    Adjacent(Bunkered),
    Aliased(Drayage),
    Below(Lightering),
    Branded(Wharfage),
    Either(Dray),
    External(Palleted),
    Filled(Lighter<String>),
    Hired(Stevedore),
    Idle,
    Inner(Demurrage),
    Merged(Freighted),
    Sorted(Ballast),
    Trimmed(#[model_schema_prop(preprocess = ["lightened"])] Tonnage),
    Weighed(Tonnage),
}

/// A struct declared below the enum that holds it.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Lightering {
    pub barge: String,
}

/// An enum under a tag alone whose newtype variant holds its own type parameter.
#[model_schema(default_types(Held = Tonnage))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum Waybilled<Held> {
    Empty,
    Held(Held),
}

/// The function [`Docketed::Trimmed`] names: drops a key `spare` from the object it is handed.
pub fn lightened(mut payload: Value) -> Value {
    if let Some(object) = payload.as_object_mut() {
        object.remove("spare");
    }
    payload
}

/// What serde writes back of `json` read as the type `read_as` names, or `refused`.
pub fn serde_told(read_as: &str, json: &str) -> String {
    if read_as == "Docketed" {
        written_back::<Docketed>(json)
    } else {
        written_back::<Waybilled<Tonnage>>(json)
    }
}

/// What serde writes back of `json` read as `T`, or `refused`.
fn written_back<T>(json: &str) -> String
where
    T: DeserializeOwned + Serialize,
{
    serde_json::from_str::<T>(json).map_or_else(
        |_| "refused".to_owned(),
        |read| format!("read {}", serde_json::to_string(&read).unwrap()),
    )
}
