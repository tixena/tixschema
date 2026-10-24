//! An `i64` through the emitted TypeScript service, run by node with the real `zod`: the client of
//! each transport hands a `bigint` as the JSON number of its digits, the dispatcher and the HTTP
//! dispatcher answer the `bigint` an implementation returns as the JSON number, and each client
//! reads that answer back into a `bigint`, exactly, beyond what a JavaScript `number` holds,
//! through the reader each emits. A reply read by a plain `JSON.parse`, within what a `number`
//! holds, is the `bigint` too. A member of a message the service declares is read from the text of
//! its path and of its query, and a header of an `i64` from the digits each client writes in it.

use core::future::ready;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::{model_schema, service_schema};

use super::runtime::{node_modules, ran_with_modules, stand_down_modules};
use super::tests::poll_once;

const RUNTIME_VAR: &str = "TIXSCHEMA_NODE";

const REQUIRED_PACKAGES: &[&str] = &["zod"];

/// The transport's own reader: `JSON.parse` with the source-text reviver the emitted readers use.
const BUS_READ: &str = r#"
const busRead = (text) =>
  JSON.parse(text, (_key, value, context) =>
    typeof value === "number" && !Number.isSafeInteger(value) && /^-?(0|[1-9][0-9]*)$/.test(context?.source ?? "")
      ? BigInt(context.source)
      : value,
  );
"#;

/// Sends a tally through each client to the dispatcher of its transport, and prints the text each
/// transport carried each way and what each client answered, a `bigint` written with an `n`. The
/// bus is joined as an adapter joins it, each text read by the harness's own source-text reader;
/// the last call's reply is read by a plain `JSON.parse`.
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

