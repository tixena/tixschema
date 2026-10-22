//! An `i64` through the emitted TypeScript service, run by node with the real `zod`: the client of
//! each transport hands a `bigint` as the string of its digits, the dispatcher and the HTTP
//! dispatcher answer the `bigint` an implementation returns as its digits, and each client reads
//! that answer back into a `bigint`, exactly, beyond what a JavaScript `number` holds. A member of
//! a message the service declares is read from the text of its path and of its query, and a header
//! of a brand over an `i64` from the digits each client writes in it.

use core::future::ready;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::{model_schema, service_schema};

use super::runtime::{node_modules, ran_with_modules, stand_down_modules};
use super::tests::poll_once;

const RUNTIME_VAR: &str = "TIXSCHEMA_NODE";

const REQUIRED_PACKAGES: &[&str] = &["zod"];

/// Sends a tally through each client to the dispatcher of its transport, and prints what each
/// transport carried each way and what each client answered, a `bigint` written with an `n`.
const DRIVER: &str = r#"
const shown = (value) => JSON.parse(JSON.stringify(value, (_, held) => (typeof held === "bigint" ? `${held}n` : held)));

const impl = {
  async count(_ctx, req) {
    return { ok: true, value: { tally: req.tally + 1n } };
  },
  async fetch(_ctx, req, ledger) {
    return { ok: true, value: { tally: req.tally + (req.limit ?? 0n) + (ledger - 9007199254740993n) } };
  },
};

const dispatch = createTallyingClientServiceDispatcher(impl);
const handle = createTallyingClientServiceHttpDispatcher(impl);

const overBus = async (call) => {
  const carried = [];
  const client = createTallyingClientServiceClient({
    async notify() { throw new Error("no one-way operation is declared"); },
    async request(operation, payload, headers) {
      const sent = JSON.parse(JSON.stringify(payload));
      const dispatched = await dispatch(undefined, operation, sent, headers);
      const answered = JSON.parse(JSON.stringify(dispatched.answered));
      carried.push(sent, headers, answered);
      return { answered, headers: dispatched.headers };
    },
  });
  return { carried, answered: shown(await call(client)) };
};

const overHttp = async (call) => {
  const carried = [];
  const client = createTallyingClientServiceHttpClient({
    async send(request) {
      const response = await handle(undefined, {
        body: request.body,
        headers: request.headers,
        method: request.method,
        path: request.path,
        query: request.query,
      });
      const sent = new TextDecoder().decode(request.body);
      carried.push(`${request.method} ${request.path}?${request.query} ${JSON.stringify(request.headers)} ${sent}`, new TextDecoder().decode(response.body));
      return { status: response.status, headers: response.headers, body: response.body };
    },
  });
  return { carried, answered: shown(await call(client)) };
};

const calls = [
  (client) => client.count({ tally: 9223372036854775806n }),
  (client) => client.fetch({ tally: -9223372036854775807n, limit: -1n }, 9007199254740993n),
  (client) => client.count({ tally: 9223372036854775807n }),
];
const ran = [];
for (const call of calls) {
  ran.push({ bus: await overBus(call), http: await overHttp(call) });
}
console.log(JSON.stringify(ran));
"#;

/// A tally, whose `i64` travels as the string of its digits.
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TallyCount {
    pub tally: i64,
}

/// A brand over an `i64`, handed in a header.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct TallyLedger(pub i64);

/// The one error each operation declares.
#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "errorCode")]
pub enum TallyError {
    Refused,
}

/// An operation over a message of the caller's, and one over a message the service declares from
/// a path and a query, beside a header.
#[service_schema(transports = [])]
pub trait TallyingClientService<Ctx> {
    #[service_schema_op(http(method = "POST", path = "/tallies"))]
    async fn count(&self, ctx: &Ctx, req: TallyCount) -> Result<TallyCount, TallyError>;

    #[service_schema_op(http(
        method = "GET",
        path = "/tallies/{tally}",
        header_in("x-ledger" = ledger),
    ))]
    async fn fetch(
        &self,
        ctx: &Ctx,
        tally: i64,
        limit: Option<i64>,
        ledger: TallyLedger,
    ) -> Result<TallyCount, TallyError>;
}

/// A backend answering the contract, so the trait is implementable rather than merely declared.
pub struct TallyingBackEnd;

impl TallyingClientService<()> for TallyingBackEnd {
    async fn count(&self, _ctx: &(), req: TallyCount) -> Result<TallyCount, TallyError> {
        ready(()).await;
        req.tally
            .checked_add(1)
            .map(|sum| TallyCount { tally: sum })
            .ok_or(TallyError::Refused)
    }

    async fn fetch(
        &self,
        _ctx: &(),
        tally: i64,
        limit: Option<i64>,
        ledger: TallyLedger,
    ) -> Result<TallyCount, TallyError> {
        ready(()).await;
        tally
            .checked_add(limit.unwrap_or(0))
            .and_then(|sum| sum.checked_add(ledger.0))
            .map(|sum| TallyCount { tally: sum })
            .ok_or(TallyError::Refused)
    }
}

