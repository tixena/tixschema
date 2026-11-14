//! The resource reads and completions whose answer is a fault, with the kind, field, listed
//! problems and operation every `mcp` server answers for them: the Rust dispatcher and the emitted
//! TypeScript and Dart servers all run this one table.

use serde_json::Value;

pub const RESOURCE_FAULT_CASES: [ResourceFaultCase; 5] = [
    ResourceFaultCase {
        field: None,
        issues: &[],
        kind: "failed-validation",
        method: "resources/read",
        operation: "get-count",
        params: r#"{"uri":"shelf://counts/abc"}"#,
    },
    ResourceFaultCase {
        field: Some("aisle"),
        issues: &[],
        kind: "failed-validation",
        method: "resources/read",
        operation: "get-slot",
        params: r#"{"uri":"shelf://aisles/too-long/slots/1"}"#,
    },
    ResourceFaultCase {
        field: Some("chosen"),
        issues: &[],
        kind: "failed-validation",
        method: "completion/complete",
        operation: "suggest-slots",
        params: r#"{"ref":{"type":"ref/resource","uri":"shelf://aisles/{aisle}/slots/{slot}"},"argument":{"name":"slot","value":"1"},"context":{"arguments":5}}"#,
    },
    ResourceFaultCase {
        field: Some("typed"),
        issues: &[],
        kind: "failed-validation",
        method: "completion/complete",
        operation: "suggest-slots",
        params: r#"{"ref":{"type":"ref/resource","uri":"shelf://aisles/{aisle}/slots/{slot}"},"argument":{"name":"slot","value":"too-long"},"context":{"arguments":{}}}"#,
    },
    ResourceFaultCase {
        field: Some("bay"),
        issues: &["bay", "level"],
        kind: "failed-validation",
        method: "resources/read",
        operation: "get-bay",
        params: r#"{"uri":"shelf://bays/east/levels/top"}"#,
    },
];

/// One `resources/read` or `completion/complete` and the fault it must answer. `field` is the
/// field of the first problem a failed validation lists, and `issues` the field of each problem
/// it lists, in order, empty where it lists that one problem alone.
pub struct ResourceFaultCase {
    pub field: Option<&'static str>,
    pub issues: &'static [&'static str],
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
        assert!(fault.get("field").is_none(), "{reply:#}");
        let Some(issues) = fault.get("issues").and_then(Value::as_array) else {
            assert_ne!(self.kind, "failed-validation", "{reply:#}");
            assert_eq!(self.field, None, "{reply:#}");
            assert_eq!(self.issues, &[] as &[&str], "{reply:#}");
            return;
        };
        assert!(fault.get("detail").is_none(), "{reply:#}");
        assert_eq!(
            issues.first().and_then(|issue| issue["field"].as_str()),
            self.field,
            "{reply:#}"
        );
        let fields: Vec<&str> = issues
            .iter()
            .filter_map(|issue| issue["field"].as_str())
            .collect();
        if self.issues.is_empty() {
            assert_eq!(issues.len(), 1, "{reply:#}");
        } else {
            assert_eq!(fields, self.issues, "{reply:#}");
        }
    }

    /// The params of the request, as a JSON value.
    pub fn params(&self) -> Value {
        serde_json::from_str(self.params).unwrap()
    }
}