const overBus = (readReply) => async (call) => {
  const carried = [];
  const client = createTallyingClientServiceClient({
    async notify() { throw new Error("no one-way operation is declared"); },
    async request(operation, payload, headers) {
      const sent = JSON.stringify(payload);
      const dispatched = await dispatch(undefined, operation, tallyingClientServiceRequestRead(sent), headers);
      const answered = JSON.stringify(dispatched.answered);
      carried.push(sent, headers, answered);
      return { answered: readReply(answered), headers: dispatched.headers };
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
  [(client) => client.count({ tally: 9223372036854775806n }), busRead],
  [(client) => client.fetch({ tally: -9223372036854775807n, limit: -1n }, 9007199254740993n), busRead],
  [(client) => client.count({ tally: 9223372036854775807n }), busRead],
  [(client) => client.count({ tally: 1719623999999n }), JSON.parse],
];
const ran = [];
for (const [call, readReply] of calls) {
  ran.push({ bus: await overBus(readReply)(call), http: await overHttp(call) });
}
console.log(JSON.stringify(ran));
"#;

/// Sends each bare `i64` through each client to the dispatcher of its transport, and prints the
/// text each transport carried each way and what each client answered, a `bigint` written with an
/// `n`. The bus is joined as an adapter joins it, each text read by the reader the side that
/// receives it emits.
const BARE_DRIVER: &str = r#"
const shown = (value) => value === undefined ? null : JSON.parse(JSON.stringify(value, (_, held) => (typeof held === "bigint" ? `${held}n` : held)));

const recorded = [];
const impl = {
  async shift(_ctx, req, offset) {
    const shifted = req + (offset ?? 0n);
    return { ok: true, value: [shifted, shifted * 2n <= 9223372036854775807n ? shifted * 2n : null] };
  },
  async total(_ctx, req) {
    const sum = req.reduce((held, entry) => held + entry, 0n);
    return sum > 9223372036854775807n ? { ok: false, error: -9223372036854775808n } : { ok: true, value: sum };
  },
  async record(_ctx, req) {
    recorded.push(req);
  },
};

const dispatch = createTallyingBareServiceDispatcher(impl);
const handle = createTallyingBareServiceHttpDispatcher(impl);

const overBus = async (call) => {
  const carried = [];
  const client = createTallyingBareServiceClient({
    async notify(operation, payload, headers) {
      const sent = JSON.stringify(payload);
      await dispatch(undefined, operation, tallyingBareServiceRequestRead(sent), headers);
      carried.push(sent, headers);
    },
    async request(operation, payload, headers) {
      const sent = JSON.stringify(payload);
      const dispatched = await dispatch(undefined, operation, tallyingBareServiceRequestRead(sent), headers);
      const answered = JSON.stringify(dispatched.answered);
      carried.push(sent, headers, answered, dispatched.headers);
      return { answered: busRead(answered), headers: dispatched.headers };
    },
  });
  return { carried, answered: shown(await call(client)) };
};

const overHttp = async (call) => {
  const carried = [];
  const client = createTallyingBareServiceHttpClient({
    async send(request) {
      const response = await handle(undefined, {
        body: request.body,
        headers: request.headers,
        method: request.method,
        path: request.path,
        query: request.query,
      });
      const sent = new TextDecoder().decode(request.body);
      carried.push(`${request.method} ${request.path} ${JSON.stringify(request.headers)} ${sent}`, `${response.status} ${JSON.stringify(response.headers)} ${new TextDecoder().decode(response.body)}`);
      return { status: response.status, headers: response.headers, body: response.body };
    },
  });
  return { carried, answered: shown(await call(client)) };
};

const calls = [
  (client) => client.shift(9223372036854775806n, 1n),
  (client) => client.shift(4611686018427387903n, undefined),
  (client) => client.total([9223372036854775807n, 1n]),
  (client) => client.record(-9223372036854775808n),
];
const ran = [];
for (const call of calls) {
  ran.push({ bus: await overBus(call), http: await overHttp(call) });
}
console.log(JSON.stringify({ ran, recorded: shown(recorded) }));
"#;

/// Sends a tally through the bus client to its dispatcher in a runtime without `JSON.rawJSON`, and
/// prints the text the transport carried each way and what the client answered: a `bigint` a
/// `number` holds exactly is written as that number, and any other is refused. Then hands each
/// dispatcher a tally whose answer no `number` holds exactly.
const FALLBACK_DRIVER: &str = r#"
const shown = (value) => JSON.parse(JSON.stringify(value, (_, held) => (typeof held === "bigint" ? `${held}n` : held)));
Reflect.deleteProperty(JSON, "rawJSON");

const impl = {
  async count(_ctx, req) {
    return { ok: true, value: { tally: req.tally + 1n } };
  },
  async fetch() {
    throw new Error("not called");
  },
};
const dispatch = createTallyingClientServiceDispatcher(impl);
const handle = createTallyingClientServiceHttpDispatcher(impl);

const run = async (tally) => {
  const carried = [];
  const client = createTallyingClientServiceClient({
    async notify() { throw new Error("no one-way operation is declared"); },
    async request(operation, payload, headers) {
      const sent = JSON.stringify(payload);
      const dispatched = await dispatch(undefined, operation, tallyingClientServiceRequestRead(sent), headers);
      const answered = JSON.stringify(dispatched.answered);
      carried.push(sent, answered);
      return { answered: busRead(answered), headers: dispatched.headers };
    },
  });
  return { carried, answered: shown(await client.count({ tally })) };
};
const overBus = await dispatch(undefined, "count", { tally: 9007199254740991n }).then(
  () => "answered",
  (refused) => `refused: ${refused.message}`,
);
const overHttp = await handle(undefined, {
  body: new TextEncoder().encode('{"tally":9007199254740991}'),
  headers: [],
  method: "POST",
  path: "/tallies",
  query: "",
});
console.log(JSON.stringify({
  sent: [await run(5n), await run(9007199254740993n)],
  overBus,
  overHttp: `${overHttp.status} ${new TextDecoder().decode(overHttp.body)}`,
}));
"#;

/// A tally, whose `i64` travels as a JSON number.
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TallyCount {
    pub tally: i64,
}

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
        ledger: i64,
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
        ledger: i64,
    ) -> Result<TallyCount, TallyError> {
        ready(()).await;
        tally
            .checked_add(limit.unwrap_or(0))
            .and_then(|sum| sum.checked_add(ledger))
            .map(|sum| TallyCount { tally: sum })
            .ok_or(TallyError::Refused)
    }
}

/// A bare `i64` as the message, a header beside it, the reply and a header beside the reply; a
/// bare list of them as the message beside an `i64` as the declared error; and a one-way message
/// of one.
#[service_schema(transports = [])]
pub trait TallyingBareService<Ctx> {
    #[service_schema_op(one_way, http(method = "PUT", path = "/bare"))]
    async fn record(&self, ctx: &Ctx, entry: i64);

    #[service_schema_op(http(
        method = "GET",
        path = "/bare/{entry}",
        header_in("x-offset" = offset),
        header_out("x-doubled"),
    ))]
    async fn shift(
        &self,
        ctx: &Ctx,
        entry: i64,
        offset: Option<i64>,
    ) -> Result<(i64, Option<i64>), TallyError>;

    #[service_schema_op(http(method = "POST", path = "/bare"))]
    async fn total(&self, ctx: &Ctx, entries: Vec<i64>) -> Result<i64, i64>;
}