/// The messages, the service, its dispatchers and its clients, beside the real `zod`.
fn emitted() -> String {
    [
        "import { z, type ZodType } from \"zod\";".to_owned(),
        TallyCount::ts_definition(),
        TallyCount::zod_schema(),
        TallyLedger::ts_definition(),
        TallyLedger::zod_schema(),
        TallyError::ts_definition(),
        TallyError::zod_schema(),
        TallyingClientServiceSchema::ts_definition(),
        TallyingClientServiceSchema::ts_service(),
        TallyingClientServiceSchema::ts_http_service(),
        TallyingClientServiceSchema::ts_client(),
        TallyingClientServiceSchema::ts_http_client(),
    ]
    .join("\n\n")
}

/// What each transport carries each way, the headers of a request beside its message, and what
/// each client answers, of each call of the driver: the third answered beyond an `i64`, which each
/// client refuses.
fn carried() -> Value {
    json!([
        {
            "bus": {
                "carried": [
                    { "tally": "9223372036854775806" },
                    [],
                    { "ok": true, "value": { "tally": "9223372036854775807" } },
                ],
                "answered": { "ok": true, "value": { "tally": "9223372036854775807n" } },
            },
            "http": {
                "carried": [
                    r#"POST /tallies? [] {"tally":"9223372036854775806"}"#,
                    r#"{"tally":"9223372036854775807"}"#,
                ],
                "answered": { "ok": true, "value": { "tally": "9223372036854775807n" } },
            },
        },
        {
            "bus": {
                "carried": [
                    { "limit": "-1", "tally": "-9223372036854775807" },
                    [["x-ledger", r#""9007199254740993""#]],
                    { "ok": true, "value": { "tally": "-9223372036854775808" } },
                ],
                "answered": { "ok": true, "value": { "tally": "-9223372036854775808n" } },
            },
            "http": {
                "carried": [
                    r#"GET /tallies/-9223372036854775807?limit=-1 [["x-ledger","9007199254740993"]] "#,
                    r#"{"tally":"-9223372036854775808"}"#,
                ],
                "answered": { "ok": true, "value": { "tally": "-9223372036854775808n" } },
            },
        },
        {
            "bus": {
                "carried": [
                    { "tally": "9223372036854775807" },
                    [],
                    { "ok": true, "value": { "tally": "9223372036854775808" } },
                ],
                "answered": {
                    "ok": false,
                    "error": {
                        "isServiceFault": true,
                        "fault": {
                            "detail": "'tally': Too big: expected bigint to be \
                                       <=9223372036854775807",
                            "field": "tally",
                            "kind": "failed-validation",
                            "operation": "count",
                        },
                    },
                },
            },
            "http": {
                "carried": [
                    r#"POST /tallies? [] {"tally":"9223372036854775807"}"#,
                    r#"{"tally":"9223372036854775808"}"#,
                ],
                "answered": {
                    "ok": false,
                    "error": {
                        "isServiceFault": true,
                        "fault": {
                            "detail": "'tally': Too big: expected bigint to be \
                                       <=9223372036854775807",
                            "kind": "undeserializable-payload",
                            "operation": "count",
                        },
                    },
                },
            },
        },
    ])
}

/// Each client sends a `bigint` as its digits, each dispatcher answers one as its digits, and each
/// client reads the answer back into the `bigint` it was, over both transports.
#[test]
fn an_i64_travels_as_its_digits_through_each_typescript_client_and_dispatcher() {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(REQUIRED_PACKAGES, "the emitted service's 64-bit integers");
        return;
    };
    let module = format!("{}\n\n{DRIVER}", emitted());
    let Some(wrote) = ran_with_modules(
        "digits",
        RUNTIME_VAR,
        "node",
        "digits.mts",
        &module,
        &modules,
    ) else {
        return;
    };
    let ran: Value = serde_json::from_str(wrote.trim()).unwrap();
    assert_eq!(
        ran,
        carried(),
        "a 64-bit integer does not travel as its digits"
    );
    let read: TallyCount = serde_json::from_value(ran[0]["bus"]["carried"][0].clone()).unwrap();
    assert_eq!(
        poll_once(TallyingBackEnd.count(&(), read))
            .map(|answered| answered.map(|count| count.tally).ok()),
        Some(Some(i64::MAX)),
        "the backend counts what serde read of what the client sends"
    );
    assert_eq!(
        poll_once(TallyingBackEnd.fetch(&(), i64::MIN + 1, Some(-1), TallyLedger(0)))
            .map(|answered| answered.map(|count| count.tally).ok()),
        Some(Some(i64::MIN)),
        "the backend answers the tally the dispatcher answered"
    );
}
