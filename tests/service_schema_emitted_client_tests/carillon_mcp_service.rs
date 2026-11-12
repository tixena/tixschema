//! `CarillonMcpService` on `mcp`, whose first tool takes the message of `CarillonService` and
//! answers it, and whose second answers a bell beside a header, and what its Rust dispatcher and
//! client answer to the cases of [`MCP_CASES`] and [`TOLL_CASES`]: the answers every emitted `mcp`
//! server and client is measured against. Nothing here waits on a clock.

use super::carillon_mcp_client::{CarillonMcpServiceClient, McpMessage, Transport};
use super::carillon_mcp_dispatcher;
use super::carillon_service::{
    CASES, CarillonBell, CarillonChime, CarillonError, STRUCK, carillon_bell_schema,
    carillon_chime_schema, payload,
};
use super::mcp_writer_rows::ready_output;
use super::tests::poll_once;
use crate::mcp_streams::through;
use core::fmt::Display;
use core::future::{Future, ready};
use core::iter::repeat_n;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Mutex;
use tixschema::{model_schema, service_schema};

/// The cases of [`CASES`] each `mcp` server is called with and each `mcp` client is answered: the
/// message read whole, a value of the wrong kind, the three refusals `serde_json` classifies as
/// text that is not the document when it reads text, and an integer written as a float in the
/// arguments and in the result.
pub const MCP_CASES: [&str; 19] = [
    "read whole",
    "a text for a number",
    "a Result of two arms",
    "a Result of no arm",
    "a key that is no number",
    "an f64 beyond its range",
    "an integral fraction for a u32",
    "an exponent for a u32",
    "a capital exponent for a u32",
    "a fraction of zero for a u32",
    "a fraction of zero below zero for a u32",
    "a zero below zero for a u32",
    "an integral fraction for an i64",
    "an exponent for an i64",
    "a capital exponent for an i64",
    "a fraction of zero for an i64",
    "a fraction of zero below zero for an i64",
    "a zero below zero for an i64",
    "a one for a u32",
];

/// The answers each `mcp` client is handed as the answer of `toll`: a success and a declared
/// error, each beside a header it reads and beside a header that holds no number.
pub const TOLL_CASES: [&str; 4] = [
    "a rung read",
    "a rung that is no number",
    "a crack read",
    "a crack that is no number",
];

/// The heaviest bell that tolls; a heavier one cracks.
const CRACKS_ABOVE: u32 = 50;

/// The request ids a server echoes, each written as the request writes it: a number serde reads as
/// an integer, as a float by a fraction, an exponent or a zero below zero, and a text.
pub const ID_CASES: [&str; 9] = [
    "7",
    "-5",
    "-0",
    "1e2",
    "1E+2",
    "1.0",
    "0.5",
    "-0.0",
    r#""seven""#,
];

/// The cases of [`CASES`] beyond [`MCP_CASES`] whose field every `mcp` server names as the Rust
/// dispatcher does: a value inside each shape of tagged enum, inside a flattened member and inside
/// a struct with a tag of its own.
const SHAPE_CASES: [&str; 17] = [
    "a key beside an external variant",
    "a key beside an adjacent variant",
    "a key beside an internal variant",
    "an internal variant without its tag",
    "an adjacent variant without its content",
    "a boolean for a number in a variant",
    "a text for a number in a flattened member",
    "a text for a number beside a flattened member",
    "a text for a number in a tagged struct",
    "a flattened member without its member",
    "a weight above its bound in a flattened member",
    "a key beside a flattened member",
    "an external enum of no variant",
    "an external enum of two variants",
    "a number for an external enum",
    "a list for an external enum",
    "a null for an external enum",
];