/// A backend answering the bare contract, so the trait is implementable rather than merely
/// declared.
pub struct TallyingBareBackEnd;

impl TallyingBareService<()> for TallyingBareBackEnd {
    async fn record(&self, _ctx: &(), _entry: i64) {
        ready(()).await;
    }

    async fn shift(
        &self,
        _ctx: &(),
        entry: i64,
        offset: Option<i64>,
    ) -> Result<(i64, Option<i64>), TallyError> {
        ready(()).await;
        let shifted = entry
            .checked_add(offset.unwrap_or(0))
            .ok_or(TallyError::Refused)?;
        Ok((shifted, shifted.checked_mul(2)))
    }

    async fn total(&self, _ctx: &(), entries: Vec<i64>) -> Result<i64, i64> {
        ready(()).await;
        entries
            .iter()
            .copied()
            .try_fold(0_i64, i64::checked_add)
            .ok_or(i64::MIN)
    }
}

/// The messages, the service, its dispatchers and its clients, beside the real `zod`.
fn emitted() -> String {
    [
        "import { z, type ZodType } from \"zod\";".to_owned(),
        TallyCount::ts_definition(),
        TallyCount::zod_schema(),
        TallyError::ts_definition(),
        TallyError::zod_schema(),
        TallyingClientServiceSchema::ts_definition(),
        TallyingClientServiceSchema::ts_service(),
        TallyingClientServiceSchema::ts_http_service(),
        TallyingClientServiceSchema::ts_client(),
        TallyingClientServiceSchema::ts_http_client(),
        TallyingBareServiceSchema::ts_definition(),
        TallyingBareServiceSchema::ts_service(),
        TallyingBareServiceSchema::ts_http_service(),
        TallyingBareServiceSchema::ts_client(),
        TallyingBareServiceSchema::ts_http_client(),
    ]
    .join("\n\n")
}

/// What each transport carries each way of each call of [`BARE_DRIVER`], and what each client
/// answers: every bare `i64` written as a JSON number, and as the text of the number in a header
/// of the `http_rest` transport.
fn carried_bare() -> Value {
    json!({
        "ran": [
            {
                "bus": {
                    "carried": [
                        "9223372036854775806",
                        [["x-offset", "1"]],
                        r#"{"ok":true,"value":9223372036854775807}"#,
                        [],
                    ],
                    "answered": { "ok": true, "value": ["9223372036854775807n", null] },
                },
                "http": {
                    "carried": [
                        r#"GET /bare/9223372036854775806 [["x-offset","1"]] "#,
                        r#"200 [["content-type","application/json"]] 9223372036854775807"#,
                    ],
                    "answered": { "ok": true, "value": ["9223372036854775807n", null] },
                },
            },
            {
                "bus": {
                    "carried": [
                        "4611686018427387903",
                        [["x-offset", "null"]],
                        r#"{"ok":true,"value":4611686018427387903}"#,
                        [["x-doubled", "9223372036854775806"]],
                    ],
                    "answered": {
                        "ok": true,
                        "value": ["4611686018427387903n", "9223372036854775806n"],
                    },
                },
                "http": {
                    "carried": [
                        "GET /bare/4611686018427387903 [] ",
                        r#"200 [["x-doubled","9223372036854775806"],["content-type","application/json"]] 4611686018427387903"#,
                    ],
                    "answered": {
                        "ok": true,
                        "value": ["4611686018427387903n", "9223372036854775806n"],
                    },
                },
            },
            {
                "bus": {
                    "carried": [
                        "[9223372036854775807,1]",
                        [],
                        r#"{"ok":false,"error":-9223372036854775808}"#,
                        [],
                    ],
                    "answered": { "ok": false, "error": "-9223372036854775808n" },
                },
                "http": {
                    "carried": [
                        "POST /bare [] [9223372036854775807,1]",
                        r#"422 [["content-type","application/json"]] -9223372036854775808"#,
                    ],
                    "answered": { "ok": false, "error": "-9223372036854775808n" },
                },
            },
            {
                "bus": {
                    "carried": ["-9223372036854775808", []],
                    "answered": null,
                },
                "http": {
                    "carried": ["PUT /bare [] -9223372036854775808", "204 [] "],
                    "answered": null,
                },
            },
        ],
        "recorded": ["-9223372036854775808n", "-9223372036854775808n"],
    })
}

