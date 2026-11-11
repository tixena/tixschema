//! The requests whose answer is a fault, with the kind, field, listed problems and operation every
//! `mcp` server answers for them: the Rust dispatcher test and the emitted TypeScript and Dart
//! server tests all run this one table.

use serde_json::Value;

pub const FAULT_CASES: [FaultCase; 20] = [
    FaultCase {
        arguments: r#"{"document_id":5}"#,
        field: Some("document_id"),
        issues: &[],
        kind: "failed-validation",
        operation: "get-version",
        tool: "get-version",
    },
    FaultCase {
        arguments: r#"{"document_id":""}"#,
        field: Some("document_id"),
        issues: &[],
        kind: "failed-validation",
        operation: "get-version",
        tool: "get-version",
    },
    FaultCase {
        arguments: r#"{"value":5}"#,
        field: None,
        issues: &[],
        kind: "failed-validation",
        operation: "lookup-document",
        tool: "lookup-document",
    },
    FaultCase {
        arguments: r#"{"value":"doc-1","if-none-match":5}"#,
        field: Some("if-none-match"),
        issues: &[],
        kind: "failed-validation",
        operation: "get-tagged",
        tool: "get-tagged",
    },
    FaultCase {
        arguments: r#"{"drafts":[{"references":[],"title":"Plan"},{"references":[]}]}"#,
        field: Some("drafts.1.title"),
        issues: &[],
        kind: "failed-validation",
        operation: "check-drafts",
        tool: "check-drafts",
    },
    FaultCase {
        arguments: r#"{"drafts":[{"references":[{"name":5}],"title":"Plan"}]}"#,
        field: Some("drafts.0.references.0.name"),
        issues: &[],
        kind: "failed-validation",
        operation: "check-drafts",
        tool: "check-drafts",
    },
    FaultCase {
        arguments: r#"{"drafts":[{"references":[{"name":"a"}],"title":""}]}"#,
        field: Some("drafts.0.title"),
        issues: &[],
        kind: "failed-validation",
        operation: "check-drafts",
        tool: "check-drafts",
    },
    FaultCase {
        arguments: r#"{"value":"panic"}"#,
        field: None,
        issues: &[],
        kind: "handler-panic",
        operation: "lookup-document",
        tool: "lookup-document",
    },
    FaultCase {
        arguments: r#"{"value":[{"document_id":7}]}"#,
        field: Some("0.document_id"),
        issues: &[],
        kind: "failed-validation",
        operation: "check-many",
        tool: "check-many",
    },
    FaultCase {
        arguments: r#"{"value":[{"document_id":""}]}"#,
        field: Some("document_id"),
        issues: &[],
        kind: "failed-validation",
        operation: "check-many",
        tool: "check-many",
    },
    FaultCase {
        arguments: r#"{"first":{"document_id":7}}"#,
        field: Some("first.document_id"),
        issues: &[],
        kind: "failed-validation",
        operation: "check-named",
        tool: "check-named",
    },
    FaultCase {
        arguments: r#"{"first":{"document_id":""}}"#,
        field: Some("document_id"),
        issues: &[],
        kind: "failed-validation",
        operation: "check-named",
        tool: "check-named",
    },
    FaultCase {
        arguments: r#"{"value":{"document_id":7}}"#,
        field: Some("document_id"),
        issues: &[],
        kind: "failed-validation",
        operation: "check-maybe",
        tool: "check-maybe",
    },
    FaultCase {
        arguments: r#"{"value":{"document_id":""}}"#,
        field: Some("document_id"),
        issues: &[],
        kind: "failed-validation",
        operation: "check-maybe",
        tool: "check-maybe",
    },
    // The message holds its request type twice, so the tool's input defines it once under
    // `$defs` and the path is named through a `$ref`.
    FaultCase {
        arguments: r#"{"first":{"document_id":"doc-1"},"second":{"document_id":7}}"#,
        field: Some("second.document_id"),
        issues: &[],
        kind: "failed-validation",
        operation: "check-pair",
        tool: "check-pair",
    },
    FaultCase {
        arguments: r#"{"drafts":[{"references":[]},{"references":[]}]}"#,
        field: Some("drafts.0.title"),
        issues: &["drafts.0.title", "drafts.1.title"],
        kind: "failed-validation",
        operation: "check-drafts",
        tool: "check-drafts",
    },
    // A draft reads no key it does not declare, so `subtitle` is refused beside the two others.
    FaultCase {
        arguments: r#"{"drafts":[{"references":[]},{"references":[],"subtitle":"Draft","title":"Plan"},{"references":[],"title":5}]}"#,
        field: Some("drafts.0.title"),
        issues: &["drafts.0.title", "drafts.1.subtitle", "drafts.2.title"],
        kind: "failed-validation",
        operation: "check-drafts",
        tool: "check-drafts",
    },
    // A key no draft declares is named at its own path, in the detail as in the field.
    FaultCase {
        arguments: r#"{"drafts":[{"references":[],"title":"Plan"},{"references":[],"subtitle":"Draft","title":"Plan"}]}"#,
        field: Some("drafts.1.subtitle"),
        issues: &[],
        kind: "failed-validation",
        operation: "check-drafts",
        tool: "check-drafts",
    },
    // A bound is the validator's, judged once the arguments read: an empty title beside values
    // that do not read is not listed.
    FaultCase {
        arguments: r#"{"drafts":[{"references":[]},{"references":[],"title":""},{"references":[]}]}"#,
        field: Some("drafts.0.title"),
        issues: &["drafts.0.title", "drafts.2.title"],
        kind: "failed-validation",
        operation: "check-drafts",
        tool: "check-drafts",
    },
    FaultCase {
        arguments: r#"{"drafts":[{"references":[],"title":""},{"references":[]}]}"#,
        field: Some("drafts.1.title"),
        issues: &[],
        kind: "failed-validation",
        operation: "check-drafts",
        tool: "check-drafts",
    },
];

