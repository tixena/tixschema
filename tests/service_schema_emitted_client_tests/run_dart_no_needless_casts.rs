//! The Dart types of an alias of a map keyed by a string, an alias of a tuple, and an enum under a
//! tag alone whose newtype variants hold structs, passed by the analyzer with no warning and run
//! under the Dart VM against serde.

#![cfg(all(feature = "dart", feature = "serde"))]

use alloc::collections::BTreeMap;
use core::fmt::Write as _;

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use tixschema::model_schema;

use super::run_dart_validate::{dart, module};

/// What prints a row: the value read and written back.
const TOLD: &str = "
void told(Object? Function() read) => print(jsonEncode(read()));
";

/// An enum declared above the struct its variant holds, so the struct is not yet known when the
/// enum is expanded.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "t")]
pub enum Corbel {
    Bracket(Mullion),
}

/// A struct held by the variants and the alias below.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Plinth {
    pub height: u32,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Mullion {
    pub width: u32,
}

#[model_schema()]
pub type Cairnmap = BTreeMap<String, Plinth>;

#[model_schema()]
pub type Cairnpair = (String, u32, Plinth);

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "t", rename_all = "kebab-case")]
pub enum Tessera {
    Chip(Plinth),
    Slab(Plinth),
    Void,
}

/// The type, the payload it reads and what serde answers of it.
fn rows() -> Vec<(&'static str, &'static str, String)> {
    let map = r#"{"a":{"height":1},"b":{"height":2}}"#;
    let pair = r#"["x",3,{"height":4}]"#;
    let slab = r#"{"t":"slab","height":5}"#;
    let void = r#"{"t":"void"}"#;
    let bracket = r#"{"t":"Bracket","width":6}"#;
    vec![
        ("Cairnmap", map, serde_told::<Cairnmap>(map)),
        ("Cairnpair", pair, serde_told::<Cairnpair>(pair)),
        ("Tessera", slab, serde_told::<Tessera>(slab)),
        ("Tessera", void, serde_told::<Tessera>(void)),
        ("Corbel", bracket, serde_told::<Corbel>(bracket)),
    ]
}

/// The payload read and written back by serde.
fn serde_told<T>(json: &str) -> String
where
    T: DeserializeOwned + Serialize,
{
    serde_json::to_string(&serde_json::from_str::<T>(json).unwrap()).unwrap()
}

fn definitions() -> Vec<String> {
    vec![
        mullion_dart::dart_definition(),
        corbel_dart::dart_definition(),
        plinth_dart::dart_definition(),
        cairnmap_dart::dart_definition(),
        cairnpair_dart::dart_definition(),
        tessera_dart::dart_definition(),
    ]
}

/// Each payload is written back by the Dart types as serde writes it back.
#[test]
fn the_types_write_back_what_serde_writes_back() {
    let mut main = String::from("void main() {\n");
    let mut expected = Vec::new();
    for (named, json, serde) in rows() {
        let _ = writeln!(
            main,
            "  told(() => {named}.fromJson(jsonDecode(r'{json}')).toJson());"
        );
        expected.push(serde_json::from_str::<Value>(&serde).unwrap());
    }
    main.push_str("}\n");
    let Some((ran, printed)) = dart(
        "no-needless-casts",
        &[],
        &module(&definitions(), &[TOLD], &main),
    ) else {
        return;
    };
    assert!(ran, "{printed}");
    let written: Vec<Value> = printed
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(written, expected);
}

/// The analyzer passes the types.
#[test]
fn the_types_have_no_issue() {
    let source = module(
        &definitions(),
        &[TOLD],
        "void main() => told(() => Cairnmap.fromJson(const <String, dynamic>{}).toJson());\n",
    );
    let Some((clean, said)) = dart("no-needless-casts-analyze", &["analyze"], &source) else {
        return;
    };
    assert!(clean, "{said}\n--- source ---\n{source}");
    assert!(said.contains("No issues found!"), "{said}");
}
