//! `SoundingService` on `ws_rpc`, whose one operation takes a message of an `i64`, a `u32`, an
//! `f64` and an `f32` and answers the message it was handed, run by node with the real `zod`. A
//! number written with a fraction or an exponent, `1.0`, `1e2`, `1E2`, `0.0`, `-0.0` and `-0`, in
//! each member, is handed to the TypeScript `ws_rpc` attachment and client and to the HTTP
//! dispatcher and client: each answers what the Rust dispatcher answers, `failed-validation` naming
//! an integer's member, which the HTTP client answers as the body it cannot read, and reads a
//! float's number. A runtime whose reviver hands no source text reads each as a number, as before.
//! What serde reads beside such a number still reads through the reader those surfaces share: a
//! literal on an `f64`, a discriminated union told apart by a numeric literal on an `f64` key, and
//! each `preprocess` and `check` function is handed a `number`, as `JSON.parse` hands it. What
//! serde refuses is refused beside it: a literal on a `u32`, a discriminated union told apart by a
//! numeric literal on a `u32` key, and an integer a `preprocess` function answers unchanged.

use core::fmt::Write as _;
use core::future::{Future, ready};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::{model_schema, service_schema};

use super::runtime::{compiled_with_modules, node_modules, ran_with_modules, stand_down_modules};
use super::sounding_ws_rpc_transport;
use super::tests::poll_once;

const RUNTIME_VAR: &str = "TIXSCHEMA_NODE";

const REQUIRED_PACKAGES: &[&str] = &["zod"];

/// The numbers serde refuses for an integer and reads for a float, each written with a fraction or
/// an exponent, `-0` among them, which serde reads as the float `-0.0`; and `1`, which every member
/// reads.
const WRITTEN: [&str; 7] = ["1.0", "1e2", "1E2", "0.0", "-0.0", "-0", "1"];

/// Each member of [`Sounding`], in the order the message writes them.
const MEMBERS: [&str; 4] = ["depth", "fathoms", "reading", "swell"];

/// Hands each message to the attachment, to the client as the reply, to the HTTP dispatcher as
/// the request's body and to the HTTP client as the response's, and prints what each answered: the
/// number read in the member the row names, or the fault's kind and field.
const DRIVER: &str = r#"
const told = (member, answered) =>
  answered.ok
    ? { read: Number(answered.value[member]) }
    : answered.error.isServiceFault
      ? { kind: answered.error.fault.kind, field: answered.error.fault.field ?? null }
      : "dropped";

const whole = { depth: 7n, fathoms: 7, reading: 7, swell: 7 };

const impl = {
  async take(_ctx, req) {
    return { ok: true, value: req };
  },
};

async function attached(member, payload) {
  let listener;
  let answer;
  const replied = new Promise((resolve) => { answer = resolve; });
  const socket = {
    send(text) { answer(text); },
    close() {},
    addEventListener(type, added) { if (type === "message") listener = added; },
    removeEventListener() {},
  };
  const detach = attachSoundingServiceWsDispatcher(socket, {}, impl, () => {});
  listener({ data: `{"kind":"request","id":"ID","service":"SoundingService","operation":"take","payload":${payload}}` });
  const reply = JSON.parse(await replied);
  detach();
  return reply.ok ? { read: Number(reply.value[member]) } : { kind: reply.error.fault.kind, field: reply.error.fault.field ?? null };
}

async function read(member, payload) {
  let listener;
  const socket = {
    send(text) {
      const { id } = JSON.parse(text);
      queueMicrotask(() => {
        listener({ data: `{"kind":"reply","id":${JSON.stringify(id)},"service":"SoundingService","ok":true,"value":${payload}}` });
        listener({ data: `{"kind":"reply","id":${JSON.stringify(id)},"service":"SoundingService","ok":false,"error":{"errorCode":"Fouled"}}` });
      });
    },
    close() {},
    addEventListener(type, added) { if (type === "message") listener = added; },
    removeEventListener() {},
  };
  const client = createSoundingServiceClient(createSoundingServiceWsTransport(socket, { heartbeat: false }));
  return told(member, await client.take(whole));
}

