//! An `ObjectId` in each position serde writes one: under a key, in a list, as a map's value, as a
//! type argument, under a brand, in the lone slot of a variant under a tag alone, and flattened,
//! beside a member of its holder's own, ahead of a flattened struct, under an `Option`, under a
//! brand and an alias, and as the type parameter a generic type flattens. serde refuses a key
//! beside `$oid`, and a `$oid` that is not 24 hexadecimal digits, wherever the `ObjectId` is held
//! as a value, and reads past the key where it is flattened.

use std::collections::HashMap;

use mongodb::bson::oid::ObjectId;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Each payload: an `ObjectId` in each position, a key beside its `$oid` there, a `$oid` that is
/// not 24 hexadecimal digits, and one in upper case, which serde alone reads.
const ROWS: [Row; 37] = [
    Row {
        dart: Told::Serde,
        json: concat!(
            r#"{"carried":{"held":{"$oid":"507f1f77bcf86cd799439011"}},"#,
            r#""id":{"$oid":"507f1f77bcf86cd799439011"},"#,
            r#""listed":[{"$oid":"507f1f77bcf86cd799439011"}],"#,
            r#""sigil":{"$oid":"507f1f77bcf86cd799439011"}}"#,
        ),
        kotlin: Told::Serde,
        read_as: "Endorsed",
        schema: true,
        serde: true,
        swift: Told::Serde,
    },
    Row {
        dart: Told::Refused,
        json: concat!(
            r#"{"carried":{"held":{"$oid":"507f1f77bcf86cd799439011"}},"#,
            r#""id":{"$oid":"507f1f77bcf86cd799439011","extra":1},"#,
            r#""listed":[],"#,
            r#""sigil":{"$oid":"507f1f77bcf86cd799439011"}}"#,
        ),
        kotlin: Told::Refused,
        read_as: "Endorsed",
        schema: false,
        serde: false,
        swift: Told::Refused,
    },
    Row {
        dart: Told::Refused,
        json: concat!(
            r#"{"carried":{"held":{"$oid":"507f1f77bcf86cd799439011"}},"#,
            r#""id":{"$oid":"507f1f77bcf86cd799439011"},"#,
            r#""listed":[{"$oid":"507f1f77bcf86cd799439011","extra":1}],"#,
            r#""sigil":{"$oid":"507f1f77bcf86cd799439011"}}"#,
        ),
        kotlin: Told::Refused,
        read_as: "Endorsed",
        schema: false,
        serde: false,
        swift: Told::Refused,
    },
    Row {
        dart: Told::Refused,
        json: concat!(
            r#"{"carried":{"held":{"$oid":"507f1f77bcf86cd799439011","extra":1}},"#,
            r#""id":{"$oid":"507f1f77bcf86cd799439011"},"#,
            r#""listed":[],"#,
            r#""sigil":{"$oid":"507f1f77bcf86cd799439011"}}"#,
        ),
        kotlin: Told::Refused,
        read_as: "Endorsed",
        schema: false,
        serde: false,
        swift: Told::Refused,
    },
    Row {
        dart: Told::Refused,
        json: concat!(
            r#"{"carried":{"held":{"$oid":"507f1f77bcf86cd799439011"}},"#,
            r#""id":{"$oid":"507f1f77bcf86cd799439011"},"#,
            r#""listed":[],"#,
            r#""sigil":{"$oid":"507f1f77bcf86cd799439011","extra":1}}"#,
        ),
        kotlin: Told::Refused,
        read_as: "Endorsed",
        schema: false,
        serde: false,
        swift: Told::Refused,
    },
    Row {
        dart: Told::Refused,
        json: concat!(
            r#"{"carried":{"held":{"$oid":"507f1f77bcf86cd799439011"}},"#,
            r#""id":{"$oid":"507f1f77bcf86cd79943901z"},"#,
            r#""listed":[],"#,
            r#""sigil":{"$oid":"507f1f77bcf86cd799439011"}}"#,
        ),
        kotlin: Told::Refused,
        read_as: "Endorsed",
        schema: false,
        serde: false,
        swift: Told::Refused,
    },
    Row {
        dart: Told::Refused,
        json: concat!(
            r#"{"carried":{"held":{"$oid":"507f1f77bcf86cd799439011"}},"#,
            r#""id":{"$oid":"507F1F77BCF86CD799439011"},"#,
            r#""listed":[],"#,
            r#""sigil":{"$oid":"507f1f77bcf86cd799439011"}}"#,
        ),
        kotlin: Told::Refused,
        read_as: "Endorsed",
        schema: false,
        serde: true,
        swift: Told::Refused,
    },
    Row {
        dart: Told::Serde,
        json: r#"{"named":{"a":{"$oid":"507f1f77bcf86cd799439011"}}}"#,
        kotlin: Told::Serde,
        read_as: "Endorsing",
        schema: true,
        serde: true,
        swift: Told::Serde,
    },
    Row {
        dart: Told::Refused,
        json: r#"{"named":{"a":{"$oid":"507f1f77bcf86cd799439011","extra":1}}}"#,
        kotlin: Told::Refused,
        read_as: "Endorsing",
        schema: false,
        serde: false,
        swift: Told::Refused,
    },
    Row {
        dart: Told::Serde,
        json: r#"{"kind":"Held","$oid":"507f1f77bcf86cd799439011"}"#,
        kotlin: Told::Serde,
        read_as: "Cachet",
        schema: true,
        serde: true,
        swift: Told::Serde,
    },
    Row {
        dart: Told::Refused,
        json: r#"{"kind":"Held","$oid":"507f1f77bcf86cd799439011","extra":1}"#,
        kotlin: Told::Refused,
        read_as: "Cachet",
        schema: false,
        serde: false,
        swift: Told::Refused,
    },
    Row {
        dart: Told::Serde,
        json: r#"{"kind":"Sealed","$oid":"507f1f77bcf86cd799439011"}"#,
        kotlin: Told::Serde,
        read_as: "Cachet",
        schema: true,
        serde: true,
        swift: Told::Serde,
    },
    Row {
        dart: Told::Refused,
        json: r#"{"kind":"Sealed","$oid":"507f1f77bcf86cd799439011","extra":1}"#,
        kotlin: Told::Refused,
        read_as: "Cachet",
        schema: false,
        serde: false,
        swift: Told::Refused,
    },
    Row {
        dart: Told::Serde,
        json: r#"{"id":1,"$oid":"507f1f77bcf86cd799439011"}"#,
        kotlin: Told::Serde,
        read_as: "Embossed",
        schema: true,
        serde: true,
        swift: Told::Serde,
    },
    Row {
        dart: Told::Serde,
        json: r#"{"id":1,"$oid":"507f1f77bcf86cd799439011","extra":1}"#,
        kotlin: Told::Serde,
        read_as: "Embossed",
        schema: false,
        serde: true,
        swift: Told::Serde,
    },
    Row {
        dart: Told::Refused,
        json: r#"{"id":1,"$oid":"507f1f77bcf86cd79943901z"}"#,
        kotlin: Told::Refused,
        read_as: "Embossed",
        schema: false,
        serde: false,
        swift: Told::Refused,
    },
    Row {
        dart: Told::Serde,
        json: r#"{"$oid":"507f1f77bcf86cd799439011","note":"n"}"#,
        kotlin: Told::Serde,
        read_as: "Docketing",
        schema: true,
        serde: true,
        swift: Told::Serde,
    },
    Row {
        dart: Told::Serde,
        json: r#"{"$oid":"507f1f77bcf86cd799439011","note":"n","extra":1}"#,
        kotlin: Told::Serde,
        read_as: "Docketing",
        schema: false,
        serde: true,
        swift: Told::Serde,
    },
    Row {
        dart: Told::Serde,
        json: r#"{"id":1}"#,
        kotlin: Told::Serde,
        read_as: "Stampage",
        schema: true,
        serde: true,
        swift: Told::Serde,
    },
    Row {
        dart: Told::Serde,
        json: r#"{"id":1,"$oid":"507f1f77bcf86cd799439011"}"#,
        kotlin: Told::Serde,
        read_as: "Stampage",
        schema: true,
        serde: true,
        swift: Told::Serde,
    },
    Row {
        dart: Told::Refused,
        json: r#"{"id":1,"$oid":"507f1f77bcf86cd79943901z"}"#,
        kotlin: Told::Refused,
        read_as: "Stampage",
        schema: false,
        serde: true,
        swift: Told::Refused,
    },
    Row {
        dart: Told::Serde,
        json: r#"{"$oid":"507f1f77bcf86cd799439011","note":"n"}"#,
        kotlin: Told::Serde,
        read_as: "Sigiled",
        schema: true,
        serde: true,
        swift: Told::Serde,
    },
    Row {
        dart: Told::Serde,
        json: r#"{"$oid":"507f1f77bcf86cd799439011","note":"n","extra":1}"#,
        kotlin: Told::Serde,
        read_as: "Sigiled",
        schema: false,
        serde: true,
        swift: Told::Serde,
    },
    Row {
        dart: Told::Serde,
        json: r#"{"$oid":"507f1f77bcf86cd799439011","note":"n"}"#,
        kotlin: Told::Serde,
        read_as: "Sigillary",
        schema: true,
        serde: true,
        swift: Told::Serde,
    },
    Row {
        dart: Told::Serde,
        json: r#"{"envelope":{"$oid":"507f1f77bcf86cd799439011","note":"n"}}"#,
        kotlin: Told::Serde,
        read_as: "Lettering",
        schema: true,
        serde: true,
        swift: Told::Serde,
    },
    Row {
        dart: Told::Serde,
        json: r#"{"envelope":{"$oid":"507f1f77bcf86cd799439011","note":"n","extra":1}}"#,
        kotlin: Told::Serde,
        read_as: "Lettering",
        schema: false,
        serde: true,
        swift: Told::Serde,
    },
    Row {
        dart: Told::Refused,
        json: r#"{"envelope":{"$oid":"507f1f77bcf86cd79943901z","note":"n"}}"#,
        kotlin: Told::Refused,
        read_as: "Lettering",
        schema: false,
        serde: false,
        swift: Told::Refused,
    },
    Row {
        dart: Told::Serde,
        json: r#"{"waxed":{"$oid":"507f1f77bcf86cd799439011","note":"n"}}"#,
        kotlin: Told::Serde,
        read_as: "Folding",
        schema: true,
        serde: true,
        swift: Told::Serde,
    },
    Row {
        dart: Told::Serde,
        json: r#"{"waxed":{"note":"n"}}"#,
        kotlin: Told::Serde,
        read_as: "Folding",
        schema: true,
        serde: true,
        swift: Told::Serde,
    },
    Row {
        dart: Told::Serde,
        json: r#"{"waxed":{"$oid":"507f1f77bcf86cd79943901z","note":"n"}}"#,
        kotlin: Told::Refused,
        read_as: "Folding",
        schema: false,
        serde: true,
        swift: Told::Serde,
    },
    Row {
        dart: Told::Serde,
        json: r#"{"relay":{"envelope":{"$oid":"507f1f77bcf86cd799439011","note":"n"}}}"#,
        kotlin: Told::Serde,
        read_as: "Relayed",
        schema: true,
        serde: true,
        swift: Told::Serde,
    },
    Row {
        dart: Told::Serde,
        json: concat!(
            r#"{"crossed":{"$oid":"507f1f77bcf86cd799439011","note":"n","#,
            r#""spare":{"$oid":"507f1f77bcf86cd799439011"}}}"#,
        ),
        kotlin: Told::Serde,
        read_as: "Crossing",
        schema: true,
        serde: true,
        swift: Told::Serde,
    },
    Row {
        dart: Told::Serde,
        json: concat!(
            r#"{"aliased":{"$oid":"507f1f77bcf86cd799439011","note":"n"},"#,
            r#""branded":{"$oid":"507f1f77bcf86cd799439011","note":"n"}}"#,
        ),
        kotlin: Told::Serde,
        read_as: "Bullae",
        schema: true,
        serde: true,
        swift: Told::Serde,
    },
    Row {
        dart: Told::Refused,
        json: r#"{"wafer":{"$oid":"507f1f77bcf86cd799439011"}}"#,
        kotlin: Told::Refused,
        read_as: "Wafered",
        schema: true,
        serde: true,
        swift: Told::Serde,
    },
    Row {
        dart: Told::Refused,
        json: r#"{"note":"n","$oid":"507f1f77bcf86cd799439011"}"#,
        kotlin: Told::Refused,
        read_as: "Wafering",
        schema: true,
        serde: true,
        swift: Told::Serde,
    },
    Row {
        dart: Told::Refused,
        json: r#"{"envelope":{"$oid":"507f1f77bcf86cd799439011","note":"n"}}"#,
        kotlin: Told::Refused,
        read_as: "Enwafered",
        schema: true,
        serde: true,
        swift: Told::Refused,
    },
    Row {
        dart: Told::Refused,
        json: r#"{"$oid":"507f1f77bcf86cd799439011","note":"n"}"#,
        kotlin: Told::Serde,
        read_as: "Belated",
        schema: true,
        serde: true,
        swift: Told::Refused,
    },
];