/// Each client sends a bare `i64` as a JSON number, in the message and in a header, each
/// dispatcher answers the reply, its header and its declared error as JSON numbers, and each client
/// reads them back into the `bigint` each was, over both transports.
#[test]
fn a_bare_i64_travels_as_a_number_through_each_typescript_client_and_dispatcher() {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(
            REQUIRED_PACKAGES,
            "the emitted service's bare 64-bit integers",
        );
        return;
    };
    let module = format!("{}\n\n{BUS_READ}\n{BARE_DRIVER}", emitted());
    let Some(wrote) = ran_with_modules(
        "bare-wide-integers",
        RUNTIME_VAR,
        "node",
        "bare.mts",
        &module,
        &modules,
    ) else {
        return;
    };
    let ran: Value = serde_json::from_str(wrote.trim()).unwrap();
    assert_eq!(
        ran,
        carried_bare(),
        "a bare 64-bit integer does not travel as a JSON number"
    );
    assert_eq!(
        serde_json::from_str::<i64>(ran["ran"][0]["bus"]["carried"][0].as_str().unwrap()).ok(),
        Some(i64::MAX - 1),
        "serde reads the number the client sends"
    );
    assert_eq!(
        poll_once(TallyingBareBackEnd.total(&(), vec![i64::MAX, 1])),
        Some(Err(i64::MIN)),
        "the backend answers the error the dispatcher answered"
    );
    assert_eq!(
        poll_once(TallyingBareBackEnd.shift(&(), i64::MAX / 2, None)).and_then(Result::ok),
        Some((i64::MAX / 2, Some(i64::MAX - 1))),
        "the backend answers the shift the dispatcher answered"
    );
    assert_eq!(
        poll_once(TallyingBareBackEnd.record(&(), i64::MIN)),
        Some(()),
        "the backend records the one-way call"
    );
}

