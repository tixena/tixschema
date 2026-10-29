//! The models whose Zod schema joins a flattened choice or an internally tagged struct variant,
//! and what serde and `validate()` answer of each payload, which the run under `node` holds the
//! Zod schemas to.

use core::fmt::Write as _;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

pub const CHECKED_REPORT: &str = "\
booking, no hours: refused
    'hours': a span is not empty
booking, hours: passes
booking, no days: refused
    'days': a day label is not blank
booking, nine days: refused
    'days': a span is under nine days
booking, days: passes
tenancy, no hours: refused
    'hours': a span is not empty
tenancy, no days: refused
    'days': a day label is not blank
tenancy, days: passes
";

pub const READ_REPORT: &str = r#"slip: a payload two members read: read {"dock":"d","label":"q","pier":3}
slip: the second member alone: read {"dock":"north","label":"q","line":"l"}
slip: the first member alone: read {"dock":"","label":"q","pier":3}
slip: no label: refused
wharfage: a payload two members read: read {"dock":"d","kind":"Moored","label":"q","pier":3}
wharfage: no member reads it: refused
wharf: a payload two members read: read {"dock":"d","label":"q","pier":3}
mole: a payload two members read: read {"dock":"d","label":"q","pier":3}
mole: the second member alone: read {"dock":"north","label":"q","line":"l"}
pause: the long member: read {"hours":2,"number":1}
pause: the short member: read {"minutes":2,"number":1}
pause: no member reads it: refused
deck: a payload two members read: read {"dock":"d","label":"q","other":""}
deck: a payload both read whole: read {"dock":"d","label":"q","other":"o"}
deck: no member reads it: refused
block: a paragraph: read {"li":[],"t":"paragraph"}
block: a paragraph of lines: read {"li":["a","b"],"t":"paragraph"}
block: a key no variant names: refused
block: the tag of another variant: refused
block: an unknown tag: refused
block: no tag: refused
block: a rule: read {"t":"rule"}
block: a quote of a quote: read {"blocks":[{"li":["a"],"t":"paragraph"},{"blocks":[{"t":"rule"}],"t":"quote"}],"t":"quote"}
block: a quote of a refused block: refused
"#;

pub const ROWS: &[Row] = &[
    Row {
        zod_refuses: false,
        name: "slip: a payload two members read",
        schema: "Slip",
        payload: r#"{"label":"q","dock":"d","pier":3}"#,
    },
    Row {
        zod_refuses: false,
        name: "slip: the second member alone",
        schema: "Slip",
        payload: r#"{"label":"q","line":"l"}"#,
    },
    Row {
        zod_refuses: false,
        name: "slip: the first member alone",
        schema: "Slip",
        payload: r#"{"label":"q","pier":3}"#,
    },
    Row {
        zod_refuses: false,
        name: "slip: no label",
        schema: "Slip",
        payload: r#"{"dock":"d","pier":3}"#,
    },
    Row {
        zod_refuses: false,
        name: "wharfage: a payload two members read",
        schema: "Wharfage",
        payload: r#"{"kind":"Moored","label":"q","dock":"d","pier":3}"#,
    },
    Row {
        zod_refuses: false,
        name: "wharfage: no member reads it",
        schema: "Wharfage",
        payload: r#"{"kind":"Moored","label":"q","dock":"d","pier":"x"}"#,
    },
    Row {
        zod_refuses: false,
        name: "wharf: a payload two members read",
        schema: "Wharf",
        payload: r#"{"label":"q","dock":"d","pier":3}"#,
    },
    Row {
        zod_refuses: false,
        name: "mole: a payload two members read",
        schema: "Mole",
        payload: r#"{"label":"q","dock":"d","pier":3}"#,
    },
    Row {
        zod_refuses: false,
        name: "mole: the second member alone",
        schema: "Mole",
        payload: r#"{"label":"q","line":"l"}"#,
    },
    Row {
        zod_refuses: false,
        name: "pause: the long member",
        schema: "Pause",
        payload: r#"{"number":1,"hours":2}"#,
    },
    Row {
        zod_refuses: false,
        name: "pause: the short member",
        schema: "Pause",
        payload: r#"{"number":1,"minutes":2}"#,
    },
    Row {
        zod_refuses: false,
        name: "pause: no member reads it",
        schema: "Pause",
        payload: r#"{"number":1}"#,
    },
    Row {
        zod_refuses: false,
        name: "deck: a payload two members read",
        schema: "Deck",
        payload: r#"{"label":"q","dock":"d","other":""}"#,
    },
    Row {
        zod_refuses: false,
        name: "deck: a payload both read whole",
        schema: "Deck",
        payload: r#"{"label":"q","dock":"d","other":"o"}"#,
    },
    Row {
        zod_refuses: false,
        name: "deck: no member reads it",
        schema: "Deck",
        payload: r#"{"label":"q","dock":"d"}"#,
    },
    Row {
        zod_refuses: false,
        name: "block: a paragraph",
        schema: "Block",
        payload: r#"{"t":"paragraph","li":[]}"#,
    },
    Row {
        zod_refuses: false,
        name: "block: a paragraph of lines",
        schema: "Block",
        payload: r#"{"t":"paragraph","li":["a","b"]}"#,
    },
    Row {
        zod_refuses: true,
        name: "block: a key no variant names",
        schema: "Block",
        payload: r#"{"t":"paragraph","li":[],"x":1}"#,
    },
    Row {
        zod_refuses: true,
        name: "block: the tag of another variant",
        schema: "Block",
        payload: r#"{"t":"rule","li":[]}"#,
    },
    Row {
        zod_refuses: false,
        name: "block: an unknown tag",
        schema: "Block",
        payload: r#"{"t":"heading","li":[]}"#,
    },
    Row {
        zod_refuses: false,
        name: "block: no tag",
        schema: "Block",
        payload: r#"{"li":[]}"#,
    },
    Row {
        zod_refuses: false,
        name: "block: a rule",
        schema: "Block",
        payload: r#"{"t":"rule"}"#,
    },
    Row {
        zod_refuses: false,
        name: "block: a quote of a quote",
        schema: "Block",
        payload: r#"{"t":"quote","blocks":[{"t":"paragraph","li":["a"]},{"t":"quote","blocks":[{"t":"rule"}]}]}"#,
    },
    Row {
        zod_refuses: true,
        name: "block: a quote of a refused block",
        schema: "Block",
        payload: r#"{"t":"quote","blocks":[{"t":"paragraph","li":[],"x":1}]}"#,
    },
];