/// A payload, the type it is read as, and what each side answers of it. Each client reads it
/// beside the README's `ObjectId`.
pub struct Row {
    pub dart: Told,
    pub json: &'static str,
    pub kotlin: Told,
    pub read_as: &'static str,
    /// Whether TypeScript's Zod schema and the JSON Schema take it.
    pub schema: bool,
    /// Whether serde reads it.
    pub serde: bool,
    pub swift: Told,
}

/// What a client's read answers of a payload.
#[derive(Clone, Copy)]
pub enum Told {
    /// A refusal.
    Refused,
    /// The value serde reads, written back as serde writes it.
    Serde,
}

/// A struct of one member, flattened beside an `ObjectId`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Notation {
    pub note: String,
}

/// A struct that declares a parameter, filled at an `ObjectId`.
#[model_schema(default_types(Held = String))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Countersigned<Held> {
    pub held: Held,
}

/// A brand over an `ObjectId`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Sigil(pub ObjectId);

/// An alias of an `ObjectId`.
#[model_schema()]
pub type Sigillum = ObjectId;

/// An `ObjectId` under a key, in a list, as a type argument and under a brand.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Endorsed {
    pub carried: Countersigned<ObjectId>,
    pub id: ObjectId,
    pub listed: Vec<ObjectId>,
    pub sigil: Sigil,
}

