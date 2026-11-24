//! The emitted Python run under a Python interpreter: each payload read by the types
//! `python_definition()` writes, and what each answers held to what serde answers for the same
//! payload, a value read and written back, or the words and the place of serde's refusal. The same
//! text is checked by `pyright` in strict mode.

#![cfg(feature = "python")]

use alloc::collections::BTreeMap;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_path_to_error::Segment;
use tixschema::model_schema;

use super::runtime::{PYRIGHT_VAR, checked_by_pyright, ran};

/// Names the interpreter the runs use, `python3` on `PATH` where it is unset.
const PYTHON_VAR: &str = "TIXSCHEMA_PYTHON";

/// The parcel every shipment row holds.
const PARCEL: &str = r#"{"label":"box","itemCount":3,"tags":["a"],"size":"Small"}"#;

/// Reads each row, a type and a payload, and prints what the type answers: the value written back,
/// or the words and the place of the refusal.
const DRIVER: &str = r#"
for kind, text in ROWS:
    reader = globals()[kind]
    try:
        print("ok\t" + reader.from_json(text).to_json())
    except ValueError as refused:
        steps = refused.args[1] if len(refused.args) == 2 else []
        print("no\t" + str(refused.args[0]) + "\t" + ".".join(str(step) for step in steps))
"#;

/// A struct of scalars, an optional member, a list and a plain enum, under a container rename.
#[model_schema()]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PyParcel {
    pub item_count: u32,
    pub label: String,
    #[serde(default, skip_serializing_if = "::core::option::Option::is_none")]
    pub note: Option<String>,
    pub size: PySize,
    pub tags: Vec<String>,
}

#[model_schema()]
#[derive(Debug, Serialize, Deserialize)]
pub enum PySize {
    Large,
    Small,
}

/// A plain enum under a variant rename and a container rename, one variant named after a word
/// Python reserves.
#[model_schema()]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PyMode {
    FullSpeed,
    #[serde(rename = "half")]
    HalfSpeed,
    None,
}

/// Every integer width a client reads in range, both floats, nested lists of optional values,
/// maps of models and of optional values, a member serde writes as `null`, and a member named after
/// a word Python reserves, published under another name.
#[model_schema(name = "PyShipment")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PyShipmentRecord {
    pub by_name: HashMap<String, Vec<PyParcel>>,
    pub r#class: String,
    pub grid: Vec<Vec<Option<u8>>>,
    pub mode: PyMode,
    pub narrow: i8,
    pub ratio: f64,
    pub signed: i64,
    pub single: f32,
    pub sorted: BTreeMap<String, Option<i32>>,
    #[model_schema_prop(nullable)]
    pub spare: Option<PyParcel>,
    pub wide: usize,
}

/// A struct that reads past a key it does not declare.
#[model_schema(nonstrict)]
#[derive(Debug, Serialize, Deserialize)]
pub struct PyLoose {
    pub kept: String,
}

/// Every type the rows read, joined into one module as a consumer joins them.
fn bundle() -> String {
    [
        py_size_python::python_definition(),
        py_mode_python::python_definition(),
        py_parcel_python::python_definition(),
        py_shipment_record_python::python_definition(),
        py_loose_python::python_definition(),
    ]
    .into_iter()
    .fold(
        String::from("from __future__ import annotations\n"),
        |mut joined, text| {
            joined.push_str("\n\n");
            joined.push_str(&text);
            joined
        },
    )
}

