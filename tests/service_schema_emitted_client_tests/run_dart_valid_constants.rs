//! The constants of a valid Dart enum and the `toValid()` of a draft that declares no rule, held to
//! what the analyzer accepts and what the run prints.

#![cfg(feature = "dart")]

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::run_dart_valid_types::dart_files;
use super::run_dart_validate::module;

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

fn models() -> String {
    module(
        &[
            lane_kind_dart::dart_definition(),
            lane_move_dart::dart_definition(),
            lane_stop_dart::dart_definition(),
            lane_note_dart::dart_definition(),
            lane_label_dart::dart_definition(),
            lane_record_dart::dart_definition(),
            lane_stamp_dart::dart_definition(),
        ],
        &[],
        "",
    )
}

#[test]
fn a_unit_variant_is_a_constant_of_the_valid_dart_type() {
    let user = "import 'dart:convert';\nimport 'models.dart';\n\nvoid main() {\n  final LaneKind express = LaneKind.express;\n  print(jsonEncode(express.toJson()));\n  print(express == LaneKind.express);\n  print(express == LaneKind.bulk);\n  print(const <LaneKind>[LaneKind.express, LaneKind.bulk].length);\n  final LaneMove idle = LaneMove.idle;\n  print(jsonEncode(idle.toJson()));\n  print(jsonEncode(LaneMove.parked.toJson()));\n  final LaneStamp stamp = LaneStamp.instance;\n  print(jsonEncode(stamp.toJson()));\n  final LaneRecord record = LaneRecordDraft(kind: LaneKind.express, passed: true).toValid();\n  print(jsonEncode(record.toJson()));\n}\n";
    let Some((ran, printed)) = dart_files(
        "valid-constants",
        &[],
        &[("models.dart", models()), ("user.dart", user.to_owned())],
        "user.dart",
    ) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(
        printed,
        format!(
            "{}\ntrue\nfalse\n2\n{}\n{}\n{}\n{}\n",
            serde_json::to_string(&LaneKind::Express).unwrap(),
            serde_json::to_string(&LaneMove::Idle).unwrap(),
            serde_json::to_string(&LaneMove::Parked).unwrap(),
            serde_json::to_string(&LaneStamp).unwrap(),
            serde_json::to_string(&LaneRecordDraft {
                kind: LaneKind::Express,
                passed: true
            })
            .unwrap(),
        )
    );
}

#[test]
fn a_draft_with_no_rule_converts_with_to_valid() {
    let user = "import 'dart:convert';\nimport 'models.dart';\n\nvoid main() {\n  final LaneNote note = LaneNoteDraft(text: 'a').toValid();\n  print(jsonEncode(note.toJson()));\n  final LaneKind kind = LaneKindDraft.bulk.toValid();\n  print(kind == LaneKind.bulk);\n  final LaneStop parked = LaneStopParked().toValid();\n  print(jsonEncode(parked.toJson()));\n}\n";
    let Some((ran, printed)) = dart_files(
        "valid-to-valid",
        &[],
        &[("models.dart", models()), ("user.dart", user.to_owned())],
        "user.dart",
    ) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(
        printed,
        format!(
            "{}\ntrue\n{}\n",
            serde_json::to_string(&LaneNoteDraft {
                text: "a".to_owned()
            })
            .unwrap(),
            serde_json::to_string(&LaneStop::Parked).unwrap(),
        )
    );
}

#[test]
fn a_draft_that_declares_a_rule_has_no_to_valid() {
    let ruled = LaneLabelDraft {
        text: String::new(),
    };
    assert_eq!(ruled.validation_issues().len(), 1);
    let user = "import 'models.dart';\n\nvoid main() {\n  LaneLabelDraft(text: 'a').toValid();\n  LaneMoveParked().toValid();\n}\n";
    let Some((clean, said)) = dart_files(
        "valid-no-to-valid",
        &["analyze"],
        &[("models.dart", models()), ("user.dart", user.to_owned())],
        "user.dart",
    ) else {
        return;
    };
    assert!(!clean, "{said}");
    assert!(
        said.contains("The method 'toValid' isn't defined for the type 'LaneLabelDraft'."),
        "{said}"
    );
}