/// An `ObjectId` as a map's value.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Endorsing {
    pub named: HashMap<String, ObjectId>,
}

/// An `ObjectId`, and a brand over one, in the lone slot of a variant under a tag alone.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum Cachet {
    Held(ObjectId),
    Sealed(Sigil),
}

/// An `ObjectId` flattened beside a member of its holder's own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Embossed {
    pub id: u32,
    #[serde(flatten)]
    pub oid: ObjectId,
}

/// An `ObjectId` flattened ahead of a flattened struct.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Docketing {
    #[serde(flatten)]
    pub oid: ObjectId,
    #[serde(flatten)]
    pub remark: Notation,
}

/// An optional `ObjectId` flattened beside a member of its holder's own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Stampage {
    pub id: u32,
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub oid: Option<ObjectId>,
}

/// A brand over an `ObjectId` flattened ahead of a flattened struct.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Sigiled {
    #[serde(flatten)]
    pub mark: Sigil,
    #[serde(flatten)]
    pub remark: Notation,
}

/// An alias of an `ObjectId` flattened ahead of a flattened struct.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Sigillary {
    #[serde(flatten)]
    pub oid: Sigillum,
    #[serde(flatten)]
    pub remark: Notation,
}

/// A struct that flattens its type parameter ahead of a flattened struct.
#[model_schema(default_types(Held = Sigil))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Enveloping<Held> {
    #[serde(flatten)]
    pub held: Held,
    #[serde(flatten)]
    pub remark: Notation,
}