/// The arguments of `chant` that every `mcp` server answers as the Rust dispatcher does: the verse
/// read whole, and a refrain that matches no variant.
const VERSE_CASES: [(&str, &str); 4] = [
    ("a verse of words", r#"{"refrain":{"text":"a"}}"#),
    ("a verse of beats", r#"{"refrain":{"beats":3}}"#),
    ("a verse of no refrain", "{}"),
    ("a number for a refrain", r#"{"refrain":5}"#),
];

/// The arguments of `chant` whose refrain holds a key, which the Rust dispatcher refuses at the
/// refrain as serde names no variant it could have read: the Dart and Kotlin `mcp` servers name the
/// same field.
const KEYED_VERSE_CASES: [(&str, &str); 3] = [
    ("a refrain of no key", r#"{"refrain":{}}"#),
    (
        "a text for the beats of a refrain",
        r#"{"refrain":{"beats":"x"}}"#,
    ),
    (
        "a key beside the words of a refrain",
        r#"{"refrain":{"text":"a","stray":1}}"#,
    ),
];

/// A verse of a carillon: the refrain it sings.
#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CarillonVerse {
    pub refrain: CarillonRefrain,
}

/// A refrain sung as words or as a count of beats, which serde tells apart by the keys alone.
#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum CarillonRefrain {
    Beats { beats: u32 },
    Words { text: String },
}

/// Strikes the bells of a carillon over `mcp`.
#[service_schema(transports = ["mcp"])]
pub trait CarillonMcpService<Ctx> {
    /// Chants the verse the arguments name, and answers it.
    async fn chant(&self, ctx: &Ctx, req: CarillonVerse) -> Result<CarillonVerse, CarillonError>;

    /// Strikes the chime the arguments name, and answers it.
    async fn strike(&self, ctx: &Ctx, req: CarillonChime) -> Result<CarillonChime, CarillonError>;

    /// Tolls the bell the arguments name, and answers it beside its weight, or that it cracked
    /// beside the weight it cracked at.
    #[service_schema_op(http(
        method = "POST",
        path = "/tolls",
        header_out("x-rung"),
        error_header_out("x-cracked-at")
    ))]
    async fn toll(
        &self,
        ctx: &Ctx,
        req: CarillonBell,
    ) -> Result<(CarillonBell, u32), (CarillonError, u32)>;
}

/// Counts the strikes and chants it is handed.
#[derive(Default)]
pub struct McpCarillon {
    struck: Mutex<u32>,
}

impl CarillonMcpService<()> for McpCarillon {
    fn chant(
        &self,
        _ctx: &(),
        req: CarillonVerse,
    ) -> impl Future<Output = Result<CarillonVerse, CarillonError>> {
        *self.struck.lock().unwrap() += 1;
        ready(Ok(req))
    }

    fn strike(
        &self,
        _ctx: &(),
        req: CarillonChime,
    ) -> impl Future<Output = Result<CarillonChime, CarillonError>> {
        *self.struck.lock().unwrap() += 1;
        ready(Ok(req))
    }

    fn toll(
        &self,
        _ctx: &(),
        req: CarillonBell,
    ) -> impl Future<Output = Result<(CarillonBell, u32), (CarillonError, u32)>> {
        let weight = req.weight;
        ready(if weight > CRACKS_ABOVE {
            Err((CarillonError::Cracked, weight))
        } else {
            Ok((req, weight))
        })
    }
}

/// Where progress is written: nowhere, since these runs compare the answer alone.
pub struct Sink;

impl carillon_mcp_dispatcher::McpEventWriter for Sink {
    fn write(
        &self,
        _message: String,
    ) -> impl Future<Output = Result<(), carillon_mcp_dispatcher::McpClientGone>> {
        ready(Ok(()))
    }
}

/// What a client was handed, and the one text it answers every message with.
struct Canned {
    answer: String,
    sent: Mutex<Vec<Value>>,
}

impl Transport for Canned {
    fn send(&self, message: McpMessage) -> impl Future<Output = Result<String, String>> + Send {
        self.sent.lock().unwrap().push(json!({
            "method": message.method(),
            "name": message.name(),
            "protocolVersion": message.protocol_version(),
            "text": serde_json::from_str::<Value>(message.text()).unwrap(),
        }));
        ready(Ok(self.answer.clone()))
    }
}

