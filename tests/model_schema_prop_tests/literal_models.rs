//! The models a `literal` is written on, in each place a member is, and [`LITERAL_READS`]: what
//! each payload reads as. Rust's read, the Zod schema and the read of each client are held to the
//! one table.

#![cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]

use core::fmt::Write as _;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Each payload, the model it is read as, and what the read answers.
pub const LITERAL_READS: [LiteralRead; 24] = [
    read("hallmark as declared", "Hallmark", HALLMARK),
    read(
        "hallmark with no offset",
        "Hallmark",
        r#"{"fineness":925,"grade":"sterling","marks":[],"purity":0.5,"stamped":true,"weight":1.25}"#,
    ),
    refused(
        "hallmark of other values",
        "Hallmark",
        r#"{"fineness":916,"grade":"britannia","marks":["lion","anchor"],"offset":4,"purity":0.75,"stamped":false,"weight":2.5}"#,
        &[
            "'fineness': not the literal 925, got 916",
            "'grade': not the literal \"sterling\", got britannia",
            "'marks.1': not the literal \"lion\", got anchor",
            "'offset': not the literal -3, got 4",
            "'purity': not the literal 0.5, got 0.75",
            "'stamped': not the literal true, got false",
            "'weight': not the literal 1.25, got 2.5",
        ],
    ),
    read("assay sealed", "Assay", r#"{"Sealed":"wax"}"#),
    read("assay struck", "Assay", r#"{"Struck":{"tool":"die"}}"#),
    read("assay weighed", "Assay", r#"{"Weighed":[31,true]}"#),
    refused(
        "assay sealed in glue",
        "Assay",
        r#"{"Sealed":"glue"}"#,
        &["'Sealed': not the literal \"wax\", got glue"],
    ),
    refused(
        "assay struck with a hammer",
        "Assay",
        r#"{"Struck":{"tool":"hammer"}}"#,
        &["'Struck.tool': not the literal \"die\", got hammer"],
    ),
    refused(
        "assay weighed short",
        "Assay",
        r#"{"Weighed":[30,false]}"#,
        &[
            "'Weighed.0': not the literal 31, got 30",
            "'Weighed.1': not the literal true, got false",
        ],
    ),
    read(
        "tagged assay rubbed",
        "TaggedAssay",
        r#"{"kind":"Rubbed","passes":2}"#,
    ),
    refused(
        "tagged assay rubbed thrice",
        "TaggedAssay",
        r#"{"kind":"Rubbed","passes":3}"#,
        &["'passes': not the literal 2, got 3"],
    ),
    refused(
        "tagged assay struck with a hammer",
        "TaggedAssay",
        r#"{"kind":"Struck","tool":"hammer"}"#,
        &["'tool': not the literal \"die\", got hammer"],
    ),
    read(
        "adjacent assay sealed",
        "AdjacentAssay",
        r#"{"kind":"Sealed","body":"wax"}"#,
    ),
    refused(
        "adjacent assay sealed in glue",
        "AdjacentAssay",
        r#"{"kind":"Sealed","body":"glue"}"#,
        &["'body': not the literal \"wax\", got glue"],
    ),
    refused(
        "adjacent assay struck with a hammer",
        "AdjacentAssay",
        r#"{"kind":"Struck","body":{"tool":"hammer"}}"#,
        &["'body.tool': not the literal \"die\", got hammer"],
    ),
    refused(
        "adjacent assay weighed short",
        "AdjacentAssay",
        r#"{"kind":"Weighed","body":[30,false]}"#,
        &[
            "'body.0': not the literal 31, got 30",
            "'body.1': not the literal true, got false",
        ],
    ),
    read("ingot of gold", "Ingot", r#"{"metal":"gold","weight":1}"#),
    read(
        "ingot of silver",
        "Ingot",
        r#"{"metal":"silver","weight":1}"#,
    ),
    LiteralRead {
        answered: Answered::Unmatched,
        held_by: "Ingot",
        name: "ingot of copper",
        payload: r#"{"metal":"copper","weight":1}"#,
    },
    read("bullion as declared", "Bullion", r#"["bar",400]"#),
    refused(
        "bullion of other values",
        "Bullion",
        r#"["coin",399]"#,
        &[
            "'0': not the literal \"bar\", got coin",
            "'1': not the literal 400, got 399",
        ],
    ),
    read(
        "touchmark as declared",
        "Touchmark",
        r#"{"die":"crown","marks":["lion"],"punches":[3,3],"serial":7,"tilt":-1}"#,
    ),
    read(
        "touchmark with no die and no marks",
        "Touchmark",
        r#"{"die":null,"punches":[],"serial":7,"tilt":-1}"#,
    ),
    refused(
        "touchmark of other values",
        "Touchmark",
        r#"{"die":"anchor","marks":["lion","anchor"],"punches":[3,4],"serial":8,"tilt":1}"#,
        &[
            "'die': not the literal \"crown\", got anchor",
            "'marks.1': not the literal \"lion\", got anchor",
            "'punches.1': not the literal 3, got 4",
            "'serial': not the literal 7, got 8",
            "'tilt': not the literal -1, got 1",
        ],
    ),
];

/// [`Hallmark`] as declared, every member at its literal.
const HALLMARK: &str = r#"{"fineness":925,"grade":"sterling","marks":["lion","lion"],"offset":-3,"purity":0.5,"stamped":true,"weight":1.25}"#;

/// What a read answers of a payload.
#[derive(Clone, Copy)]
pub enum Answered {
    Read,
    /// Refused, each violation `validate()` reports of the value in its order.
    Refused(&'static [&'static str]),
    /// Refused by an untagged enum, no variant of which reads the payload: each target says so in
    /// words of its own.
    Unmatched,
}

/// One payload, the model it is read as, and what the read answers.
#[derive(Clone, Copy)]
pub struct LiteralRead {
    pub answered: Answered,
    pub held_by: &'static str,
    pub name: &'static str,
    pub payload: &'static str,
}

/// A variant of each shape under no tag attribute, a literal on each of its members.
#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub enum Assay {
    Sealed(#[model_schema_prop(literal = "wax")] String),
    Struck {
        #[model_schema_prop(literal = "die")]
        tool: String,
    },
    Weighed(
        #[model_schema_prop(literal = 31)] u8,
        #[model_schema_prop(literal = true)] bool,
    ),
}

/// The variants of [`Assay`] under a tag and a content key.
#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "body")]
pub enum AdjacentAssay {
    Sealed(#[model_schema_prop(literal = "wax")] String),
    Struck {
        #[model_schema_prop(literal = "die")]
        tool: String,
    },
    Weighed(
        #[model_schema_prop(literal = 31)] u8,
        #[model_schema_prop(literal = true)] bool,
    ),
}

/// A literal on each slot of a tuple struct.
#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Bullion(
    #[model_schema_prop(literal = "bar")] pub String,
    #[model_schema_prop(literal = 400)] pub u16,
);

/// A literal of each kind on a struct's members, one under an `Option` and one on each item of a
/// list.
#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Hallmark {
    #[model_schema_prop(literal = 925)]
    pub fineness: u16,
    #[model_schema_prop(literal = "sterling")]
    pub grade: String,
    #[model_schema_prop(literal = "lion")]
    pub marks: Vec<String>,
    #[model_schema_prop(literal = -3)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<i32>,
    #[model_schema_prop(literal = 0.5)]
    pub purity: f64,
    #[model_schema_prop(literal = true)]
    pub stamped: bool,
    #[model_schema_prop(literal = 1.25)]
    pub weight: f32,
}

/// Two variants of one shape, told apart by the literal each member holds.
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Ingot {
    Gold {
        #[model_schema_prop(literal = "gold")]
        metal: String,
        weight: u8,
    },
    Silver {
        #[model_schema_prop(literal = "silver")]
        metal: String,
        weight: u8,
    },
}

/// A literal under `nullable`, one on each item of an `Option` of a list and of a list of
/// numbers, and one on each width of integer a client compares in a way of its own.
#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Touchmark {
    #[model_schema_prop(literal = "crown", nullable)]
    pub die: Option<String>,
    #[model_schema_prop(literal = "lion")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marks: Option<Vec<String>>,
    #[model_schema_prop(literal = 3)]
    pub punches: Vec<i64>,
    #[model_schema_prop(literal = 7)]
    pub serial: u32,
    #[model_schema_prop(literal = -1)]
    pub tilt: i16,
}

/// The variants of [`Assay`] that write named members, under a tag.
#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum TaggedAssay {
    Rubbed {
        #[model_schema_prop(literal = 2)]
        passes: u8,
    },
    Struck {
        #[model_schema_prop(literal = "die")]
        tool: String,
    },
}

/// Whether a target that names each violation of a refused row names those of `row`: a row an
/// untagged enum refuses for want of a variant is refused in words of the target's own.
pub const fn names_each_violation(row: &LiteralRead) -> bool {
    matches!(row.answered, Answered::Refused(_))
}

/// What a target prints of [`LITERAL_READS`]: each row's name and whether it is read, then each
/// violation it is refused with where `names` says the target names them.
pub fn literal_report(names: fn(&LiteralRead) -> bool) -> String {
    let mut printed = String::new();
    for row in &LITERAL_READS {
        match row.answered {
            Answered::Read => {
                let _ = writeln!(printed, "{}: read", row.name);
            }
            Answered::Refused(violations) => {
                let _ = writeln!(printed, "{}: refused", row.name);
                if names(row) {
                    for violation in violations {
                        let _ = writeln!(printed, "    {violation}");
                    }
                }
            }
            Answered::Unmatched => {
                let _ = writeln!(printed, "{}: refused", row.name);
            }
        }
    }
    printed
}

/// A row the read takes.
const fn read(name: &'static str, held_by: &'static str, payload: &'static str) -> LiteralRead {
    LiteralRead {
        answered: Answered::Read,
        held_by,
        name,
        payload,
    }
}

/// A row the read refuses with `violations`.
const fn refused(
    name: &'static str,
    held_by: &'static str,
    payload: &'static str,
    violations: &'static [&'static str],
) -> LiteralRead {
    LiteralRead {
        answered: Answered::Refused(violations),
        held_by,
        name,
        payload,
    }
}

/// What serde's read of `payload` as the model `held_by` answers: `Ok` where it reads it, and the
/// read's refusal where it does not.
pub fn serde_read(held_by: &str, payload: &str) -> Result<(), String> {
    /// The read of `payload` as `Read`, its refusal as text.
    fn of<Read>(payload: &str) -> Result<(), String>
    where
        Read: DeserializeOwned,
    {
        serde_json::from_str::<Read>(payload)
            .map(|_| ())
            .map_err(|refusal| refusal.to_string())
    }
    match held_by {
        "AdjacentAssay" => of::<AdjacentAssay>(payload),
        "Assay" => of::<Assay>(payload),
        "Bullion" => of::<Bullion>(payload),
        "Hallmark" => of::<Hallmark>(payload),
        "Ingot" => of::<Ingot>(payload),
        "Touchmark" => of::<Touchmark>(payload),
        "TaggedAssay" => of::<TaggedAssay>(payload),
        other => Err(format!("no model {other}")),
    }
}
