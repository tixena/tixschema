//! The constants of a valid Swift enum and the `valid` of a draft that declares no rule, held to
//! what the compiler accepts and what the run prints.

#![cfg(feature = "swift")]

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::runtime::{checked_together, judged};

const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

const MAIN: &str = r#"
let encoder = JSONEncoder()
encoder.outputFormatting = [.sortedKeys]
let express: LaneKind = .express
print(String(decoding: try encoder.encode(express), as: UTF8.self))
print(express == LaneKind.express)
print(express == LaneKind.bulk)
let idle: LaneMove = .idle
print(String(decoding: try encoder.encode(idle), as: UTF8.self))
let note: LaneNote = LaneNoteDraft(text: "a").valid
print(String(decoding: try encoder.encode(note), as: UTF8.self))
let parked: LaneStop = LaneStopDraft.parked.valid
print(String(decoding: try encoder.encode(parked), as: UTF8.self))
let stamp: LaneStamp = .instance
print(String(decoding: try encoder.encode(stamp), as: UTF8.self))
let record: LaneRecord = LaneRecordDraft(kind: .express, passed: true).valid
print(String(decoding: try encoder.encode(record), as: UTF8.self))
"#;

#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum LaneKind {
    Bulk,
    Express,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum LaneMove {
    Busy {
        #[model_schema_prop(minimum = 1)]
        parcel_count: u32,
    },
    Idle,
    Parked,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum LaneStop {
    Idle,
    Moving { parcel_count: u32 },
    Parked,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LaneNote {
    pub text: String,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LaneRecord {
    pub kind: LaneKind,
    pub passed: bool,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LaneStamp;

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LaneLabel {
    #[model_schema_prop(minLength = 1)]
    pub text: String,
}

fn generated() -> String {
    [
        "import Foundation".to_owned(),
        lane_kind_swift::swift_definition(),
        lane_move_swift::swift_definition(),
        lane_stop_swift::swift_definition(),
        lane_note_swift::swift_definition(),
        lane_label_swift::swift_definition(),
        lane_record_swift::swift_definition(),
        lane_stamp_swift::swift_definition(),
    ]
    .join("\n\n")
}

#[test]
fn a_unit_case_is_a_constant_and_a_rule_less_draft_is_valid() {
    let source = format!("{}\n\n{}", generated(), MAIN);
    let Some(run) = judged(
        "swift-valid-constants",
        RUNTIME_VAR,
        "swift",
        &["-warnings-as-errors"],
        "main.swift",
        &source,
    ) else {
        return;
    };
    assert!(run.succeeded, "{}\n{source}", run.stderr);
    assert_eq!(
        run.stdout.lines().collect::<Vec<_>>(),
        [
            serde_json::to_string(&LaneKind::Express).unwrap().as_str(),
            "true",
            "false",
            serde_json::to_string(&LaneMove::Idle).unwrap().as_str(),
            serde_json::to_string(&LaneNoteDraft {
                text: "a".to_owned()
            })
            .unwrap()
            .as_str(),
            serde_json::to_string(&LaneStop::Parked).unwrap().as_str(),
            serde_json::to_string(&LaneStamp).unwrap().as_str(),
            serde_json::to_string(&LaneRecordDraft {
                kind: LaneKind::Express,
                passed: true
            })
            .unwrap()
            .as_str(),
        ]
    );
    let ruled = LaneLabelDraft {
        text: String::new(),
    };
    assert_eq!(ruled.validation_issues().len(), 1);
}

#[test]
fn a_draft_that_declares_a_rule_has_no_valid() {
    let source = generated();
    let Some((succeeded, said)) = checked_together(
        "swift-valid-no-valid",
        RUNTIME_VAR,
        "swift",
        &["-warnings-as-errors"],
        &[
            ("generated.swift", &source),
            (
                "other.swift",
                "func bad() -> LaneLabel { LaneLabelDraft(text: \"a\").valid }\n",
            ),
        ],
    )
    .map(|run| (run.succeeded, run.stderr)) else {
        return;
    };
    assert!(!succeeded, "{said}");
    assert!(
        said.contains("value of type 'LaneLabelDraft' has no member 'valid'"),
        "{said}"
    );
}