/// The payload of the case named `name`.
pub fn mcp_payload(name: &str) -> String {
    payload(CASES.into_iter().find(|case| case.0 == name).unwrap())
}

/// The `tools/call` request `id` that carries `arguments` to `strike`, written as text: a number
/// beyond an `f64` is held by no `Value`.
pub fn call(id: u32, arguments: &str) -> String {
    tool_call(id, "strike", arguments)
}

/// Every request a server is handed beside the Rust dispatcher, with its name: each case of
/// [`MCP_CASES`] and [`SHAPE_CASES`] with the id `1`, each of [`VERSE_CASES`] to `chant`, and the
/// message read whole with each id of [`ID_CASES`]; the Dart and Kotlin servers are also handed
/// [`KEYED_VERSE_CASES`].
pub fn server_cases(typescript: bool) -> Vec<(String, String)> {
    let strikes = MCP_CASES.into_iter().chain(SHAPE_CASES).map(|name| {
        let case = CASES.into_iter().find(|case| case.0 == name).unwrap();
        (name.to_owned(), call(1, &payload(case)))
    });
    let keyed: &[(&str, &str)] = if typescript { &[] } else { &KEYED_VERSE_CASES };
    let verses = VERSE_CASES
        .into_iter()
        .chain(keyed.iter().copied())
        .map(|(name, arguments)| (name.to_owned(), tool_call(1_u32, "chant", arguments)));
    let ids = ID_CASES
        .into_iter()
        .map(|id| (format!("the id {id}"), tool_call(id, "strike", STRUCK)));
    strikes.chain(verses).chain(ids).collect()
}

/// The text a JSON-RPC answer writes for its `id`, `None` where it writes none.
pub fn written_id(answer: &str) -> Option<&str> {
    let at = answer.find(r#""id":"#)? + r#""id":"#.len();
    let rest = &answer[at..];
    let end = match rest.strip_prefix('"') {
        Some(quoted) => quoted.find('"')? + 2,
        None => rest.find([',', '}'])?,
    };
    Some(&rest[..end])
}

/// The `tools/call` request `id` that carries `arguments` to `tool`, written as text.
fn tool_call(id: impl Display, tool: &str, arguments: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":{id},"method":"tools/call","params":{{"name":"{tool}","arguments":{arguments},"_meta":{{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{{}}}}}}}}"#
    )
}

/// The answer the Rust dispatcher writes to `text`, and how many strikes reached the handler.
pub fn over_mcp(text: &str) -> (Option<String>, u32) {
    let carillon = McpCarillon::default();
    let answered = poll_once(carillon_mcp_dispatcher::answer(
        text,
        &carillon,
        |_call| (),
        &Sink,
        &carillon_mcp_dispatcher::McpInFlight::default(),
    ))
    .flatten();
    let struck = *carillon.struck.lock().unwrap();
    (answered, struck)
}