const handle = createSoundingServiceHttpDispatcher(impl);

async function handled(member, payload) {
  const response = await handle(undefined, {
    body: new TextEncoder().encode(payload),
    headers: [["content-type", "application/json"]],
    method: "POST",
    path: "/soundings",
    query: "",
  });
  const body = JSON.parse(new TextDecoder().decode(response.body));
  return response.status === 200 ? { read: Number(body[member]) } : { kind: body.kind, field: body.field ?? null };
}

async function fetched(member, payload) {
  const client = createSoundingServiceHttpClient({
    async send() {
      return { status: 200, headers: [["content-type", "application/json"]], body: new TextEncoder().encode(payload) };
    },
  });
  return told(member, await client.take(whole));
}

async function main() {
  if (!WITH_SOURCE) {
    const parse = JSON.parse;
    JSON.parse = (text, reviver) =>
      parse(text, reviver === undefined ? undefined : function (key, value) { return reviver.call(this, key, value); });
  }
  const answered = [];
  for (const [member, payload] of ROWS) {
    answered.push({
      attached: await attached(member, payload),
      read: await read(member, payload),
      handled: await handled(member, payload),
      fetched: await fetched(member, payload),
    });
  }
  console.log(JSON.stringify(answered));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"#;

/// The payloads read through the service's reader beside an integer written as a float: the name,
/// the type that reads it, and the payload.
const HELD_APART: [(&str, &str, &str); 19] = [
    (
        "an f64 literal written as a float",
        "Leveled",
        r#"{"float_level":2.0,"whole_level":2}"#,
    ),
    (
        "a u32 literal written as a float",
        "Leveled",
        r#"{"float_level":2,"whole_level":2.0}"#,
    ),
    (
        "a numeric key written as a float",
        "Graded",
        r#"{"grade":2.0,"gain":1}"#,
    ),
    (
        "a numeric key written whole",
        "Graded",
        r#"{"grade":2,"gain":1}"#,
    ),
    (
        "the other numeric key written as a float",
        "Graded",
        r#"{"grade":1.0,"note":"a"}"#,
    ),
    (
        "floats under a function",
        "Plumbed",
        r#"{"depth":1.0,"gauge":{"level":4.0},"marks":[2.0,3],"reading":5.0}"#,
    ),
    (
        "floats under a function, written whole",
        "Plumbed",
        r#"{"depth":1,"gauge":{"level":4},"marks":[2,3],"reading":5}"#,
    ),
    (
        "a u32 literal written whole",
        "Leveled",
        r#"{"float_level":2.0,"whole_level":2}"#,
    ),
    (
        "a u32 key written as a float",
        "Ranked",
        r#"{"grade":2.0,"gain":1}"#,
    ),
    (
        "a u32 key written whole",
        "Ranked",
        r#"{"grade":2,"gain":1}"#,
    ),
    (
        "the f64 key of a union beside a u32 key, written as a float",
        "Mixed",
        r#"{"grade":3.0,"note":"a"}"#,
    ),
    (
        "the u32 key of a union beside an f64 key, written as a float",
        "Mixed",
        r#"{"grade":2.0,"gain":1}"#,
    ),
    (
        "integers under a function, written as floats",
        "Counted",
        r#"{"count":1.0,"level":4,"marks":[2],"tail":{"depth":3}}"#,
    ),
    (
        "integers under a function, in a list, written as a float",
        "Counted",
        r#"{"count":1,"level":4,"marks":[2.0],"tail":{"depth":3}}"#,
    ),
    (
        "integers under a function, in an object, written as a float",
        "Counted",
        r#"{"count":1,"level":4,"marks":[2],"tail":{"depth":3.0}}"#,
    ),
    (
        "integers under a function, written whole",
        "Counted",
        r#"{"count":1,"level":4,"marks":[2],"tail":{"depth":3}}"#,
    ),
    (
        "an integer a function changed, written as a float",
        "Replaced",
        r#"{"count":1.0}"#,
    ),
    (
        "an integer a function reads from text, written as text",
        "Replaced",
        r#"{"count":"9"}"#,
    ),
    (
        "an f64 under a function, written as a float",
        "Counted",
        r#"{"count":1,"level":4.0,"marks":[2],"tail":{"depth":3}}"#,
    ),
];

/// Reads each of [`HELD_APART`] through the reader the `ws_rpc` attachment and client share, and
/// prints what each schema read, the variant a discriminated union read named by its numeric key,
/// and the type of each number every `preprocess` and `check` function was handed.
const HELD_APART_DRIVER: &str = r#"
const handed = [];
function typedWhole(value) {
  handed.push(["typedWhole", typeof value.depth, typeof value.gauge.level, typeof value.marks[0], typeof value.reading]);
  return value;
}
function typedDepth(value) {
  handed.push(["typedDepth", typeof value]);
  return value;
}
function checkedWhole(value, ctx) {
  handed.push(["checkedWhole", typeof value.depth, typeof value.gauge.level, typeof value.marks[0], typeof value.reading]);
  if (value.marks.length === 0) ctx.issues.push({ code: "custom", message: "a plumb line holds a mark", input: value });
}
function checkedReading(value, ctx) {
  handed.push(["checkedReading", typeof value]);
  if (Number.isNaN(value)) ctx.issues.push({ code: "custom", message: "a reading is a number", input: value });
}
function typedTally(value) {
  handed.push(["typedTally", typeof value.count, typeof value.marks[0], typeof value.tail.depth, typeof value.level]);
  return value;
}
function typedCount(value) {
  handed.push(["typedCount", typeof value]);
  return value;
}
function replacedCount(value) {
  return typeof value === "string" ? Number(value) : 3;
}
const schemas = { Leveled: Leveled$Schema, Graded: Graded$Schema, Plumbed: Plumbed$Schema, Ranked: Ranked$Schema, Mixed: Mixed$Schema, Counted: Counted$Schema, Replaced: Replaced$Schema };
const answered = [];
for (const [name, read, text] of HELD_APART) {
  handed.length = 0;
  const parsed = schemas[read].safeParse(soundingServiceWsRead(text));
  answered.push({
    name,
    read: parsed.success ? JSON.parse(JSON.stringify(parsed.data)) : parsed.error.issues.map((issue) => issue.message),
    handed: [...handed],
  });
}
console.log(JSON.stringify(answered));
process.exit(0);
"#;

/// A level held to `2`, as an `f64` and as a `u32`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Leveled {
    #[model_schema_prop(literal = 2)]
    pub float_level: f64,
    #[model_schema_prop(literal = 2)]
    pub whole_level: u32,
}