/// A struct that flattens an `Option` of its type parameter ahead of a flattened struct.
#[model_schema(default_types(Held = Sigil))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Waxing<Held> {
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub held: Option<Held>,
    #[serde(flatten)]
    pub remark: Notation,
}

/// A struct that hands its type parameter on to [`Enveloping`].
#[model_schema(default_types(Held = Sigil))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Relaying<Held> {
    pub envelope: Enveloping<Held>,
}

/// A struct of one slot over an `ObjectId`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Wafer(pub ObjectId);

/// [`Enveloping`] filled at an `ObjectId`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Lettering {
    pub envelope: Enveloping<ObjectId>,
}

/// [`Waxing`] filled at an `ObjectId`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Folding {
    pub waxed: Waxing<ObjectId>,
}

/// [`Relaying`] filled at an `ObjectId`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Relayed {
    pub relay: Relaying<ObjectId>,
}

/// A struct that flattens its type parameter ahead of a flattened struct, and reads it under a key
/// as well.
#[model_schema(default_types(Held = Sigil))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Crossed<Held> {
    #[serde(flatten)]
    pub held: Held,
    #[serde(flatten)]
    pub remark: Notation,
    pub spare: Held,
}

/// [`Crossed`] filled at an `ObjectId`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Crossing {
    pub crossed: Crossed<ObjectId>,
}

