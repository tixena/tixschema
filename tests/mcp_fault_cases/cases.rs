//! The requests whose answer is a fault, with the kind, field and operation every `mcp` server
//! answers for them: the Rust dispatcher test and the emitted TypeScript and Dart server tests all
//! run this one table.

use serde_json::Value;

pub const FAULT_CASES: [FaultCase; 5] = [
    FaultCase {
        arguments: r#"{"document_id":5}"#,
        field: None,
        kind: "undeserializable-payload",
        operation: "get-version",
        tool: "get-version",
    },
    FaultCase {
        arguments: r#"{"document_id":""}"#,
        field: Some("document_id"),
        kind: "failed-validation",
        operation: "get-version",
        tool: "get-version",
    },
    FaultCase {
        arguments: r#"{"value":5}"#,
        field: None,
        kind: "undeserializable-payload",
        operation: "lookup-document",
        tool: "lookup-document",
    },
    FaultCase {
        arguments: r#"{"value":"doc-1","if-none-match":5}"#,
        field: Some("if-none-match"),
        kind: "failed-validation",
        operation: "get-tagged",
        tool: "get-tagged",
    },
    FaultCase {
        arguments: r#"{"value":"panic"}"#,
        field: None,
        kind: "handler-panic",
        operation: "lookup-document",
        tool: "lookup-document",
    },
];

/// One `tools/call` and the fault it must answer.
pub struct FaultCase {
    pub arguments: &'static str,
    pub field: Option<&'static str>,
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