/// A grade told apart by a numeric literal on the key `grade`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Graded {
    High {
        #[model_schema_prop(literal = 2)]
        grade: f64,
        gain: u32,
    },
    Low {
        #[model_schema_prop(literal = 1)]
        grade: f64,
        note: String,
    },
}

/// A grade told apart by a numeric literal on the key `grade`, an integer.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Ranked {
    High {
        #[model_schema_prop(literal = 2)]
        grade: u32,
        gain: u32,
    },
    Low {
        #[model_schema_prop(literal = 1)]
        grade: u32,
        note: String,
    },
}

/// A grade told apart by a numeric literal on the key `grade`, an integer in one variant and a
/// float in the other.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Mixed {
    Fractional {
        #[model_schema_prop(literal = 3)]
        grade: f64,
        note: String,
    },
    Whole {
        #[model_schema_prop(literal = 2)]
        grade: u32,
        gain: u32,
    },
}

/// A depth, an integer.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Reach {
    pub depth: u32,
}

/// Integers at each depth, under a function of the type's own and a function of a member's.
#[model_schema(preprocess = ["typedTally"])]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Counted {
    #[model_schema_prop(preprocess = ["typedCount"])]
    pub count: u32,
    pub level: f64,
    pub marks: Vec<u32>,
    pub tail: Reach,
}

/// An integer a function answers another number for, or reads from text.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Replaced {
    #[model_schema_prop(preprocess = ["replacedCount"])]
    pub count: u32,
}

/// A gauge whose level is a float.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Gauge {
    pub level: f64,
}

/// A reading over a float, judged by a check of its own.
#[model_schema(check = ["checkedReading"])]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Reading(f64);