/// What serde answers for `text` read as `T`, in the form [`DRIVER`] prints: `ok` and the value
/// written back, or `no`, serde's words and the place it refused, a missing field placed at its key
/// as every client and dispatcher places it.
fn serde_answer<T>(text: &str) -> String
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    let mut reader = serde_json::Deserializer::from_str(text);
    match serde_path_to_error::deserialize::<_, T>(&mut reader) {
        Ok(read) => format!("ok\t{}", serde_json::to_string(&read).unwrap()),
        Err(refused) => {
            let whole = refused.inner().to_string();
            let said = whole.split(" at line ").next().unwrap().to_owned();
            let mut steps: Vec<String> = refused
                .path()
                .iter()
                .filter_map(|segment| match segment {
                    Segment::Seq { index } => Some(index.to_string()),
                    Segment::Map { key } => Some(key.clone()),
                    Segment::Enum { variant } => Some(variant.clone()),
                    Segment::Unknown => None,
                })
                .collect();
            if let Some(missing) = said
                .strip_prefix("missing field `")
                .and_then(|rest| rest.strip_suffix('`'))
            {
                steps.push(missing.to_owned());
            }
            format!("no\t{said}\t{}", steps.join("."))
        }
    }
}

/// A shipment whose every member is in range, with each member of `replaced`, the members of a JSON
/// object, written in place of its own.
fn shipment(replaced: &str) -> String {
    let mut members: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&format!(
        r#"{{"wide":18446744073709551615,"narrow":-128,"signed":-9223372036854775808,"ratio":0.1,"single":0.1,"grid":[[1,null],[]],"byName":{{"a":[{PARCEL}]}},"sorted":{{"x":null,"y":-5}},"spare":null,"class":"c","mode":"full-speed"}}"#
    ))
    .unwrap();
    let written: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&format!("{{{replaced}}}")).unwrap();
    members.extend(written);
    serde_json::to_string(&members).unwrap()
}

/// The payloads each read as a parcel.
fn parcel_payloads() -> Vec<String> {
    vec![
        PARCEL.to_owned(),
        r#"{"label":"box","itemCount":3,"note":"n","tags":[],"size":{"Large":null}}"#.to_owned(),
        r#"{"label":"box","itemCount":3.5,"tags":[],"size":"Small"}"#.to_owned(),
        r#"{"label":"box","itemCount":1.0,"tags":[],"size":"Small"}"#.to_owned(),
        r#"{"label":"box","itemCount":-0,"tags":[],"size":"Small"}"#.to_owned(),
        r#"{"label":"box","itemCount":true,"tags":[],"size":"Small"}"#.to_owned(),
        r#"{"label":"box","itemCount":4294967296,"tags":[],"size":"Small"}"#.to_owned(),
        r#"{"label":"box","itemCount":18446744073709551616,"tags":[],"size":"Small"}"#.to_owned(),
        r#"{"label":"box","itemCount":1,"tags":[1],"size":"Small"}"#.to_owned(),
        r#"{"label":"box","itemCount":1,"tags":"x","size":"Small"}"#.to_owned(),
        r#"{"label":null,"itemCount":1,"tags":[],"size":"Small"}"#.to_owned(),
        r#"{"itemCount":1,"tags":[],"size":"Small"}"#.to_owned(),
        r#"{"label":"box","itemCount":1,"tags":[],"size":"Small","stray":1}"#.to_owned(),
        r#"{"label":"box","itemCount":1,"tags":[],"size":"Medium"}"#.to_owned(),
        r#"{"label":"box","itemCount":1,"tags":[],"size":{"Large":1}}"#.to_owned(),
        r#"{"label":"box","itemCount":1,"tags":[],"size":{"Large":null,"Small":null}}"#.to_owned(),
        r#"{"label":"box","itemCount":1,"tags":[],"size":1}"#.to_owned(),
        format!(
            r#"{{"label":"bo{} \"x\"","itemCount":1,"tags":["\n"],"size":"Small"}}"#,
            '\u{eb}'
        ),
        r#"{"label":"a","itemCount":1,"label":"b","tags":[],"size":"Small"}"#.to_owned(),
        r#"{"label":"a","itemCount":1,"tags":[],"size":"Small","stray":1,"stray":2}"#.to_owned(),
        // A value serde's parser refuses, refused at its place in serde's words.
        r#"{"label":"box","itemCount":NaN,"tags":[],"size":"Small"}"#.to_owned(),
        r#"{"label":"box","itemCount":-Infinity,"tags":[],"size":"Small"}"#.to_owned(),
        r#"{"label":"box","itemCount":1,"note":Infinity,"tags":[],"size":"Small"}"#.to_owned(),
        r#"{"label":"box","itemCount":1,"tags":["a",NaN],"size":"Small"}"#.to_owned(),
        r#"{"label":"box","itemCount":1e400,"tags":[],"size":"Small"}"#.to_owned(),
        r#"{"label":"box","itemCount":1,"tags":[],"size":NaN}"#.to_owned(),
        r#"{"label":"box","itemCount":1,"tags":[],"size":{"Large":NaN}}"#.to_owned(),
        "NaN".to_owned(),
        "1e400".to_owned(),
        // Of two faults, the one the text writes first, and a member left out after every other.
        r#"{"size":1,"label":5,"itemCount":1,"tags":[]}"#.to_owned(),
        r#"{"label":5,"stray":1,"itemCount":1,"tags":[],"size":"Small"}"#.to_owned(),
        r#"{"stray":1,"label":5,"itemCount":1,"tags":[],"size":"Small"}"#.to_owned(),
        r#"{"label":"box","tags":[],"size":"Small","note":5}"#.to_owned(),
        r#"{"label":5,"itemCount":1,"label":"b","tags":[],"size":"Small"}"#.to_owned(),
        r#"{"tags":[NaN],"label":5,"itemCount":1,"size":"Small"}"#.to_owned(),
    ]
}

