//! The numbers `jsonDecode` and kotlinx read apart from serde, each written into the message of
//! `CarillonService`: what the Rust `http_rest` client of `CarillonHttpService`, and the Rust `mcp`
//! dispatcher and client of `CarillonMcpService`, answer to each. Every emitted `http_rest` client,
//! `mcp` server and `mcp` client is measured against these answers. Nothing here waits on a clock.

use super::carillon_http_rest_client::{
    CarillonHttpServiceClient, IncomingResponse, OutgoingRequest, Transport,
};
use super::carillon_mcp_service::{call, over_mcp, reply, rust_client_answer, told};
use super::carillon_service::{CarillonChime, CarillonError, STRUCK, payload};
use super::tests::poll_once;
use core::future::{Future, ready};
use serde_json::{Value, json};
use std::sync::Mutex;
use tixschema::service_schema;

/// The cases each surface reads: `-0` for an integer, which serde reads as the float `-0.0` and
/// refuses, as it refuses `1.0` and `1e2`, and `-0` for an `f64`, which it reads as `-0.0`; beside
/// them the numbers and the text that hold no `-0` serde reads, and an `f32` read as an infinity,
/// which serde writes as `null`.
pub const NUMBER_CASES: [(&str, &str, Option<&str>); 14] = [
    ("read whole", "", None),
    ("a zero below zero for a u32", "strokes", Some("-0")),
    ("a zero below zero for an i64", "depth", Some("-0")),
    ("a zero below zero in a list", "rounds.1", Some("-0")),
    ("an integral fraction for a u32", "strokes", Some("1.0")),
    ("an exponent for an i64", "depth", Some("1e2")),
    ("a zero below zero for an f64", "sway", Some("-0")),
    ("a zero for a u32", "strokes", Some("0")),
    ("a one for a u32", "strokes", Some("1")),
    ("a one below zero for an i64", "depth", Some("-1")),
    (
        "an exponent of zero below zero for an f64",
        "sway",
        Some("1e-0"),
    ),
    ("a text of a zero below zero", "label", Some(r#""-0""#)),
    ("an f32 beyond its range", "ring", Some("1e39")),
    ("an f32 beyond its range below zero", "ring", Some("-1e39")),
];

/// The cases serde refuses, and the field each names: an integer written as a float, `-0` among
/// them, which it reads as the float `-0.0`.
const REFUSED: [(&str, &str); 5] = [
    ("a zero below zero for a u32", "strokes"),
    ("a zero below zero for an i64", "depth"),
    ("a zero below zero in a list", "rounds.1"),
    ("an integral fraction for a u32", "strokes"),
    ("an exponent for an i64", "depth"),
];

/// Strikes the bells of a carillon over `http_rest`.
#[service_schema(transports = ["http_rest"])]
pub trait CarillonHttpService<Ctx> {
    /// Strikes the chime the message names, and answers it.
    async fn strike(&self, ctx: &Ctx, req: CarillonChime) -> Result<CarillonChime, CarillonError>;
}

/// Counts the strikes it is handed.
#[derive(Default)]
pub struct HttpCarillon {
    struck: Mutex<u32>,
}

impl CarillonHttpService<()> for HttpCarillon {
    fn strike(
        &self,
        _ctx: &(),
        req: CarillonChime,
    ) -> impl Future<Output = Result<CarillonChime, CarillonError>> {
        *self.struck.lock().unwrap() += 1;
        ready(Ok(req))
    }
}

/// Answers every call with `body` at the operation's own status, once the handler has been handed
/// the message the call carries.
struct Answering {
    body: String,
    carillon: HttpCarillon,
}

impl Transport for Answering {
    async fn send(&self, request: OutgoingRequest) -> Result<IncomingResponse, String> {
        assert_eq!(
            (request.method(), request.path(), request.query()),
            ("POST", "/strike", "")
        );
        assert!(request.headers().is_empty(), "{:?}", request.headers());
        let sent: CarillonChime = serde_json::from_slice(request.body()).unwrap();
        self.carillon.strike(&(), sent).await.unwrap();
        let response = IncomingResponse::new(
            200,
            vec![("Content-Type".to_owned(), "application/json".to_owned())],
            self.body.clone().into_bytes(),
        );
        assert_eq!(response.header("content-type"), Some("application/json"));
        Ok(response)
    }
}

/// Each payload of [`NUMBER_CASES`].
pub fn number_payloads() -> Vec<String> {
    NUMBER_CASES.into_iter().map(payload).collect()
}

/// What the Rust `http_rest` client answers each payload of [`NUMBER_CASES`] handed to it as the
/// body of the success: `"read"`, or the fault's kind and field.
pub fn rust_http_answers() -> Vec<Value> {
    let whole: CarillonChime = serde_json::from_str(STRUCK).unwrap();
    number_payloads()
        .into_iter()
        .map(|body| {
            let client = CarillonHttpServiceClient::new(Answering {
                body,
                carillon: HttpCarillon::default(),
            });
            let answer = match poll_once(client.strike(whole.clone())).unwrap() {
                Ok(_) => json!("read"),
                Err(carillon_http_service_schema::CallError::Fault(fault)) => {
                    let written = serde_json::to_value(&fault).unwrap();
                    json!({ "kind": written["kind"], "field": fault.field() })
                }
                Err(declared) => json!({ "declared": format!("{declared:?}") }),
            };
            assert_eq!(*client.transport().carillon.struck.lock().unwrap(), 1);
            answer
        })
        .collect()
}

/// Each `tools/call` request a server is handed, in the order of [`NUMBER_CASES`].
pub fn number_requests() -> Vec<String> {
    number_payloads()
        .iter()
        .map(|arguments| call(1, arguments))
        .collect()
}

/// Each answer a client is handed, in the order of [`NUMBER_CASES`].
pub fn number_replies() -> Vec<String> {
    number_payloads().iter().map(|text| reply(text)).collect()
}

/// What a server's answer to a `tools/call` tells, as [`told`] tells it, with a success's
/// structured content and the text beside it each written back by `serde_json`, which writes
/// `-0.0` apart from `0.0`.
pub fn summed(answer: &Value) -> Value {
    let said = told(answer);
    if said != "read" {
        return said;
    }
    let text: Value =
        serde_json::from_str(answer["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    json!({
        "structured": answer["result"]["structuredContent"].to_string(),
        "text": text.to_string(),
    })
}

/// What the Rust `mcp` dispatcher answers each request of [`number_requests`], as [`summed`]
/// tells it.
pub fn rust_number_server_answers() -> Vec<Value> {
    number_requests()
        .iter()
        .map(|text| {
            let (answer, _) = over_mcp(text);
            summed(&serde_json::from_str(&answer.unwrap()).unwrap())
        })
        .collect()
}

/// What the Rust `mcp` client answers each reply of [`number_replies`].
pub fn rust_number_client_answers() -> Vec<Value> {
    number_replies()
        .into_iter()
        .map(|answered| rust_client_answer(answered).0)
        .collect()
}

#[test]
fn the_rust_client_and_dispatcher_refuse_a_zero_below_zero_for_an_integer_alone() {
    let refused = |name: &str| REFUSED.iter().find(|(case, _)| *case == name);
    let http: Vec<Value> = NUMBER_CASES
        .into_iter()
        .map(|(name, _, _)| match refused(name) {
            Some(_) => json!({ "kind": "undeserializable-payload", "field": null }),
            None => json!("read"),
        })
        .collect();
    assert_eq!(rust_http_answers(), http);
    assert_eq!(rust_number_client_answers(), http);
    let served = rust_number_server_answers();
    for ((name, _, _), answer) in NUMBER_CASES.into_iter().zip(&served) {
        match refused(name) {
            Some((_, field)) => assert_eq!(
                *answer,
                json!({ "kind": "failed-validation", "field": field }),
                "{name}"
            ),
            None => assert!(answer["structured"].is_string(), "{name}: {answer}"),
        }
    }
    let structured = |name: &str| -> Value {
        let at = NUMBER_CASES
            .iter()
            .position(|(case, _, _)| *case == name)
            .unwrap();
        serde_json::from_str(served[at]["structured"].as_str().unwrap()).unwrap()
    };
    assert_eq!(
        structured("a zero below zero for an f64")["sway"].to_string(),
        "-0.0"
    );
    assert_eq!(
        structured("an exponent of zero below zero for an f64")["sway"].to_string(),
        "1.0"
    );
    assert_eq!(structured("a text of a zero below zero")["label"], "-0");
    assert_eq!(structured("an f32 beyond its range")["ring"], Value::Null);
    assert_eq!(
        structured("an f32 beyond its range below zero")["ring"],
        Value::Null
    );
    assert_eq!(structured("read whole")["depth"], json!(i64::MIN));
}