/// Floats at each depth, under a function and a check of the type's own and a function of a
/// member's.
#[model_schema(preprocess = ["typedWhole"], check = ["checkedWhole"])]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Plumbed {
    #[model_schema_prop(preprocess = ["typedDepth"])]
    pub depth: f64,
    pub gauge: Gauge,
    pub marks: Vec<f64>,
    pub reading: Reading,
}

/// A sounding, each member a number of its own kind.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Sounding {
    pub depth: i64,
    pub fathoms: u32,
    pub reading: f64,
    pub swell: f32,
}

/// The one error the operation declares.
#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "errorCode")]
pub enum SoundingError {
    Fouled,
}

/// Takes a sounding, and answers it.
#[service_schema(transports = ["ws_rpc"])]
pub trait SoundingService<Ctx> {
    /// Takes the sounding the message holds, and answers it.
    #[service_schema_op(http(method = "POST", path = "/soundings"))]
    async fn take(&self, ctx: &Ctx, req: Sounding) -> Result<Sounding, SoundingError>;
}

/// Answers the sounding it is handed.
pub struct Sounder;

impl SoundingService<()> for Sounder {
    fn take(
        &self,
        _ctx: &(),
        req: Sounding,
    ) -> impl Future<Output = Result<Sounding, SoundingError>> {
        ready(Ok(req))
    }
}

/// The function of [`Plumbed`]: it hands the payload on.
const fn typed_whole(payload: Value) -> Value {
    payload
}

/// The function of [`Plumbed::depth`]: it hands the payload on.
const fn typed_depth(payload: Value) -> Value {
    payload
}

/// The function of [`Counted`]: it hands the payload on.
const fn typed_tally(payload: Value) -> Value {
    payload
}

/// The function of [`Counted::count`]: it hands the payload on.
const fn typed_count(payload: Value) -> Value {
    payload
}

/// The function of [`Replaced::count`]: it answers a text as the number it holds and any other
/// payload as 3.
fn replaced_count(payload: Value) -> Value {
    match payload {
        Value::String(text) => text
            .parse::<u32>()
            .map_or(Value::Null, |number| json!(number)),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::Array(_) | Value::Object(_) => {
            json!(3)
        }
    }
}

/// The check of [`Plumbed`]: a plumb line holds a mark.
fn checked_whole(value: &PlumbedDraft) -> Result<(), Vec<String>> {
    if value.marks.is_empty() {
        Err(vec!["a plumb line holds a mark".to_owned()])
    } else {
        Ok(())
    }
}

/// The check of [`Reading`]: a reading is a number.
fn checked_reading(value: &ReadingDraft) -> Result<(), Vec<String>> {
    if value.as_inner().is_nan() {
        Err(vec!["a reading is a number".to_owned()])
    } else {
        Ok(())
    }
}

/// The message with `written` in `member`, each other member a number every member reads.
fn payload(member: &str, written: &str) -> String {
    let held = |named: &str| if named == member { written } else { "7" };
    format!(
        r#"{{"depth":{},"fathoms":{},"reading":{},"swell":{}}}"#,
        held("depth"),
        held("fathoms"),
        held("reading"),
        held("swell")
    )
}

/// Each member beside each number [`WRITTEN`] holds, and the message that holds it there.
fn rows() -> Vec<(&'static str, &'static str, String)> {
    MEMBERS
        .into_iter()
        .flat_map(|member| {
            WRITTEN
                .into_iter()
                .map(move |written| (member, written, payload(member, written)))
        })
        .collect()
}

/// What the Rust `ws_rpc` dispatcher answers the message: the number it read in the member, or the
/// fault's kind and field.
fn rust_answer(member: &str, payload: &str) -> Value {
    let framed = format!(
        r#"{{"kind":"request","id":"ID","service":"SoundingService","operation":"take","payload":{payload}}}"#
    );
    let replied = poll_once(sounding_ws_rpc_transport::answer(&framed, &Sounder, &()))
        .flatten()
        .unwrap();
    let reply: Value = serde_json::from_str(&replied).unwrap();
    if reply["ok"] == true {
        json!({ "read": reply["value"][member].as_f64().unwrap() })
    } else {
        json!({
            "kind": reply["error"]["fault"]["kind"],
            "field": reply["error"]["fault"]["field"],
        })
    }
}

