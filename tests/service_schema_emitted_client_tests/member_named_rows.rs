//! Types whose checks and functions are named after a member of their own type, and what Rust
//! answers of each value and payload the emitted clients are held to.

use alloc::collections::BTreeMap;
use core::fmt::Write as _;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::model_schema;

macro_rules! judged {
    ($name:expr, $read_as:expr, $value:expr) => {
        validated($name, $read_as, &$value, $value.validate())
    };
}

/// A plain enum whose check and function are named after its variants.
#[model_schema(check = ["rated"], preprocess = ["unrated"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Rating {
    Rated,
    Unrated,
}

/// A struct whose check is named after a member and whose function is named after another, with a
/// check and a function on members of their own names.
#[model_schema(check = ["label"], preprocess = ["note"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Parcel {
    #[model_schema_prop(check = ["code"])]
    pub code: String,
    pub label: String,
    pub note: String,
    #[model_schema_prop(preprocess = ["seal"])]
    pub seal: String,
}

/// A struct whose check shares its name with the `validate()` the emitter writes.
#[model_schema(check = ["validate"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Gauge {
    pub size: u32,
}

/// A brand whose check shares its name with the member that holds its value.
#[model_schema(check = ["value"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Stamp(pub String);

/// A second type whose check shares its name with one of its members.
#[model_schema(check = ["tag"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Crate {
    pub tag: String,
}

/// An enum whose variant holds a member the check is named after.
#[model_schema(check = ["title"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Freight {
    Free,
    Held { title: String },
}

/// A generic type whose check is named after its member, and a member held under an `Option` whose
/// check is named after it.
#[model_schema(default_types(CodeType = String), check = ["codes"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Hold<CodeType> {
    pub codes: Vec<CodeType>,
    #[model_schema_prop(check = ["spare"])]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare: Option<Vec<String>>,
}

/// A struct whose tuple member carries a check named after it.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Couple {
    #[model_schema_prop(check = ["pair"])]
    pub pair: (String, i32),
}

/// A struct whose map member, keyed by an integer, carries a check named after it.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Ledger {
    #[model_schema_prop(check = ["entries"])]
    pub entries: BTreeMap<u32, String>,
}

/// A type whose check meets no member.
#[model_schema(check = ["ruled"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Plain {
    pub size: u32,
}

/// How a row is run: its value judged and each message printed, or its payload read and written
/// back.
pub enum Kind {
    Read,
    Validate,
}

/// One payload, the type each client reads it as, and what Rust prints of it.
pub struct Row {
    pub expected: String,
    pub kind: Kind,
    pub name: &'static str,
    pub payload: String,
    pub read_as: &'static str,
}

fn refused(message: &str) -> Result<(), Vec<String>> {
    Err(vec![message.to_owned()])
}

fn rated(value: &Rating) -> Result<(), Vec<String>> {
    if matches!(value, Rating::Rated) {
        return refused("a rating is unrated");
    }
    Ok(())
}

fn unrated(payload: Value) -> Value {
    if let Value::String(text) = payload {
        Value::String(text.trim().to_owned())
    } else {
        payload
    }
}

fn label(value: &Parcel) -> Result<(), Vec<String>> {
    if value.label.is_empty() {
        return refused("a parcel is labelled");
    }
    Ok(())
}

fn code(value: &String) -> Result<(), Vec<String>> {
    if value == "dup" {
        return refused("a code is not dup");
    }
    Ok(())
}

fn note(payload: Value) -> Value {
    if let Value::Object(mut map) = payload {
        map.entry("note").or_insert_with(|| json!("none"));
        Value::Object(map)
    } else {
        payload
    }
}

fn seal(payload: Value) -> Value {
    unrated(payload)
}

fn validate(value: &Gauge) -> Result<(), Vec<String>> {
    if value.size > 9 {
        return refused("a gauge is at most 9");
    }
    Ok(())
}

fn value(value: &Stamp) -> Result<(), Vec<String>> {
    if value.0.is_empty() {
        return refused("a stamp is not empty");
    }
    Ok(())
}

fn tag(value: &Crate) -> Result<(), Vec<String>> {
    if value.tag == "x" {
        return refused("a crate is not x");
    }
    Ok(())
}

fn title(value: &Freight) -> Result<(), Vec<String>> {
    if matches!(value, Freight::Held { title } if title.is_empty()) {
        return refused("a freight is titled");
    }
    Ok(())
}

