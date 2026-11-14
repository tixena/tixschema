//! The `of` function of a valid Swift type, which builds the draft from its members and throws
//! what `validate()` throws, held to what the compiler accepts and what the run prints.

#![cfg(feature = "swift")]

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::runtime::judged;

const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

const MAIN: &str = r#"
let encoder = JSONEncoder()
encoder.outputFormatting = [.sortedKeys]
func show<Value: Encodable>(_ value: Value) throws {
    print(String(decoding: try encoder.encode(value), as: UTF8.self))
}
try show(try DockTime.of(hour: 12, minute: 0))
try show(try DockFlag.of(advanced: false))
try show(try DockOf.of(of: "x"))
try show(try DockStamp.of())
try show(try DockWrapped.of(value: "w"))
do {
    _ = try DockTime.of(hour: 24, minute: 0)
    print("built")
} catch DecodingError.dataCorrupted(let context) {
    print(context.debugDescription)
}
"#;

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

fn generated() -> String {
    [
        "import Foundation".to_owned(),
        dock_time_swift::swift_definition(),
        dock_flag_swift::swift_definition(),
        dock_of_swift::swift_definition(),
        dock_stamp_swift::swift_definition(),
        dock_wrapped_swift::swift_definition(),
    ]
    .join("\n\n")
}

#[test]
fn of_builds_the_valid_value_and_throws_what_validate_throws() {
    let source = format!("{}\n\n{}", generated(), MAIN);
    let Some(run) = judged(
        "swift-of",
        RUNTIME_VAR,
        "swift",
        &["-warnings-as-errors"],
        "main.swift",
        &source,
    ) else {
        return;
    };
    assert!(run.succeeded, "{}\n{source}", run.stderr);
    let refused = DockTimeDraft {
        hour: 24,
        minute: 0,
    }
    .validate()
    .unwrap_err();
    assert_eq!(
        run.stdout.lines().collect::<Vec<_>>(),
        [
            serde_json::to_string(&DockTimeDraft {
                hour: 12,
                minute: 0
            })
            .unwrap()
            .as_str(),
            serde_json::to_string(&DockFlagDraft {
                advanced: false,
                is_calculated: true,
                kind: "dock".to_owned(),
                version: 3
            })
            .unwrap()
            .as_str(),
            serde_json::to_string(&DockOfDraft { of: "x".to_owned() })
                .unwrap()
                .as_str(),
            serde_json::to_string(&DockStampDraft).unwrap().as_str(),
            serde_json::to_string(&DockWrappedDraft("w".to_owned()))
                .unwrap()
                .as_str(),
            refused.join("; ").as_str(),
        ]
    );
}