/// [`Enveloping`] filled at an alias and a brand over an `ObjectId`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Bullae {
    pub aliased: Enveloping<Sigillum>,
    pub branded: Enveloping<Sigil>,
}

/// [`Wafer`] under a key. The Dart and Kotlin reads take a struct of one slot from an array of
/// one item, where serde writes the slot's value itself.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Wafered {
    pub wafer: Wafer,
}

/// [`Wafer`] flattened ahead of a flattened struct.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Wafering {
    #[serde(flatten)]
    pub remark: Notation,
    #[serde(flatten)]
    pub wafer: Wafer,
}

/// [`Enveloping`] filled at [`Wafer`].
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Enwafered {
    pub envelope: Enveloping<Wafer>,
}

/// A brand over an `ObjectId` declared below it, flattened ahead of a flattened struct.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Belated {
    #[serde(flatten)]
    pub mark: LateSigil,
    #[serde(flatten)]
    pub remark: Notation,
}

/// A brand over an `ObjectId`, declared below [`Belated`].
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct LateSigil(pub ObjectId);

/// [`ROWS`], each held to what its columns state of one another: neither a client nor the schemas
/// take a payload serde refuses.
pub fn rows() -> &'static [Row] {
    for row in &ROWS {
        for told in [row.dart, row.kotlin, row.swift] {
            assert!(
                row.serde || matches!(told, Told::Refused),
                "a client is stated to read {}, which serde refuses",
                row.json
            );
            assert!(
                row.serde || !row.schema,
                "the schemas are stated to take {}, which serde refuses",
                row.json
            );
        }
    }
    &ROWS
}

/// What serde writes back of `json` read as the type `read_as` names, or `refused`.
pub fn serde_told(read_as: &str, json: &str) -> String {
    match read_as {
        "Endorsed" => written_back::<Endorsed>(json),
        "Endorsing" => written_back::<Endorsing>(json),
        "Cachet" => written_back::<Cachet>(json),
        "Embossed" => written_back::<Embossed>(json),
        "Docketing" => written_back::<Docketing>(json),
        "Stampage" => written_back::<Stampage>(json),
        "Sigiled" => written_back::<Sigiled>(json),
        "Sigillary" => written_back::<Sigillary>(json),
        "Lettering" => written_back::<Lettering>(json),
        "Folding" => written_back::<Folding>(json),
        "Relayed" => written_back::<Relayed>(json),
        "Bullae" => written_back::<Bullae>(json),
        "Crossing" => written_back::<Crossing>(json),
        "Belated" => written_back::<Belated>(json),
        "Wafered" => written_back::<Wafered>(json),
        "Wafering" => written_back::<Wafering>(json),
        _ => written_back::<Enwafered>(json),
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