/// What a server's answer to a `tools/call` tells: `"read"` for a success, the fault's kind and
/// field for a tool error, and the code of a JSON-RPC error.
pub fn told(answer: &Value) -> Value {
    if let Some(code) = answer["error"]["code"].as_i64() {
        return json!({ "code": code });
    }
    if answer["result"]["isError"] == false {
        return json!("read");
    }
    let carried: Value =
        serde_json::from_str(answer["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    json!({ "kind": carried["fault"]["kind"], "field": carried["fault"]["field"] })
}

/// What the Rust dispatcher answers each case of [`MCP_CASES`].
pub fn rust_server_answers() -> Vec<Value> {
    MCP_CASES
        .into_iter()
        .map(|name| {
            let (answer, _) = over_mcp(&call(1, &mcp_payload(name)));
            told(&serde_json::from_str(&answer.unwrap()).unwrap())
        })
        .collect()
}

/// The success the Rust dispatcher answers to the message read whole, with `structured` written
/// as its structured content and as the text beside it.
pub fn reply(structured: &str) -> String {
    const HELD: &str = "the structured content";
    let (answer, _) = over_mcp(&call(1, STRUCK));
    let mut answered: Value = serde_json::from_str(&answer.unwrap()).unwrap();
    answered["result"]["content"][0]["text"] = json!(structured);
    answered["result"]["structuredContent"] = json!(HELD);
    answered
        .to_string()
        .replace(&format!("\"{HELD}\""), structured)
}

/// What the Rust client answers `answered` as the answer of a call that sends the message read
/// whole: `"read"` for the message read, and the fault's kind and field otherwise; and what it
/// handed the transport.
pub fn rust_client_answer(answered: String) -> (Value, Vec<Value>) {
    let whole: CarillonChime = serde_json::from_str(STRUCK).unwrap();
    let client = CarillonMcpServiceClient::new(Canned {
        answer: answered,
        sent: Mutex::new(Vec::new()),
    });
    let answer = match poll_once(client.strike(whole)).unwrap() {
        Ok(_) => json!("read"),
        Err(carillon_mcp_service_schema::CallError::Fault(fault)) => fault_told(&fault),
        Err(declared) => json!({ "declared": format!("{declared:?}") }),
    };
    (answer, client.transport().sent.lock().unwrap().clone())
}

/// What the Rust client answers each case of [`MCP_CASES`] handed to it as the success of the
/// call.
pub fn rust_client_answers() -> Vec<Value> {
    MCP_CASES
        .into_iter()
        .map(|name| rust_client_answer(reply(&mcp_payload(name))).0)
        .collect()
}

/// The kind and the field of a fault the Rust client answered.
fn fault_told(fault: &carillon_mcp_service_schema::ServiceFault) -> Value {
    let written = serde_json::to_value(fault).unwrap();
    json!({ "kind": written["kind"], "field": fault.field() })
}

/// The answer each case of [`TOLL_CASES`] hands a client: the Rust dispatcher's answer to a toll
/// of a bell that tolls or of one that cracks, its header written as a text where the case holds
/// no number.
pub fn toll_replies() -> Vec<String> {
    TOLL_CASES
        .into_iter()
        .map(|name| {
            let cracked = name.starts_with("a crack");
            let weight = if cracked { CRACKS_ABOVE + 1 } else { 3 };
            let (answer, _) = over_mcp(&tool_call(
                1_u32,
                "toll",
                &json!({ "weight": weight }).to_string(),
            ));
            let mut answered: Value = serde_json::from_str(&answer.unwrap()).unwrap();
            if name.ends_with("no number") {
                let result = &mut answered["result"];
                let text = &mut result["content"][0]["text"];
                let mut carried: Value = serde_json::from_str(text.as_str().unwrap()).unwrap();
                let header = if cracked { "x-cracked-at" } else { "x-rung" };
                carried[header] = json!("x");
                *text = json!(carried.to_string());
                if !cracked {
                    result["structuredContent"] = carried;
                }
            }
            answered.to_string()
        })
        .collect()
}

/// What the Rust client answers each case of [`TOLL_CASES`] as the answer of a toll: the header
/// it read beside the success or the declared error, and the fault's kind and field otherwise.
pub fn rust_toll_answers() -> Vec<Value> {
    toll_replies()
        .into_iter()
        .map(|answered| {
            let client = CarillonMcpServiceClient::new(Canned {
                answer: answered,
                sent: Mutex::new(Vec::new()),
            });
            match poll_once(client.toll(CarillonBell { weight: 3 })).unwrap() {
                Ok((_, rung)) => json!({ "rung": rung }),
                Err(carillon_mcp_service_schema::CallError::Operation((_, at))) => {
                    json!({ "crackedAt": at })
                }
                Err(carillon_mcp_service_schema::CallError::Fault(fault)) => fault_told(&fault),
            }
        })
        .collect()
}

/// What the Rust dispatcher answers to `text` over HTTP, its call admitted by `admit`: its status
/// and its body, and how many strikes reached the handler.
fn over_http(
    text: &str,
    admit: impl AsyncFn(
        &carillon_mcp_dispatcher::McpHttpRequest,
        carillon_mcp_dispatcher::McpCall,
    ) -> Result<
        carillon_mcp_dispatcher::McpAdmitted<()>,
        carillon_mcp_dispatcher::McpDenied,
    >,
) -> (u16, String, u32) {
    let asked = carillon_mcp_dispatcher::McpHttpRequest::new(
        "POST".to_owned(),
        "/mcp".to_owned(),
        vec![
            ("Content-Type".to_owned(), "application/json".to_owned()),
            ("MCP-Protocol-Version".to_owned(), "2026-07-28".to_owned()),
            ("Mcp-Method".to_owned(), "tools/call".to_owned()),
            ("Mcp-Name".to_owned(), "strike".to_owned()),
        ],
        text.as_bytes().to_vec(),
    );
    let settings = carillon_mcp_dispatcher::McpHttpSettings::new(&[], None);
    let carillon = McpCarillon::default();
    let answered = through(|stream| {
        ready_output(carillon_mcp_dispatcher::answer_http(
            &asked, &settings, stream, &carillon, admit,
        ))
    });
    let struck = *carillon.struck.lock().unwrap();
    (
        answered.status(),
        String::from_utf8(answered.body().to_vec()).unwrap(),
        struck,
    )
}

/// What the Rust dispatcher answers each case of [`MCP_CASES`], measured: `serde_json` reads the
/// message as a `Value` and the arguments out of it, so a number beyond an `f64` refuses the
/// message, a key that is no number is text that is not the document, a `Result` of other than
/// one key is a value of the wrong kind, and an integer written with a fraction or an exponent, or
/// as `-0`, is a float that no integer member reads.
fn measured() -> Vec<Value> {
    let refused = |field: &str| json!({ "kind": "failed-validation", "field": field });
    let mut answers = vec![
        json!("read"),
        refused("strokes"),
        refused("tally"),
        refused("tally"),
        json!({ "kind": "undeserializable-payload", "field": null }),
        json!({ "code": -32_600_i32 }),
    ];
    answers.extend(repeat_n(refused("strokes"), 6));
    answers.extend(repeat_n(refused("depth"), 6));
    answers.push(json!("read"));
    answers
}

#[test]
fn the_rust_dispatcher_answers_what_is_no_document_as_serde_reads_it_from_a_value() {
    assert_eq!(rust_server_answers(), measured());
    for name in MCP_CASES {
        let (_, struck) = over_mcp(&call(1, &mcp_payload(name)));
        assert_eq!(
            struck,
            u32::from(matches!(name, "read whole" | "a one for a u32")),
            "{name}"
        );
    }
    let detail = |name: &str| {
        let answer: Value =
            serde_json::from_str(&over_mcp(&call(1, &mcp_payload(name))).0.unwrap()).unwrap();
        let carried: Value =
            serde_json::from_str(answer["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
        carried["fault"]["detail"].clone()
    };
    assert_eq!(
        detail("a Result of two arms"),
        "'tally': invalid value: map, expected map with a single key"
    );
    assert_eq!(
        detail("a key that is no number"),
        "invalid value: expected key to be a number in quotes"
    );
    let (written, _) = over_mcp(&call(1, &mcp_payload("an f64 beyond its range")));
    let refused: Value = serde_json::from_str(&written.unwrap()).unwrap();
    assert_eq!(refused.get("id"), None, "{refused}");
    assert!(
        refused["error"]["message"]
            .as_str()
            .unwrap()
            .starts_with("Invalid request: number out of range"),
        "{refused}"
    );
}

#[test]
fn the_rust_dispatcher_answers_each_case_over_http_as_it_answers_its_text() {
    for name in MCP_CASES {
        let text = call(1, &mcp_payload(name));
        let (status, body, struck) = over_http(&text, async |_request, _call| {
            Ok(carillon_mcp_dispatcher::McpAdmitted::new(()))
        });
        let (answer, alone) = over_mcp(&text);
        assert_eq!(body, answer.unwrap(), "{name}");
        assert_eq!(struck, alone, "{name}");
        let refused = name == "an f64 beyond its range";
        assert_eq!(status, if refused { 400 } else { 200 }, "{name}");
    }
}

#[test]
fn an_admit_that_denies_stops_a_strike_before_its_arguments_are_read() {
    for (denied, status) in [
        (carillon_mcp_dispatcher::McpDenied::Forbidden, 403_u16),
        (
            carillon_mcp_dispatcher::McpDenied::InsufficientScope {
                scope: "bells:strike".to_owned(),
            },
            403,
        ),
        (
            carillon_mcp_dispatcher::McpDenied::NoValidCredentials { scope: None },
            401,
        ),
    ] {
        let text = call(1, &mcp_payload("a key that is no number"));
        let (answered, _, struck) = over_http(&text, async |request, _call| {
            assert_eq!(request.mcp_name().as_deref(), Some("strike"));
            Err(denied.clone())
        });
        assert_eq!((answered, struck), (status, 0));
    }
}

#[test]
fn the_rust_client_answers_every_success_it_cannot_read_with_no_field() {
    let mut expected = vec![json!("read")];
    expected.extend(
        [None; 18].map(
            |field: Option<&str>| json!({ "kind": "undeserializable-payload", "field": field }),
        ),
    );
    expected[18] = json!("read");
    assert_eq!(rust_client_answers(), expected);
    let (_, sent) = rust_client_answer(reply(STRUCK));
    assert_eq!(sent.len(), 1, "{sent:?}");
    let message = &sent[0];
    assert_eq!(message["method"], "tools/call");
    assert_eq!(message["name"], "strike");
    assert_eq!(message["protocolVersion"], "2026-07-28");
    assert_eq!(
        message["text"]["params"]["arguments"],
        serde_json::from_str::<Value>(STRUCK).unwrap()
    );
}

#[test]
fn the_rust_client_answers_a_header_it_cannot_read_with_no_field() {
    let unread = json!({ "kind": "undeserializable-payload", "field": null });
    assert_eq!(
        rust_toll_answers(),
        [
            json!({ "rung": 3_u32 }),
            unread.clone(),
            json!({ "crackedAt": CRACKS_ABOVE + 1 }),
            unread,
        ]
    );
}

#[test]
fn the_rust_dispatcher_answers_alone_beside_an_endpoint_and_over_http_alike() {
    let (alone, served, status) = rust_serves_alike!(
        carillon_mcp_dispatcher,
        CarillonMcpServiceSchema,
        McpCarillon::default(),
        ready_output
    );
    assert_eq!(alone, served);
    assert!(alone.is_some());
    assert_eq!(status, 200);
}

#[test]
fn the_rust_dispatcher_listens_alike_through_every_entry_point() {
    let (alone, served) = rust_listens_alike!(
        carillon_mcp_dispatcher,
        CarillonMcpServiceSchema,
        McpCarillon::default(),
        ready_output
    );
    assert_eq!(alone, served);
    assert!(alone.is_some());
}

#[test]
fn the_rust_client_reads_the_verse_the_rust_dispatcher_chants() {
    let (answer, struck) = over_mcp(&tool_call(1_u32, "chant", r#"{"refrain":{"beats":3}}"#));
    let client = CarillonMcpServiceClient::new(Canned {
        answer: answer.unwrap(),
        sent: Mutex::new(Vec::new()),
    });
    let verse = CarillonVerse {
        refrain: CarillonRefrain::Beats { beats: 3 },
    };
    assert_eq!(
        poll_once(client.chant(verse.clone())).unwrap().unwrap(),
        verse
    );
    assert_eq!(struck, 1);
}