/// One `tools/call` and the fault it must answer. `issues` is the field of each problem the fault
/// lists, in order, and empty where it names one problem and lists none.
pub struct FaultCase {
    pub arguments: &'static str,
    pub field: Option<&'static str>,
    pub issues: &'static [&'static str],
    pub kind: &'static str,
    pub operation: &'static str,
    pub tool: &'static str,
}

impl FaultCase {
    /// The arguments as a JSON value.
    pub fn arguments(&self) -> Value {
        serde_json::from_str(self.arguments).unwrap()
    }

    /// Holds the fault in `reply` to this case's kind, field and operation.
    pub fn holds(&self, reply: &Value) {
        let fault = fault_in(reply);
        assert_eq!(fault["kind"], self.kind, "{}: {reply:#}", self.arguments);
        assert_eq!(fault["operation"], self.operation, "{reply:#}");
        assert_eq!(
            fault.get("field").and_then(Value::as_str),
            self.field,
            "{reply:#}"
        );
        if let (Some(field), "check-drafts") = (self.field, self.tool) {
            let detail = fault["detail"].as_str().unwrap();
            assert!(detail.starts_with(&format!("'{field}': ")), "{reply:#}");
        }
        let Some(issues) = fault.get("issues").and_then(Value::as_array) else {
            assert!(fault.get("issues").is_none(), "{reply:#}");
            assert_eq!(self.issues, &[] as &[&str], "{reply:#}");
            return;
        };
        let fields: Vec<&str> = issues
            .iter()
            .map(|issue| issue["field"].as_str().unwrap())
            .collect();
        assert_eq!(fields, self.issues, "{reply:#}");
        let detail = fault["detail"].as_str().unwrap();
        for issue in issues {
            let said = issue["detail"].as_str().unwrap();
            let at = issue["field"].as_str().unwrap();
            assert!(
                !said.is_empty() && detail.contains(&format!("'{at}': {said}")),
                "{reply:#}"
            );
        }
    }
}

/// The fault an answer carries, whether it is a tool error or a protocol error.
fn fault_in(reply: &Value) -> Value {
    let carried = if reply["error"]["data"].is_object() {
        reply["error"]["data"].clone()
    } else {
        serde_json::from_str(reply["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
    };
    assert_eq!(carried["isServiceFault"], true, "{reply:#}");
    carried["fault"].clone()
}