fn codes<CodeType>(value: &Hold<CodeType>) -> Result<(), Vec<String>> {
    if value.codes.is_empty() {
        return refused("a hold has a code");
    }
    Ok(())
}

fn spare(value: &[String]) -> Result<(), Vec<String>> {
    if value.len() > 1 {
        return refused("a hold has one spare");
    }
    Ok(())
}

fn pair(value: &(String, i32)) -> Result<(), Vec<String>> {
    if value.1 < 0_i32 {
        return refused("a pair is not negative");
    }
    Ok(())
}

fn entries(value: &BTreeMap<u32, String>) -> Result<(), Vec<String>> {
    if value.is_empty() {
        return refused("a ledger has an entry");
    }
    Ok(())
}

fn ruled(value: &Plain) -> Result<(), Vec<String>> {
    if value.size == 0 {
        return refused("a plain is sized");
    }
    Ok(())
}

fn validated<T>(
    name: &'static str,
    read_as: &'static str,
    value: &T,
    answered: Result<(), Vec<String>>,
) -> Row
where
    T: Serialize,
{
    let mut expected = String::new();
    match answered {
        Ok(()) => {
            let _ = writeln!(expected, "{name}: passes");
        }
        Err(messages) => {
            let _ = writeln!(expected, "{name}: refused");
            for message in messages {
                let _ = writeln!(expected, "    {message}");
            }
        }
    }
    Row {
        expected,
        kind: Kind::Validate,
        name,
        payload: serde_json::to_string(value).unwrap(),
        read_as,
    }
}

fn read<T>(name: &'static str, read_as: &'static str, payload: &str) -> Row
where
    T: DeserializeOwned + Serialize,
{
    let held = serde_json::from_str::<T>(payload).unwrap();
    Row {
        expected: format!("{name}: read {}\n", serde_json::to_string(&held).unwrap()),
        kind: Kind::Read,
        name,
        payload: payload.to_owned(),
        read_as,
    }
}

fn parcel(code: &str, label: &str) -> Parcel {
    Parcel {
        code: code.to_owned(),
        label: label.to_owned(),
        note: "n".to_owned(),
        seal: "s".to_owned(),
    }
}

fn titled(title: &str) -> Freight {
    Freight::Held {
        title: title.to_owned(),
    }
}

fn held(codes: &[&str], spare: Option<&[&str]>) -> Hold<String> {
    Hold {
        codes: codes.iter().map(|&code| code.to_owned()).collect(),
        spare: spare.map(|spares| spares.iter().map(|&one| one.to_owned()).collect()),
    }
}

fn named(tag: &str) -> Crate {
    Crate {
        tag: tag.to_owned(),
    }
}

fn variant_and_member_rows() -> Vec<Row> {
    let (rated_value, unrated_value) = (Rating::Rated, Rating::Unrated);
    let (sound, unlabelled, repeated, both) = (
        parcel("c", "l"),
        parcel("c", ""),
        parcel("dup", "l"),
        parcel("dup", ""),
    );
    vec![
        judged!("a variant named as the check, rated", "Rating", rated_value),
        judged!(
            "a variant named as the check, unrated",
            "Rating",
            unrated_value
        ),
        judged!("a member named as the check, passes", "Parcel", sound),
        judged!("a member named as the check, refused", "Parcel", unlabelled),
        judged!(
            "a check on a member named as the member",
            "Parcel",
            repeated
        ),
        judged!("both checks of a parcel", "Parcel", both),
    ]
}

fn emitted_member_rows() -> Vec<Row> {
    let (small, large) = (Gauge { size: 3 }, Gauge { size: 10 });
    let (stamped, blank) = (Stamp("a".to_owned()), Stamp(String::new()));
    let (crate_y, crate_x) = (named("y"), named("x"));
    let (sized, empty) = (Plain { size: 1 }, Plain { size: 0 });
    vec![
        judged!("a check named validate, passes", "Gauge", small),
        judged!("a check named validate, refused", "Gauge", large),
        judged!("a brand check named value, passes", "Stamp", stamped),
        judged!("a brand check named value, refused", "Stamp", blank),
        judged!(
            "a second type with a check named as its member, passes",
            "Crate",
            crate_y
        ),
        judged!(
            "a second type with a check named as its member, refused",
            "Crate",
            crate_x
        ),
        judged!("a check that meets no member, passes", "Plain", sized),
        judged!("a check that meets no member, refused", "Plain", empty),
    ]
}

