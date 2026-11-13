//! Tests of a model type declared beside the enum that holds it, under a name the standard
//! library also uses.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Path {
    pub segments: Vec<String>,
}

#[model_schema(minLength = 1)]
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum Edit {
    Move { to: Path },
    Relocate { dir: PathBuf },
    Rename { name: String },
}

fn path(segments: &[&str]) -> Path {
    PathDraft {
        segments: segments.iter().map(|&segment| segment.to_owned()).collect(),
    }
    .validate()
    .unwrap()
}

#[test]
fn the_check_does_not_reach_a_field_typed_with_the_model() {
    assert_eq!(
        EditDraft::Move { to: path(&[]) }.validation_issues(),
        Vec::<String>::new()
    );
    assert_eq!(
        EditDraft::Move { to: path(&["a"]) }.validation_issues(),
        Vec::<String>::new()
    );
}

#[test]
fn the_check_still_reaches_the_standard_types() {
    assert_eq!(
        EditDraft::Rename {
            name: String::new()
        }
        .validation_issues(),
        vec!["'name': too short: minimum length is 1, got 0".to_owned()]
    );
    assert_eq!(
        EditDraft::Relocate {
            dir: PathBuf::new()
        }
        .validation_issues(),
        vec!["'dir': too short: minimum length is 1, got 0".to_owned()]
    );
}

#[cfg(feature = "zod")]
#[test]
fn the_zod_schema_names_the_model() {
    assert!(
        Edit::zod_schema().contains("to: Path$Schema"),
        "{}",
        Edit::zod_schema()
    );
}

#[test]
fn the_read_keeps_the_model_whole() {
    let read: Edit = serde_json::from_value(serde_json::json!({
        "op": "Move", "to": { "segments": [] }
    }))
    .unwrap();
    assert_eq!(read, EditDraft::Move { to: path(&[]) }.validate().unwrap());
}
