//! The `of` factory of a valid Dart type, which builds the draft from its members and judges it,
//! held to what the analyzer accepts and what the run prints.

#![cfg(feature = "dart")]

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::run_dart_valid_types::dart_files;
use super::run_dart_validate::module;

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DockTime {
    #[model_schema_prop(maximum = 23)]
    pub hour: u8,
    #[model_schema_prop(maximum = 59)]
    pub minute: u8,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DockFlag {
    pub advanced: bool,
    #[model_schema_prop(literal = true)]
    pub is_calculated: bool,
    #[model_schema_prop(literal = "dock")]
    pub kind: String,
    #[model_schema_prop(literal = 3)]
    pub version: u32,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DockOf {
    pub of: String,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DockStamp;

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DockWrapped(pub String);

fn models() -> String {
    module(
        &[
            dock_time_dart::dart_definition(),
            dock_flag_dart::dart_definition(),
            dock_of_dart::dart_definition(),
            dock_stamp_dart::dart_definition(),
            dock_wrapped_dart::dart_definition(),
        ],
        &[],
        "",
    )
}

#[test]
fn of_builds_the_valid_value_the_draft_validates() {
    let user = "import 'dart:convert';\nimport 'models.dart';\n\nvoid main() {\n  final DockTime time = DockTime.of(hour: 12, minute: 0);\n  print(jsonEncode(time.toJson()));\n  final DockFlag flag = DockFlag.of(advanced: false);\n  print(jsonEncode(flag.toJson()));\n  final DockOf named = DockOf.of(of_: 'x');\n  print(jsonEncode(named.toJson()));\n  print(jsonEncode(DockStamp.of().toJson()));\n  print(jsonEncode(DockWrapped.of('w').toJson()));\n}\n";
    let Some((ran, printed)) = dart_files(
        "of-builds",
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
            "{}\n{}\n{}\n{}\n{}\n",
            serde_json::to_string(&DockTimeDraft {
                hour: 12,
                minute: 0
            })
            .unwrap(),
            serde_json::to_string(&DockFlagDraft {
                advanced: false,
                is_calculated: true,
                kind: "dock".to_owned(),
                version: 3
            })
            .unwrap(),
            serde_json::to_string(&DockOfDraft { of: "x".to_owned() }).unwrap(),
            serde_json::to_string(&DockStampDraft).unwrap(),
            serde_json::to_string(&DockWrappedDraft("w".to_owned())).unwrap(),
        )
    );
}

#[test]
fn of_throws_what_validate_throws() {
    let refused = DockTimeDraft {
        hour: 24,
        minute: 0,
    }
    .validate()
    .unwrap_err();
    let user = "import 'models.dart';\n\nvoid main() {\n  try {\n    DockTime.of(hour: 24, minute: 0);\n    print('built');\n  } on FormatException catch (error) {\n    print(error.message);\n  }\n}\n";
    let Some((ran, printed)) = dart_files(
        "of-refuses",
        &[],
        &[("models.dart", models()), ("user.dart", user.to_owned())],
        "user.dart",
    ) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, format!("{}\n", refused.join("; ")));
}
