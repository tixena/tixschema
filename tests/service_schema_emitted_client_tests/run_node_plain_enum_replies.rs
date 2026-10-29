//! A reply typed with a plain enum that names a check, handed to the emitted TypeScript clients of
//! each transport by a canned transport, run by node with the real `zod`: a reply the check
//! refuses is the fault each client answers for a reply it cannot read, with the check's
//! sentence, and one it keeps is the value. Each payload is held beside what the serde read says
//! of it.

use serde_json::{Value, json};

use super::plain_enum_check_rows::{AppraisalClientServiceSchema, AppraisalError, Verdict};
use super::runtime::{node_modules, ran_with_modules, stand_down_modules};

const RUNTIME_VAR: &str = "TIXSCHEMA_NODE";

const REQUIRED_PACKAGES: &[&str] = &["zod"];

const DRIVER: &str = r#"
function affirmed(value, ctx) {
  if (value === "Overturned") ctx.issues.push({ code: "custom", message: "a verdict is upheld", input: value });
}

const asked = async (client) => client.appraise("c1");

const overBus = async (reply) =>
  asked(createAppraisalClientServiceClient({
    async notify() { throw new Error("no one-way operation is declared"); },
    async request() { return { answered: { ok: true, value: reply }, headers: [] }; },
  }));

const overHttp = async (reply) =>
  asked(createAppraisalClientServiceHttpClient({
    async send() {
      return { status: 200, headers: [["content-type", "application/json"]], body: new TextEncoder().encode(JSON.stringify(reply)) };
    },
  }));

console.log(JSON.stringify({
  busKept: await overBus("Upheld"),
  busBroken: await overBus("Overturned"),
  httpKept: await overHttp("Upheld"),
  httpBroken: await overHttp("Overturned"),
}));
"#;

fn emitted() -> String {
    [
        "import { z, type ZodType } from \"zod\";".to_owned(),
        Verdict::ts_definition(),
        Verdict::zod_schema(),
        AppraisalError::ts_definition(),
        AppraisalError::zod_schema(),
        AppraisalClientServiceSchema::ts_definition(),
        AppraisalClientServiceSchema::ts_service(),
        AppraisalClientServiceSchema::ts_client(),
        AppraisalClientServiceSchema::ts_http_client(),
    ]
    .join("\n\n")
}

#[test]
fn each_typescript_client_refuses_a_reply_typed_with_a_plain_enum_that_names_a_check() {
    assert_eq!(
        serde_json::from_str::<Verdict>(r#""Overturned""#)
            .unwrap_err()
            .to_string(),
        "a verdict is upheld"
    );
    serde_json::from_str::<Verdict>(r#""Upheld""#).unwrap();
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(REQUIRED_PACKAGES, "the emitted clients' reply checks");
        return;
    };
    let module = format!("{}\n\n{DRIVER}", emitted());
    let Some(wrote) = ran_with_modules(
        "plain-enum-replies",
        RUNTIME_VAR,
        "node",
        "replies.mts",
        &module,
        &modules,
    ) else {
        return;
    };
    let ran: Value = serde_json::from_str(wrote.trim()).unwrap();
    let refused = |kind: &str| {
        json!({
            "ok": false,
            "error": {
                "isServiceFault": true,
                "fault": {
                    "detail": "a verdict is upheld",
                    "kind": kind,
                    "operation": "appraise"
                }
            }
        })
    };
    assert_eq!(
        ran,
        json!({
            "busKept": { "ok": true, "value": "Upheld" },
            "busBroken": refused("failed-validation"),
            "httpKept": { "ok": true, "value": "Upheld" },
            "httpBroken": refused("undeserializable-payload"),
        })
    );
}