/// The payloads each read as a shipment.
fn shipment_payloads() -> Vec<String> {
    let shipments = [
        shipment(""),
        shipment(r#""spare":{"label":"s","itemCount":0,"tags":[],"size":"Large"}"#),
        shipment(r#""mode":"half""#),
        shipment(r#""mode":"None""#),
        shipment(r#""single":3.4028235e38"#),
        shipment(r#""single":-1e-46"#),
        shipment(r#""ratio":12"#),
        shipment(r#""ratio":1e16"#),
        shipment(r#""narrow":-129"#),
        shipment(r#""signed":9223372036854775808"#),
        shipment(r#""wide":-1"#),
        shipment(r#""grid":[[256]]"#),
        shipment(r#""grid":[[1.5e300]]"#),
        shipment(r#""byName":{"a":[{"label":"x"}]}"#),
        shipment(r#""sorted":{"x":"y"}"#),
        shipment(r#""class":1"#),
        shipment(r#""ratio":1e-5"#),
        shipment(r#""ratio":1.5e-7"#),
        shipment(r#""ratio":1e15"#),
    ];
    // Written by hand, since `shipment` writes each key once, in key order, and only what a JSON
    // value holds. A map reads every value of a key written twice and keeps the last, as serde's
    // does; a `BTreeMap` written out of key order is written back in it; and a value serde's parser
    // refuses is refused at its place.
    let by_hand = [
        (r#""sorted":{"x":null,"y":-5}"#, r#""sorted":{"x":1,"x":2}"#),
        (
            r#""sorted":{"x":null,"y":-5}"#,
            r#""sorted":{"x":"bad","x":2}"#,
        ),
        (
            r#""sorted":{"x":null,"y":-5}"#,
            r#""sorted":{"y":-5,"x":null,"w":1}"#,
        ),
        (r#""grid":[[1,null],[]]"#, r#""grid":[[1e400]]"#),
        (r#""grid":[[1,null],[]]"#, r#""grid":[[1,NaN]]"#),
        (r#""ratio":0.1"#, r#""ratio":NaN"#),
        (r#""single":0.1"#, r#""single":-Infinity"#),
    ]
    .map(|(written, replaced)| {
        let whole = shipment("");
        assert!(whole.contains(written), "{whole} holds {written}");
        whole.replace(written, replaced)
    });
    shipments.into_iter().chain(by_hand).collect()
}

/// Each row: the type it is read as, the payload, and what serde answers for it.
fn rows() -> Vec<(&'static str, String, String)> {
    let loose = [
        r#"{"kept":"k","extra":1}"#.to_owned(),
        r#"{"extra":1}"#.to_owned(),
        r#"{"kept":"k","extra":NaN}"#.to_owned(),
        r#"{"kept":"k","extra":[1,-Infinity]}"#.to_owned(),
        r#"{"kept":"k","extra":{"x":[1e400]}}"#.to_owned(),
        r#"{"extra":1,"extra":2,"kept":"k"}"#.to_owned(),
        r#"{"kept":5,"extra":NaN}"#.to_owned(),
        r#"{"extra":NaN,"kept":5}"#.to_owned(),
    ];
    let sizes = [
        r#""Small""#,
        "NaN",
        r#"{"Large":NaN}"#,
        r#"{"Large":-Infinity}"#,
    ];
    let mut every = Vec::new();
    every.extend(
        parcel_payloads()
            .into_iter()
            .map(|text| ("PyParcel", serde_answer::<PyParcel>(&text), text)),
    );
    every.extend(
        shipment_payloads()
            .into_iter()
            .map(|text| ("PyShipment", serde_answer::<PyShipmentRecord>(&text), text)),
    );
    every.extend(
        loose
            .into_iter()
            .map(|text| ("PyLoose", serde_answer::<PyLoose>(&text), text)),
    );
    every.extend(
        sizes
            .into_iter()
            .map(|text| ("PySize", serde_answer::<PySize>(text), text.to_owned())),
    );
    every
        .into_iter()
        .map(|(kind, answer, text)| (kind, text, answer))
        .collect()
}

#[test]
fn every_payload_reads_in_python_as_serde_reads_it() {
    let every = rows();
    let listed = every
        .iter()
        .map(|(kind, text, _)| format!("    ({kind:?}, {}),", serde_json::to_string(text).unwrap()))
        .collect::<Vec<_>>()
        .join("\n");
    let source = format!("{}\n\n\nROWS = [\n{listed}\n]\n{DRIVER}", bundle());
    let Some(printed) = ran("python", PYTHON_VAR, "python3", "main.py", &source) else {
        return;
    };
    let answered: Vec<&str> = printed.lines().collect();
    assert_eq!(answered.len(), every.len(), "{printed}");
    for ((kind, text, serde), python) in every.iter().zip(answered) {
        assert_eq!(python, serde, "{kind} read {text}");
    }
}

#[test]
fn a_value_built_in_python_is_written_as_serde_writes_it() {
    let built = PyParcelDraft {
        item_count: 7,
        label: "crate".to_owned(),
        note: Some("fragile".to_owned()),
        size: PySize::Large,
        tags: vec!["a".to_owned(), "b".to_owned()],
    };
    let source = format!(
        "{}\n\n\nprint(PyParcel.of(label=\"crate\", item_count=7, note=\"fragile\", tags=[\"a\", \
         \"b\"], size=PySize.Large).to_json())\ntry:\n    PyParcel()\nexcept TypeError as \
         refused:\n    print(refused)\ntry:\n    PyParcel.of(label=\"crate\", item_count=-1, \
         tags=[], size=PySize.Small).to_json()\nexcept ValueError as refused:\n    \
         print(refused.args[0])\n",
        bundle()
    );
    let Some(printed) = ran("python", PYTHON_VAR, "python3", "main.py", &source) else {
        return;
    };
    let lines: Vec<&str> = printed.lines().collect();
    assert_eq!(
        lines,
        [
            serde_json::to_string(&built).unwrap().as_str(),
            "a PyParcel is made by PyParcelDraft.validate(), PyParcel.of() or PyParcel.from_json()",
            "expected an integer from 0 to 4294967295, found -1",
        ]
    );
}

#[test]
fn the_emitted_python_type_checks_under_pyright_strict() {
    let Some((passed, said)) = checked_by_pyright("pyright", &bundle()) else {
        return;
    };
    assert!(
        passed,
        "{PYRIGHT_VAR}: {said}\n--- source ---\n{}",
        bundle()
    );
}
