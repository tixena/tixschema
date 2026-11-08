//! The resource reads and completions whose answer is a fault, with the kind, field and operation
//! every `mcp` server answers for them: the Rust dispatcher and the emitted TypeScript and Dart
//! servers all run this one table.

use serde_json::Value;

pub const RESOURCE_FAULT_CASES: [ResourceFaultCase; 4] = [
    ResourceFaultCase {
        field: None,
        kind: "undeserializable-payload",
        method: "resources/read",
        operation: "get-count",
        params: r#"{"uri":"shelf://counts/abc"}"#,
    },
    ResourceFaultCase {
        field: Some("aisle"),
        kind: "failed-validation",
        method: "resources/read",
        operation: "get-slot",
        params: r#"{"uri":"shelf://aisles/too-long/slots/1"}"#,
    },
    ResourceFaultCase {
        field: None,
        kind: "undeserializable-payload",
        method: "completion/complete",
        operation: "suggest-slots",
        params: r#"{"ref":{"type":"ref/resource","uri":"shelf://aisles/{aisle}/slots/{slot}"},"argument":{"name":"slot","value":"1"},"context":{"arguments":5}}"#,
    },
    ResourceFaultCase {
        field: Some("typed"),
        kind: "failed-validation",
        method: "completion/complete",
        operation: "suggest-slots",
        params: r#"{"ref":{"type":"ref/resource","uri":"shelf://aisles/{aisle}/slots/{slot}"},"argument":{"name":"slot","value":"too-long"},"context":{"arguments":{}}}"#,
    },
];

/// One `resources/read` or `completion/complete` and the fault it must answer.
pub struct ResourceFaultCase {
    pub field: Option<&'static str>,
    pub kind: &'static str,
    pub method: &'static str,
    pub operation: &'static str,
    pub params: &'static str,
}

impl ResourceFaultCase {
    /// Holds the fault in `reply`, a `-32602` error, to this case's kind, field and operation.
    pub fn holds(&self, reply: &Value) {
        assert_eq!(reply["error"]["code"], -32_602_i32, "{reply:#}");
        let carried = &reply["error"]["data"];
        assert_eq!(carried["isServiceFault"], true, "{reply:#}");
        let fault = &carried["fault"];
        assert_eq!(fault["kind"], self.kind, "{}: {reply:#}", self.params);
        assert_eq!(fault["operation"], self.operation, "{reply:#}");
        assert_eq!(
            fault.get("field").and_then(Value::as_str),
            self.field,
            "{reply:#}"
        );
    }

    /// The params of the request, as a JSON value.
    pub fn params(&self) -> Value {
        serde_json::from_str(self.params).unwrap()
    }
}