/// The messages, the service, its attachment and dispatchers and its clients, beside the real
/// `zod`.
fn emitted() -> String {
    [
        "import { z, type ZodType, type $brand } from \"zod\";".to_owned(),
        Sounding::ts_definition(),
        Sounding::zod_schema(),
        SoundingError::ts_definition(),
        SoundingError::zod_schema(),
        SoundingServiceSchema::ts_definition(),
        SoundingServiceSchema::ts_service(),
        SoundingServiceSchema::ts_client(),
        SoundingServiceSchema::ts_ws_client(),
        SoundingServiceSchema::ts_ws_service(),
        SoundingServiceSchema::ts_http_service(),
        SoundingServiceSchema::ts_http_client(),
    ]
    .join("\n\n")
}

/// [`emitted`] beside the types of [`HELD_APART`].
fn emitted_beside_held_apart() -> String {
    [
        emitted(),
        Leveled::ts_definition(),
        Leveled::zod_schema(),
        Graded::ts_definition(),
        Graded::zod_schema(),
        Gauge::ts_definition(),
        Gauge::zod_schema(),
        Reading::ts_definition(),
        Reading::zod_schema(),
        Plumbed::ts_definition(),
        Plumbed::zod_schema(),
        Ranked::ts_definition(),
        Ranked::zod_schema(),
        Mixed::ts_definition(),
        Mixed::zod_schema(),
        Reach::ts_definition(),
        Reach::zod_schema(),
        Counted::ts_definition(),
        Counted::zod_schema(),
        Replaced::ts_definition(),
        Replaced::zod_schema(),
    ]
    .join("\n\n")
}

/// `value` with each number written as an `f64`, so `2` and `2.0` are held equal.
fn as_floats(value: &Value) -> Value {
    match value {
        Value::Number(number) => json!(number.as_f64().unwrap()),
        Value::Array(items) => Value::Array(items.iter().map(as_floats).collect()),
        Value::Object(members) => Value::Object(
            members
                .iter()
                .map(|(key, member)| (key.clone(), as_floats(member)))
                .collect(),
        ),
        Value::Null | Value::Bool(_) | Value::String(_) => value.clone(),
    }
}

/// What serde reads each of [`HELD_APART`] as, written back, or `refused`.
fn serde_reads(read_as: &str, text: &str) -> Value {
    let written = match read_as {
        "Leveled" => serde_json::from_str::<Leveled>(text).map(|read| json!(read)),
        "Graded" => serde_json::from_str::<Graded>(text).map(|read| json!(read)),
        "Ranked" => serde_json::from_str::<Ranked>(text).map(|read| json!(read)),
        "Mixed" => serde_json::from_str::<Mixed>(text).map(|read| json!(read)),
        "Counted" => serde_json::from_str::<Counted>(text).map(|read| json!(read)),
        "Replaced" => serde_json::from_str::<Replaced>(text).map(|read| json!(read)),
        _ => serde_json::from_str::<Plumbed>(text).map(|read| json!(read)),
    };
    written.map_or_else(|_| json!("refused"), |read| as_floats(&read))
}

/// What each surface answered each row of [`rows`], run on a runtime that hands a reviver the
/// source text where `with_source` holds, and on one made to hand it none otherwise.
fn answered(with_source: bool) -> Option<Value> {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(
            REQUIRED_PACKAGES,
            "the emitted service's integers written as floats",
        );
        return None;
    };
    let mut listed = String::new();
    for (member, _, payload) in rows() {
        let _ = writeln!(listed, "  [\"{member}\", String.raw`{payload}`],");
    }
    let module = format!(
        "{}\n\nconst WITH_SOURCE = {with_source};\nconst ROWS = [\n{listed}];\n{DRIVER}",
        emitted()
    );
    let wrote = ran_with_modules(
        "integral-floats",
        RUNTIME_VAR,
        "node",
        "integral-floats.mts",
        &module,
        &modules,
    )?;
    Some(serde_json::from_str(wrote.trim()).unwrap())
}

