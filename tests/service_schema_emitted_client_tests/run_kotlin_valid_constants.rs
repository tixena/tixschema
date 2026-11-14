//! The constants of a valid Kotlin enum and the `toValid()` of a draft that declares no rule, held
//! to what kotlinc accepts and what the run prints.

#![cfg(feature = "kotlin")]

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::runtime::{ran_kotlin, refused_by_kotlinc_beside};

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

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

fn module(main: &str) -> String {
    [
        KOTLIN_IMPORTS.to_owned(),
        lane_kind_kotlin::kotlin_definition(),
        lane_move_kotlin::kotlin_definition(),
        lane_stop_kotlin::kotlin_definition(),
        lane_note_kotlin::kotlin_definition(),
        lane_label_kotlin::kotlin_definition(),
        lane_record_kotlin::kotlin_definition(),
        lane_stamp_kotlin::kotlin_definition(),
        main.to_owned(),
    ]
    .join("\n\n")
}

#[test]
fn a_unit_variant_is_a_constant_and_a_rule_less_draft_is_valid() {
    let main = r#"
fun main() {
    val express: LaneKind = LaneKind.Express
    println(Json.encodeToString(express))
    println(express == LaneKind.Express)
    println(express == LaneKind.Bulk)
    val idle: LaneMove = LaneMove.Idle
    println(Json.encodeToString(idle))
    val note: LaneNote = LaneNoteDraft("a").toValid()
    println(Json.encodeToString(note))
    val parked: LaneStop = LaneStopParked.toValid()
    println(Json.encodeToString(parked))
    println(LaneKindDraft.Bulk.toValid() == LaneKind.Bulk)
    val stamp: LaneStamp = LaneStamp.instance
    println(Json.encodeToString(stamp))
    val record: LaneRecord = LaneRecordDraft(LaneKind.Express, true).toValid()
    println(Json.encodeToString(record))
}
"#;
    let Some(printed) = ran_kotlin(&module(main)) else {
        return;
    };
    assert_eq!(
        printed,
        format!(
            "{}\ntrue\nfalse\n{}\n{}\n{}\ntrue\n{}\n{}\n",
            serde_json::to_string(&LaneKind::Express).unwrap(),
            serde_json::to_string(&LaneMove::Idle).unwrap(),
            serde_json::to_string(&LaneNoteDraft {
                text: "a".to_owned()
            })
            .unwrap(),
            serde_json::to_string(&LaneStop::Parked).unwrap(),
            serde_json::to_string(&LaneStamp).unwrap(),
            serde_json::to_string(&LaneRecordDraft {
                kind: LaneKind::Express,
                passed: true
            })
            .unwrap(),
        )
    );
    let ruled = LaneLabelDraft {
        text: String::new(),
    };
    assert_eq!(ruled.validation_issues().len(), 1);
}

#[test]
fn a_draft_that_declares_a_rule_has_no_to_valid() {
    let beside = "fun built(draft: LaneLabelDraft) = draft.toValid()";
    let Some(refused) = refused_by_kotlinc_beside(&module("fun main() {}"), Some(beside)) else {
        return;
    };
    assert!(refused.contains("toValid"), "{refused}");
}
