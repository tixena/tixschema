//! The Zod schemas of a holder that names an untagged enum, run by node with the real `zod`, with
//! the enum declared above its holder, below it and in `untagged_members_apart`, another crate: a
//! refrain no variant reads is refused at the member that holds it, as serde names it, and the
//! content of a tagged variant that is the enum still reads the keys beside its tag.

use core::fmt::Write as _;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::model_schema;
use untagged_members_apart::RefrainApart;

use super::runtime::{node_modules, ran_with_modules, stand_down_modules};

const REQUIRED_PACKAGES: &[&str] = &["zod"];

/// The arguments each verse is parsed from, and the tagged line beside them.
const VERSES: [&str; 5] = [
    r#"{"refrain":{}}"#,
    r#"{"refrain":{"beats":"x"}}"#,
    r#"{"refrain":{"text":"a","stray":1}}"#,
    r#"{"refrain":{"text":"a"}}"#,
    r#"{"refrain":{"beats":3}}"#,
];

const LINES: [&str; 3] = [
    r#"{"kind":"Chorus","text":"a"}"#,
    r#"{"kind":"Chorus","beats":3}"#,
    r#"{"kind":"Chorus","text":"a","stray":1}"#,
];

const DRIVER: &str = r#"
const verses = {
  above: VerseAbove$Schema,
  below: VerseBelow$Schema,
  apart: VerseApart$Schema,
};
const lines = {
  above: LineAbove$Schema,
  below: LineBelow$Schema,
  apart: LineApart$Schema,
};
const told = (schema, text) => {
  const read = schema.safeParse(JSON.parse(text));
  return read.success ? "read" : read.error.issues.map((issue) => issue.path.join(".")).join("|");
};
const answers = {};
for (const layout of Object.keys(verses)) {
  answers[layout] = {
    verses: VERSES.map((text) => told(verses[layout], text)),
    lines: LINES.map((text) => told(lines[layout], text)),
  };
}
console.log(JSON.stringify(answers));
"#;

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum RefrainAbove {
    Beats { beats: u32 },
    Words { text: String },
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VerseAbove {
    pub refrain: RefrainAbove,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum LineAbove {
    Chorus(RefrainAbove),
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VerseBelow {
    pub refrain: RefrainBelow,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum LineBelow {
    Chorus(RefrainBelow),
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum RefrainBelow {
    Beats { beats: u32 },
    Words { text: String },
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VerseApart {
    pub refrain: RefrainApart,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum LineApart {
    Chorus(RefrainApart),
}

fn emitted() -> String {
    [
        "import { z, type ZodType, type $brand } from \"zod\";".to_owned(),
        RefrainAbove::ts_definition(),
        RefrainAbove::zod_schema(),
        VerseAbove::ts_definition(),
        VerseAbove::zod_schema(),
        LineAbove::ts_definition(),
        LineAbove::zod_schema(),
        VerseBelow::ts_definition(),
        VerseBelow::zod_schema(),
        LineBelow::ts_definition(),
        LineBelow::zod_schema(),
        RefrainBelow::ts_definition(),
        RefrainBelow::zod_schema(),
        RefrainApart::ts_definition(),
        RefrainApart::zod_schema(),
        VerseApart::ts_definition(),
        VerseApart::zod_schema(),
        LineApart::ts_definition(),
        LineApart::zod_schema(),
    ]
    .join("\n\n")
}

fn listed(items: &[&str]) -> String {
    items.iter().fold(String::new(), |mut written, item| {
        let _ = write!(written, "String.raw`{item}`, ");
        written
    })
}

#[test]
fn a_refrain_no_variant_reads_is_refused_at_its_member_wherever_the_enum_is_declared() {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(
            REQUIRED_PACKAGES,
            "the Zod schemas of an untagged enum's holders",
        );
        return;
    };
    let module = format!(
        "{}\n\nconst VERSES = [{}];\nconst LINES = [{}];\n{DRIVER}",
        emitted(),
        listed(&VERSES),
        listed(&LINES),
    );
    let Some(wrote) = ran_with_modules(
        "named-unions",
        "TIXSCHEMA_NODE",
        "node",
        "named-unions.mts",
        &module,
        &modules,
    ) else {
        return;
    };
    let answers: Value = serde_json::from_str(wrote.trim()).unwrap();
    let verses = json!(["refrain", "refrain", "refrain", "read", "read"]);
    for layout in ["above", "below", "apart"] {
        assert_eq!(answers[layout]["verses"], verses, "{layout}: {answers}");
        assert_eq!(answers[layout]["lines"][0], "read", "{layout}: {answers}");
        assert_eq!(answers[layout]["lines"][1], "read", "{layout}: {answers}");
        assert_eq!(
            answers[layout]["lines"], answers["above"]["lines"],
            "{layout}"
        );
    }
    let serde_reads: Vec<bool> = VERSES
        .iter()
        .map(|text| serde_json::from_str::<VerseAbove>(text).is_ok())
        .collect();
    assert_eq!(serde_reads, [false, false, false, true, true]);
}