/// An internally tagged enum whose variants hold structs, one of which holds the enum.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "t", rename_all = "kebab-case")]
pub enum Block {
    Paragraph(Paragraph),
    Quote(Quote),
    Rule,
}

/// A flattened choice that names no check.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Gap {
    Long { hours: u32 },
    Short { minutes: u32 },
}

/// An untagged enum that names a `preprocess`, whose members both read a payload that holds a
/// refused `other`.
#[model_schema(preprocess = ["asGiven"])]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Hatch {
    Opened {
        dock: String,
        other: String,
    },
    Sealed {
        dock: String,
        #[model_schema_prop(minLength = 1)]
        other: String,
    },
}

/// An untagged enum whose second member is read as its defaults, so a holder joins its binding
/// whole.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Moorage {
    Docked {
        #[serde(default)]
        dock: String,
        pier: u32,
    },
    Moored {
        #[serde(default = "north_dock")]
        dock: String,
        #[model_schema_prop(minLength = 1)]
        #[serde(default)]
        line: String,
    },
}

/// An untagged enum with a check of its own and a check on a member of a variant.
#[model_schema(check = ["aSpan"])]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Span {
    Days {
        #[model_schema_prop(check = ["notBlank"])]
        days: String,
    },
    Hours {
        hours: u32,
    },
}

/// A variant of another enum that flattens the checked choice.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "kind")]
pub enum Tenancy {
    Booked {
        number: u32,
        #[serde(flatten)]
        span: Span,
    },
    Open,
}

/// An untagged enum one variant of which holds the untagged enum read as defaults.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Tethered {
    Held(Moorage),
    Named { harbour: String },
}

/// A variant of another enum that flattens the untagged enum read as defaults.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "kind")]
pub enum Wharfage {
    Moored {
        label: String,
        #[serde(flatten)]
        berth: Moorage,
    },
    Open,
}

/// The untagged enum read as defaults under another name.
#[model_schema()]
pub type Berthage = Moorage;

#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Booking {
    pub number: u32,
    #[serde(flatten)]
    pub span: Span,
}

/// One value, the payload serde writes for it, and what `validate()` answers of it.
pub struct Checked {
    pub answered: Result<(), Vec<String>>,
    pub name: &'static str,
    pub payload: String,
    pub schema: &'static str,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Deck {
    #[serde(flatten)]
    pub hatch: Hatch,
    pub label: String,
}