fn held_member_rows() -> Vec<Row> {
    let (positive, negative) = (
        Couple {
            pair: ("a".to_owned(), 1_i32),
        },
        Couple {
            pair: ("a".to_owned(), -1_i32),
        },
    );
    let (filled, bare) = (
        Ledger {
            entries: BTreeMap::from([(1, "x".to_owned())]),
        },
        Ledger {
            entries: BTreeMap::new(),
        },
    );
    vec![
        judged!(
            "a tuple member with a check named as it, passes",
            "Couple",
            positive
        ),
        judged!(
            "a tuple member with a check named as it, refused",
            "Couple",
            negative
        ),
        judged!(
            "an integer-keyed map member with a check named as it, passes",
            "Ledger",
            filled
        ),
        judged!(
            "an integer-keyed map member with a check named as it, refused",
            "Ledger",
            bare
        ),
    ]
}

fn shaped_rows() -> Vec<Row> {
    let (with_title, untitled, free) = (titled("t"), titled(""), Freight::Free);
    let (coded, codeless, spares) = (
        held(&["a"], None),
        held(&[], Some(&["x", "y"])),
        held(&["a"], Some(&["x", "y"])),
    );
    vec![
        judged!(
            "a variant member named as the check, passes",
            "Freight",
            with_title
        ),
        judged!(
            "a variant member named as the check, refused",
            "Freight",
            untitled
        ),
        judged!("a variant with no member of that name", "Freight", free),
        judged!(
            "a generic type with a check named as its member, passes",
            "Hold<String>",
            coded
        ),
        judged!(
            "a generic type with a check named as its member, refused",
            "Hold<String>",
            codeless
        ),
        judged!(
            "a member under an option with a check named as it",
            "Hold<String>",
            spares
        ),
    ]
}

/// The rows of the table: a check named after a variant, a member, a method the emitter writes
/// and the member that holds a brand's value, a function named after a variant and after a
/// member, and a check that meets no member.
pub fn rows() -> Vec<Row> {
    let mut table = variant_and_member_rows();
    table.extend(emitted_member_rows());
    table.extend(shaped_rows());
    table.extend(held_member_rows());
    table.push(read::<Rating>(
        "a function named as a variant",
        "Rating",
        r#""  Unrated ""#,
    ));
    table.push(read::<Parcel>(
        "functions named as members",
        "Parcel",
        r#"{"code":"c","label":"l","seal":"  s  "}"#,
    ));
    table
}

/// What Rust prints of every row.
pub fn expected(rows: &[Row]) -> String {
    rows.iter().map(|row| row.expected.as_str()).collect()
}

#[cfg(test)]
mod tests {
    use super::{expected, rows};

    /// The sentences Rust answers, which every client is held to.
    #[test]
    fn rust_answers_what_each_check_states() {
        assert_eq!(
            expected(&rows()),
            "\
a variant named as the check, rated: refused
    a rating is unrated
a variant named as the check, unrated: passes
a member named as the check, passes: passes
a member named as the check, refused: refused
    a parcel is labelled
a check on a member named as the member: refused
    'code': a code is not dup
both checks of a parcel: refused
    'code': a code is not dup
    a parcel is labelled
a check named validate, passes: passes
a check named validate, refused: refused
    a gauge is at most 9
a brand check named value, passes: passes
a brand check named value, refused: refused
    a stamp is not empty
a second type with a check named as its member, passes: passes
a second type with a check named as its member, refused: refused
    a crate is not x
a check that meets no member, passes: passes
a check that meets no member, refused: refused
    a plain is sized
a variant member named as the check, passes: passes
a variant member named as the check, refused: refused
    a freight is titled
a variant with no member of that name: passes
a generic type with a check named as its member, passes: passes
a generic type with a check named as its member, refused: refused
    'spare': a hold has one spare
    a hold has a code
a member under an option with a check named as it: refused
    'spare': a hold has one spare
a tuple member with a check named as it, passes: passes
a tuple member with a check named as it, refused: refused
    'pair': a pair is not negative
an integer-keyed map member with a check named as it, passes: passes
an integer-keyed map member with a check named as it, refused: refused
    'entries': a ledger has an entry
a function named as a variant: read \"Unrated\"
functions named as members: read {\"code\":\"c\",\"label\":\"l\",\"note\":\"none\",\"seal\":\"s\"}
"
        );
    }
}