/// Each row named by its member and number, beside what `answer` makes of the row at its index: a
/// number read written as an `f64`, so `1` and `1.0`, and `0` and `-0.0`, are held equal.
fn named(answer: impl Fn(usize, &str, &str) -> Value) -> Vec<Value> {
    rows()
        .iter()
        .enumerate()
        .map(|(at, (member, written, payload))| {
            let mut told = answer(at, member, payload);
            if let Some(read) = told.get("read").and_then(Value::as_f64) {
                told = json!({ "read": read });
            }
            json!({ "member": member, "written": written, "answer": told })
        })
        .collect()
}

#[test]
fn each_typescript_surface_refuses_an_integer_written_as_a_float_as_the_rust_dispatcher_does() {
    let Some(ran) = answered(true) else {
        return;
    };
    let rust = named(|_, member, payload| rust_answer(member, payload));
    for surface in ["attached", "read", "handled"] {
        let told = named(|at, _, _| ran[at][surface].clone());
        assert_eq!(told, rust, "what the TypeScript {surface} answered");
    }
    let fetched = named(|at, _, _| ran[at]["fetched"].clone());
    let unread = named(|_, member, payload| match rust_answer(member, payload) {
        read if read.get("read").is_some() => read,
        _ => json!({ "kind": "undeserializable-payload", "field": null }),
    });
    assert_eq!(fetched, unread, "what the TypeScript HTTP client answered");
}

#[test]
fn the_rust_dispatcher_refuses_an_integer_written_as_a_float_and_reads_a_float() {
    for (member, written, payload) in rows() {
        let answer = rust_answer(member, &payload);
        let integer = matches!(member, "depth" | "fathoms");
        if integer && written != "1" {
            assert_eq!(
                answer,
                json!({ "kind": "failed-validation", "field": member }),
                "{payload}"
            );
        } else {
            let held: f64 = written.parse().unwrap();
            assert_eq!(answer, json!({ "read": held }), "{payload}");
        }
    }
}

#[test]
fn without_the_source_text_every_number_reads_as_before() {
    let Some(ran) = answered(false) else {
        return;
    };
    for surface in ["attached", "read", "handled", "fetched"] {
        let told = named(|at, _, _| ran[at][surface].clone());
        let before = named(|at, _, _| json!({ "read": rows()[at].1.parse::<f64>().unwrap() }));
        assert_eq!(told, before, "what the TypeScript {surface} answered");
    }
}

#[test]
fn what_reads_beside_an_integer_written_as_a_float_reads_as_before() {
    assert!(
        Graded::zod_schema().contains("z.discriminatedUnion(\"grade\""),
        "{}",
        Graded::zod_schema()
    );
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(
            REQUIRED_PACKAGES,
            "what the emitted service reads beside an integer written as a float",
        );
        return;
    };
    let listed: Vec<(&str, &str, &str)> = HELD_APART.to_vec();
    let module = format!(
        "{}\n\nconst HELD_APART = {};\n{HELD_APART_DRIVER}",
        emitted_beside_held_apart(),
        serde_json::to_string(&listed).unwrap()
    );
    let Some(wrote) = ran_with_modules(
        "integral-floats-beside",
        RUNTIME_VAR,
        "node",
        "beside.mts",
        &module,
        &modules,
    ) else {
        return;
    };
    let ran: Value = serde_json::from_str(wrote.trim()).unwrap();
    let told: Vec<Value> = ran
        .as_array()
        .unwrap()
        .iter()
        .map(|answered| json!({ "name": answered["name"], "read": as_floats(&answered["read"]) }))
        .collect();
    let refused: Vec<&str> = HELD_APART
        .iter()
        .filter(|(_, read_as, text)| serde_reads(read_as, text) == "refused")
        .map(|(name, _, _)| *name)
        .collect();
    assert_eq!(
        refused,
        [
            "a u32 literal written as a float",
            "a u32 key written as a float",
            "the u32 key of a union beside an f64 key, written as a float",
            "integers under a function, written as floats",
            "integers under a function, in a list, written as a float",
            "integers under a function, in an object, written as a float",
        ]
    );
    let read_as_serde: Vec<Value> = HELD_APART
        .iter()
        .zip(ran.as_array().unwrap())
        .map(|((name, read_as, text), answered)| {
            let read = serde_reads(read_as, text);
            if read == "refused" {
                assert!(
                    answered["read"]
                        .as_array()
                        .is_some_and(|issues| !issues.is_empty()),
                    "{name} must be refused: {answered}"
                );
                json!({ "name": name, "read": answered["read"] })
            } else {
                json!({ "name": name, "read": read })
            }
        })
        .collect();
    assert_eq!(told, read_as_serde, "what the schemas read: {ran}");
    for answered in ran.as_array().unwrap() {
        for handed in answered["handed"].as_array().unwrap() {
            let types = handed.as_array().unwrap();
            assert!(
                types[1..].iter().all(|kind| kind == "number"),
                "{} handed {handed}",
                answered["name"]
            );
        }
    }
    let functions: Vec<&Value> = ran[5]["handed"]
        .as_array()
        .unwrap()
        .iter()
        .map(|handed| &handed[0])
        .collect();
    assert_eq!(
        functions,
        ["typedWhole", "typedDepth", "checkedReading", "checkedWhole"]
    );
}