/// A struct that flattens the enum that holds the one read as defaults.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Mole {
    pub label: String,
    #[serde(flatten)]
    pub tether: Tethered,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Paragraph {
    #[serde(rename = "li")]
    pub lines: Vec<String>,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Pause {
    #[serde(flatten)]
    pub gap: Gap,
    pub number: u32,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Quote {
    pub blocks: Vec<Block>,
}

/// One payload, the schema it is read by, and the type serde reads it as.
pub struct Row {
    pub name: &'static str,
    pub payload: &'static str,
    pub schema: &'static str,
    /// Whether the Zod schema refuses a key serde ignores, which every strict object does.
    pub zod_refuses: bool,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Slip {
    #[serde(flatten)]
    pub berth: Moorage,
    pub label: String,
}

/// A struct that flattens the alias of the enum read as defaults.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Wharf {
    #[serde(flatten)]
    pub berth: Berthage,
    pub label: String,
}

fn a_span(value: &Span) -> Result<(), Vec<String>> {
    let refused = match value {
        Span::Days { days } => (days == "9").then_some("'days': a span is under nine days"),
        Span::Hours { hours } => (*hours == 0).then_some("'hours': a span is not empty"),
    };
    refused.map_or(Ok(()), |message| Err(vec![message.to_owned()]))
}

const fn as_given(payload: serde_json::Value) -> serde_json::Value {
    payload
}

fn checked<T>(
    name: &'static str,
    schema: &'static str,
    value: &T,
    answered: Result<(), Vec<String>>,
) -> Checked
where
    T: Serialize,
{
    Checked {
        answered,
        name,
        payload: serde_json::to_string(value).unwrap(),
        schema,
    }
}

pub fn checked_cases() -> Vec<Checked> {
    let booking = |name, span: Span| {
        let value = Booking { number: 1, span };
        let answered = value.validate();
        checked(name, "Booking", &value, answered)
    };
    let tenancy = |name, span: Span| {
        let value = Tenancy::Booked { number: 1, span };
        let answered = value.validate();
        checked(name, "Tenancy", &value, answered)
    };
    vec![
        booking("booking, no hours", Span::Hours { hours: 0 }),
        booking("booking, hours", Span::Hours { hours: 2 }),
        booking(
            "booking, no days",
            Span::Days {
                days: String::new(),
            },
        ),
        booking(
            "booking, nine days",
            Span::Days {
                days: "9".to_owned(),
            },
        ),
        booking(
            "booking, days",
            Span::Days {
                days: "3".to_owned(),
            },
        ),
        tenancy("tenancy, no hours", Span::Hours { hours: 0 }),
        tenancy(
            "tenancy, no days",
            Span::Days {
                days: String::new(),
            },
        ),
        tenancy(
            "tenancy, days",
            Span::Days {
                days: "3".to_owned(),
            },
        ),
    ]
}

/// What `validate()` prints for each of [`checked_cases`].
pub fn checked_report(cases: &[Checked]) -> String {
    let mut printed = String::new();
    for held in cases {
        match &held.answered {
            Ok(()) => writeln!(printed, "{}: passes", held.name).unwrap(),
            Err(messages) => {
                writeln!(printed, "{}: refused", held.name).unwrap();
                for message in messages {
                    writeln!(printed, "    {message}").unwrap();
                }
            }
        }
    }
    printed
}

pub fn north_dock() -> String {
    "north".to_owned()
}

fn not_blank(value: &str) -> Result<(), Vec<String>> {
    if value.is_empty() {
        Err(vec!["a day label is not blank".to_owned()])
    } else {
        Ok(())
    }
}

fn read_as<T>(payload: &str) -> String
where
    T: DeserializeOwned + Serialize,
{
    serde_json::from_str::<T>(payload).map_or_else(
        |_| "refused".to_owned(),
        |read| format!("read {}", sorted(serde_json::to_value(&read).unwrap())),
    )
}

/// What a serde read prints for each row of [`ROWS`], and `refused` for a row whose key the Zod
/// schema refuses and serde ignores.
pub fn read_report() -> String {
    report_of(true)
}

/// What a serde read prints for each row of [`ROWS`], the key it ignores included.
pub fn serde_report() -> String {
    report_of(false)
}

fn report_of(as_zod: bool) -> String {
    let mut printed = String::new();
    for row in ROWS {
        if as_zod && row.zod_refuses {
            writeln!(printed, "{}: refused", row.name).unwrap();
            continue;
        }
        let read = match row.schema {
            "Block" => read_as::<Block>(row.payload),
            "Deck" => read_as::<Deck>(row.payload),
            "Mole" => read_as::<Mole>(row.payload),
            "Pause" => read_as::<Pause>(row.payload),
            "Slip" => read_as::<Slip>(row.payload),
            "Wharf" => read_as::<Wharf>(row.payload),
            "Wharfage" => read_as::<Wharfage>(row.payload),
            other => format!("no model named {other}"),
        };
        writeln!(printed, "{}: {read}", row.name).unwrap();
    }
    printed
}

fn sorted(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(object) => {
            let mut entries: Vec<_> = object.into_iter().collect();
            entries.sort_by(|left, right| left.0.cmp(&right.0));
            serde_json::Value::Object(
                entries
                    .into_iter()
                    .map(|(key, held)| (key, sorted(held)))
                    .collect(),
            )
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.into_iter().map(sorted).collect())
        }
        other @ (serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::String(_)) => other,
    }
}
