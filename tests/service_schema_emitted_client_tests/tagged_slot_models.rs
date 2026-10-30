//! The lone slot of a newtype variant under a tag alone, in each shape serde writes beside the
//! tag: a struct, one under its own tag, a brand and an alias over one, a generic struct, the
//! enum's own type parameter, an enum of each tagging, an alias of an externally tagged one, a unit
//! struct, a struct with a flattened member, one declared below the enum, and a struct and an
//! externally tagged enum a function reads. serde writes the slot's members beside the tag, an
//! externally tagged enum's variant as an entry, and reads the slot from the object without the
//! tag. A struct that flattens the alias reads and writes the same entries, and so do a slot and a
//! struct over an externally tagged enum that declares a parameter, written with an argument. An
//! `ObjectId` in the slot is written as its `$oid` key beside the tag, as a flattened one is.

#[cfg(feature = "mongodb")]
use mongodb::bson::oid::ObjectId;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::model_schema;

/// Each payload serde writes, and the type it is read as, as Kotlin and Swift name it: an
/// externally tagged enum's unit variant among them, as an entry of its name over `null`.
pub const WRITTEN: [(&str, &str); 29] = [
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
    ("Docketed", r#"{"kind":"External","Loose":null}"#),
    ("Docketed", r#"{"kind":"Either","miles":6}"#),
    ("Docketed", r#"{"kind":"Either","bay":"b"}"#),
    ("Docketed", r#"{"kind":"Hired"}"#),
    ("Docketed", r#"{"kind":"Merged","note":"n","cost":"c"}"#),
    ("Docketed", r#"{"kind":"Below","barge":"g"}"#),
    ("Docketed", r#"{"kind":"Trimmed","cost":"c"}"#),
    ("Docketed", r#"{"kind":"Pared","Loose":null}"#),
    ("Docketed", r#"{"kind":"Pared","Stacked":{"layers":5}}"#),
    ("Docketed", r#"{"kind":"Lashed","Loose":null}"#),
    ("Docketed", r#"{"kind":"Lashed","Stacked":{"layers":5}}"#),
    ("Trussed", r#"{"note":"n","Loose":null}"#),
    ("Trussed", r#"{"note":"n","Stacked":{"layers":5}}"#),
    ("Docketed", r#"{"kind":"Idle"}"#),
    ("Waybilled<Tonnage>", r#"{"kind":"Held","cost":"c"}"#),
    ("Docketed", r#"{"kind":"Cradled","Carried":7}"#),
    ("Docketed", r#"{"kind":"Cradled","Empty":null}"#),
    ("Swaddled", r#"{"note":"n","Carried":7}"#),
    ("Swaddled", r#"{"note":"n","Empty":null}"#),
];

/// The payloads of [`Docketed::External`] serde refuses, and the words each client refuses each
/// with: the tag alone, the entries of two variants, and a unit variant's entry over a number.
pub const ENTRIES_REFUSED: [Refused; 3] = [
    Refused {
        dart: "FormatException: empty object for Palleted",
        json: r#"{"kind":"External"}"#,
        kotlin: "refused: expected an object of one key, found {}",
        swift: "empty object for Palleted",
    },
    Refused {
        dart: "FormatException: more than one key for Palleted",
        json: r#"{"kind":"External","Stacked":{"layers":5},"Loose":null}"#,
        kotlin: "refused: expected an object of one key, found \
                 {\"Stacked\":{\"layers\":5},\"Loose\":null}",
        swift: "more than one key for Palleted",
    },
    Refused {
        dart: "FormatException: Palleted: Loose takes no value",
        json: r#"{"kind":"External","Loose":1}"#,
        kotlin: "refused: Palleted: Loose takes no value",
        swift: "unknown tag Loose",
    },
];

/// Each payload serde writes for [`Stamping`]: an `ObjectId` beside the tag, owned, in a `Box` and
/// under a brand.
#[cfg(feature = "mongodb")]
pub const STAMPED: [&str; 3] = [
    r#"{"kind":"Held","$oid":"507f1f77bcf86cd799439011"}"#,
    r#"{"kind":"Boxed","$oid":"507f1f77bcf86cd799439011"}"#,
    r#"{"kind":"Branded","$oid":"507f1f77bcf86cd799439011"}"#,
];

/// The tag of [`Stamping::Held`] alone, which serde refuses, and the words each client's
/// `ObjectId` refuses it with.
#[cfg(feature = "mongodb")]
pub const STAMP_REFUSED: Refused = Refused {
    dart: "FormatException: Invalid ObjectId: {}",
    json: r#"{"kind":"Held"}"#,
    kotlin: "refused: Field '$oid' is required for type with serial name 'ObjectId', but it was \
             missing at path: $",
    swift: "missing field `$oid`",
};

/// A payload serde refuses, which TypeScript and the generated Zod schema refuse too, and what
/// each client refuses it with.
pub struct Refused {
    /// What the Dart read throws, its class and its message.
    pub dart: &'static str,
    pub json: &'static str,
    /// What the Kotlin read throws under either `Json`.
    pub kotlin: &'static str,
    /// The description of what the Swift read throws.
    pub swift: &'static str,
}

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

/// An alias of [`Palleted`].
#[model_schema()]
pub type Shrinkwrapped = Palleted;

/// [`Shrinkwrapped`] flattened beside a member of its holder's own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Trussed {
    pub note: String,
    #[serde(flatten)]
    pub pallet: Shrinkwrapped,
}

/// An enum with no tag attribute that declares a parameter, of a variant that holds it and a unit
/// variant.
#[model_schema(default_types(Load = String))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Cradled<Load> {
    Carried(Load),
    Empty,
}

/// [`Cradled`] written with an argument and flattened beside a member of its holder's own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Swaddled {
    pub note: String,
    #[serde(flatten)]
    pub wrapping: Cradled<u32>,
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

/// An enum under a tag alone whose newtype variants hold each shape above, one declared below it,
/// and a struct and an externally tagged enum a function reads.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum Docketed {
    Adjacent(Bunkered),
    Aliased(Drayage),
    Below(Lightering),
    Branded(Wharfage),
    Cradled(Cradled<u32>),
    Either(Dray),
    External(Palleted),
    Filled(Lighter<String>),
    Hired(Stevedore),
    Idle,
    Inner(Demurrage),
    Lashed(Shrinkwrapped),
    Merged(Freighted),
    Pared(#[model_schema_prop(preprocess = ["lightened"])] Palleted),
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

/// A brand over an `ObjectId`.
#[cfg(feature = "mongodb")]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Imprint(pub ObjectId);

/// An enum under a tag alone whose newtype variants hold an `ObjectId`, owned, in a `Box` and
/// under [`Imprint`].
#[cfg(feature = "mongodb")]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum Stamping {
    Boxed(Box<ObjectId>),
    Branded(Imprint),
    Held(ObjectId),
}

/// The function [`Docketed::Trimmed`] and [`Docketed::Pared`] name: drops a key `spare` from the
/// object it is handed.
pub fn lightened(mut payload: Value) -> Value {
    if let Some(object) = payload.as_object_mut() {
        object.remove("spare");
    }
    payload
}

/// What serde writes back of `json` read as the type `read_as` names, or `refused`.
pub fn serde_told(read_as: &str, json: &str) -> String {
    match read_as {
        "Docketed" => written_back::<Docketed>(json),
        "Trussed" => written_back::<Trussed>(json),
        "Swaddled" => written_back::<Swaddled>(json),
        #[cfg(feature = "mongodb")]
        "Stamping" => written_back::<Stamping>(json),
        _ => written_back::<Waybilled<Tonnage>>(json),
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