#[test]
fn the_float_schemas_read_into_the_types_they_read_into_before() {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(
            REQUIRED_PACKAGES,
            "the type-check of the emitted integral-float service",
        );
        return;
    };
    let source = format!(
        "{}\n\n\
         type Same<Left, Right> = [Left] extends [Right] ? ([Right] extends [Left] ? true : false) \
         : false;\n\
         export const readAsBefore: Same<\n  z.infer<typeof Sounding$RawSchema>,\n  \
         {{ depth: bigint; fathoms: number; reading: number; swell: number }}\n> = true;\n\
         export const writtenAsBefore: Same<z.output<typeof Sounding$RawSchema>, Sounding> = \
         true;\n\
         export const leveledAsBefore: Same<z.infer<typeof Leveled$RawSchema>, Leveled> = true;\n\
         export const gradedAsBefore: Same<z.infer<typeof Graded$RawSchema>, Graded> = true;\n\
         export function narrowed(value: z.infer<typeof Graded$RawSchema>): number {{\n  \
         return value.grade === 2 ? value.gain : value.note.length;\n}}\n\
         export const rankedAsBefore: Same<z.infer<typeof Ranked$RawSchema>, Ranked> = true;\n\
         export const mixedAsBefore: Same<z.infer<typeof Mixed$RawSchema>, Mixed> = true;\n\
         export const countedAsBefore: Same<z.infer<typeof Counted$RawSchema>, Counted> = true;\n\
         export const replacedAsBefore: Same<z.infer<typeof Replaced$RawSchema>, Replaced> = true;\n\
         export function narrowedRank(value: z.infer<typeof Ranked$RawSchema>): number {{\n  \
         return value.grade === 2 ? value.gain : value.note.length;\n}}\n\
         export function typedTally(value: unknown): unknown {{\n  return value;\n}}\n\
         export function typedCount(value: unknown): unknown {{\n  return value;\n}}\n\
         export function replacedCount(value: unknown): unknown {{\n  return value;\n}}\n\
         export function typedWhole(value: unknown): unknown {{\n  return value;\n}}\n\
         export function typedDepth(value: unknown): unknown {{\n  return value;\n}}\n\
         export function checkedWhole(value: Plumbed, ctx: {{ issues: unknown[] }}): void {{\n  \
         void value;\n  void ctx;\n}}\n\
         export function checkedReading(value: number, ctx: {{ issues: unknown[] }}): void {{\n  \
         void value;\n  void ctx;\n}}\n",
        emitted_beside_held_apart()
    );
    let Some((succeeded, said)) = compiled_with_modules(
        "integral-float-types",
        "TIXSCHEMA_TSC",
        &[("bundle.ts", &source)],
        &modules,
    ) else {
        return;
    };
    assert!(succeeded, "the emitted text does not type-check:\n{said}");
}