/// What each transport carries each way, the headers of a request beside its message, and what
/// each client answers, of each call of the driver: the third answered beyond an `i64`, which each
/// client refuses, and the last read by a plain `JSON.parse` on the bus.
fn carried() -> Value {
    json!([
        {
            "bus": {
                "carried": [
                    r#"{"tally":9223372036854775806}"#,
                    [],
                    r#"{"ok":true,"value":{"tally":9223372036854775807}}"#,
                ],
                "answered": { "ok": true, "value": { "tally": "9223372036854775807n" } },
            },
            "http": {
                "carried": [
                    r#"POST /tallies? [] {"tally":9223372036854775806}"#,
                    r#"{"tally":9223372036854775807}"#,
                ],
                "answered": { "ok": true, "value": { "tally": "9223372036854775807n" } },
            },
        },
        {
            "bus": {
                "carried": [
                    r#"{"tally":-9223372036854775807,"limit":-1}"#,
                    [["x-ledger", "9007199254740993"]],
                    r#"{"ok":true,"value":{"tally":-9223372036854775808}}"#,
                ],
                "answered": { "ok": true, "value": { "tally": "-9223372036854775808n" } },
            },
            "http": {
                "carried": [
                    r#"GET /tallies/-9223372036854775807?limit=-1 [["x-ledger","9007199254740993"]] "#,
                    r#"{"tally":-9223372036854775808}"#,
                ],
                "answered": { "ok": true, "value": { "tally": "-9223372036854775808n" } },
            },
        },
        {
            "bus": {
                "carried": [
                    r#"{"tally":9223372036854775807}"#,
                    [],
                    r#"{"ok":true,"value":{"tally":9223372036854775808}}"#,
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
                    r#"POST /tallies? [] {"tally":9223372036854775807}"#,
                    r#"{"tally":9223372036854775808}"#,
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
        {
            "bus": {
                "carried": [
                    r#"{"tally":1719623999999}"#,
                    [],
                    r#"{"ok":true,"value":{"tally":1719624000000}}"#,
                ],
                "answered": { "ok": true, "value": { "tally": "1719624000000n" } },
            },
            "http": {
                "carried": [
                    r#"POST /tallies? [] {"tally":1719623999999}"#,
                    r#"{"tally":1719624000000}"#,
                ],
                "answered": { "ok": true, "value": { "tally": "1719624000000n" } },
            },
        },
    ])
}

/// Each client sends a `bigint` as a JSON number, each dispatcher answers one as a JSON number, and
/// each client reads the answer back into the `bigint` it was, over both transports.
#[test]
fn an_i64_travels_as_a_number_through_each_typescript_client_and_dispatcher() {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(REQUIRED_PACKAGES, "the emitted service's 64-bit integers");
        return;
    };
    let module = format!("{}\n\n{BUS_READ}\n{DRIVER}", emitted());
    let Some(wrote) = ran_with_modules(
        "wide-integers",
        RUNTIME_VAR,
        "node",
        "wide.mts",
        &module,
        &modules,
    ) else {
        return;
    };
    let ran: Value = serde_json::from_str(wrote.trim()).unwrap();
    assert_eq!(
        ran,
        carried(),
        "a 64-bit integer does not travel as a JSON number"
    );
    let read: TallyCount =
        serde_json::from_str(ran[0]["bus"]["carried"][0].as_str().unwrap()).unwrap();
    assert_eq!(
        poll_once(TallyingBackEnd.count(&(), read))
            .map(|answered| answered.map(|count| count.tally).ok()),
        Some(Some(i64::MAX)),
        "the backend counts what serde read of what the client sends"
    );
    assert_eq!(
        poll_once(TallyingBackEnd.fetch(&(), i64::MIN + 1, Some(-1), 0))
            .map(|answered| answered.map(|count| count.tally).ok()),
        Some(Some(i64::MIN)),
        "the backend answers the tally the dispatcher answered"
    );
}

/// Without `JSON.rawJSON`, the client writes a `bigint` a `number` holds exactly as that number and
/// refuses any other as a message that fails validation, the dispatcher rejects as a handler that
/// throws, and the HTTP dispatcher answers the fault of a handler that panicked.
#[test]
fn without_raw_json_a_bigint_beyond_a_number_is_refused_and_any_other_is_sent_as_the_number() {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(
            REQUIRED_PACKAGES,
            "the emitted service's 64-bit integers without JSON.rawJSON",
        );
        return;
    };
    let module = format!("{}\n\n{BUS_READ}\n{FALLBACK_DRIVER}", emitted());
    let Some(wrote) = ran_with_modules(
        "wide-integers-fallback",
        RUNTIME_VAR,
        "node",
        "fallback.mts",
        &module,
        &modules,
    ) else {
        return;
    };
    let ran: Value = serde_json::from_str(wrote.trim()).unwrap();
    let unwritten = "is written exactly only through JSON.rawJSON, which this runtime has not";
    assert_eq!(
        ran,
        json!({
            "sent": [
                {
                    "carried": [r#"{"tally":5}"#, r#"{"ok":true,"value":{"tally":6}}"#],
                    "answered": { "ok": true, "value": { "tally": "6n" } },
                },
                {
                    "carried": [],
                    "answered": {
                        "ok": false,
                        "error": {
                            "isServiceFault": true,
                            "fault": {
                                "detail": format!("9007199254740993 {unwritten}"),
                                "kind": "failed-validation",
                                "operation": "count",
                            },
                        },
                    },
                },
            ],
            "overBus": format!("refused: 9007199254740992 {unwritten}"),
            "overHttp": format!(
                r#"500 {{"detail":"9007199254740992 {unwritten}","kind":"handler-panic","operation":"count"}}"#
            ),
        }),
        "what the client sends and each dispatcher answers without JSON.rawJSON"
    );
}
