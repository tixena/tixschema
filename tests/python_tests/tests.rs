//! The text `python_definition()` writes, read with no Python at hand: the names it binds, the
//! members it moves and the keys it reads them under. `run_python.rs` in the emitted-client tests
//! runs the same shapes under an interpreter.

#![cfg(feature = "python")]

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A member named after a Python keyword, one named after a member every class writes, and one
/// named after a soft keyword, published under another name.
#[model_schema(name = "Docket")]
#[derive(Debug, Serialize, Deserialize)]
pub struct DocketRecord {
    pub r#class: String,
    pub r#match: u8,
    pub validate: bool,
}

/// A struct that reads past a key it does not declare.
#[model_schema(nonstrict)]
#[derive(Debug, Serialize, Deserialize)]
pub struct Lenient {
    pub kept: String,
}

/// A plain enum whose variant is named after a Python keyword.
#[model_schema()]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Dial {
    Full,
    None,
}

#[test]
fn a_member_named_after_a_keyword_or_a_generated_member_takes_a_trailing_underscore() {
    let text = docket_record_python::python_definition();
    assert!(text.contains("    class_: str\n"), "{text}");
    assert!(text.contains("    validate_: bool\n"), "{text}");
    assert!(text.contains("    match: int\n"), "{text}");
    assert!(
        text.contains(r#"member_class_ = DocketDraft._tix_at("class", "#),
        "{text}"
    );
    assert!(
        text.contains("            class_=member_class_,\n"),
        "{text}"
    );
}

#[test]
fn a_renamed_type_is_written_under_its_name_and_bound_to_its_rust_ident() {
    let text = docket_record_python::python_definition();
    assert!(text.contains("class DocketDraft:\n"), "{text}");
    assert!(text.contains("class Docket:\n"), "{text}");
    assert!(
        text.ends_with("\n\n\nDocketRecord = Docket\nDocketRecordDraft = DocketDraft"),
        "{text}"
    );
}

#[test]
fn every_module_a_type_imports_is_bound_under_its_published_name() {
    let text = docket_record_python::python_definition();
    let imports: Vec<&str> = text
        .lines()
        .filter(|line| line.starts_with("import "))
        .collect();
    assert!(!imports.is_empty(), "{text}");
    for import in imports {
        let (module, bound) = import
            .trim_start_matches("import ")
            .split_once(" as ")
            .unwrap();
        assert_eq!(bound, format!("_Docket_{module}"), "{text}");
    }
}

#[test]
fn a_nonstrict_struct_reads_past_a_key_it_does_not_declare() {
    let strict = docket_record_python::python_definition();
    let lenient = lenient_python::python_definition();
    assert!(strict.contains("unknown field `"), "{strict}");
    assert!(!lenient.contains("unknown field `"), "{lenient}");
    assert!(lenient.contains("duplicate field `"), "{lenient}");
}

#[test]
fn a_variant_named_after_a_keyword_takes_a_trailing_underscore_and_keeps_its_wire_name() {
    let text = dial_python::python_definition();
    assert!(text.contains("    None_ = \"none\"\n"), "{text}");
    assert!(text.contains("    Full = \"full\"\n"), "{text}");
}

#[test]
fn serde_writes_the_keys_and_the_names_the_python_reads() {
    let docket = DocketRecordDraft {
        r#class: "c".to_owned(),
        r#match: 1,
        validate: true,
    };
    let lenient = LenientDraft {
        kept: "k".to_owned(),
    };
    assert_eq!(
        serde_json::to_string(&docket).unwrap(),
        r#"{"class":"c","match":1,"validate":true}"#
    );
    assert_eq!(serde_json::to_string(&lenient).unwrap(), r#"{"kept":"k"}"#);
    assert_eq!(serde_json::to_string(&Dial::None).unwrap(), r#""none""#);
    assert_eq!(serde_json::to_string(&Dial::Full).unwrap(), r#""full""#);
}
