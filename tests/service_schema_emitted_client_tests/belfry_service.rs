//! `BelfryService` on `ws_rpc` and `http_rest`, one operation of which takes no argument and one a
//! message of its own, and what the Rust dispatchers answer to each message: the message reaches
//! the handler, and a key no member declares and a member left out answer `failed-validation`
//! naming the field and reach no handler. The emitted dispatchers of every language are measured
//! against these answers. Nothing here waits on a clock.

use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context, Poll, Waker};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Mutex;
use tixschema::{model_schema, service_schema};

use super::{belfry_http_rest_transport, belfry_ws_rpc_transport};

/// The payloads every dispatcher is handed for `ring-bell`: the message itself, and the message
/// with a key no member declares.
pub const PAYLOADS: [&str; 2] = ["{}", r#"{"stray":1}"#];

/// The payloads every dispatcher is handed for `toll-bell`: the message, a key no member declares
/// beside its members and beside the members of the struct it holds, and the message without one
/// of its members.
pub const TOLLS: [&str; 4] = [
    r#"{"rope":{"length":3},"strokes":2}"#,
    r#"{"rope":{"length":3},"strokes":2,"stray":1}"#,
    r#"{"rope":{"length":3,"stray":1},"strokes":2}"#,
    r#"{"rope":{"length":3}}"#,
];

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BellPeal {
    pub strokes: u32,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum BellError {
    Cracked,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BellRope {
    pub length: u32,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BellToll {
    pub rope: BellRope,
    pub strokes: u32,
}

/// Rings the bell of a belfry.
#[service_schema(transports = ["ws_rpc", "http_rest"], openapi)]
pub trait BelfryService<Ctx> {
    /// Rings the bell once.
    async fn ring_bell(&self, ctx: &Ctx) -> Result<BellPeal, BellError>;

    /// Tolls the bell the strokes the message names.
    async fn toll_bell(&self, ctx: &Ctx, req: BellToll) -> Result<BellPeal, BellError>;
}

/// Counts the rings it is handed.
#[derive(Default)]
pub struct Belfry {
    rung: Mutex<u32>,
}

/// Answers every fault as its own JSON, at the status the default handler answers it at.
struct FaultAsJson;

impl Belfry {
    /// How many rings reached the handler.
    pub fn rung(&self) -> u32 {
        *self.rung.lock().unwrap()
    }
}

impl BelfryService<()> for Belfry {
    fn ring_bell(&self, _ctx: &()) -> impl Future<Output = Result<BellPeal, BellError>> {
        let mut rung = self.rung.lock().unwrap();
        *rung += 1;
        ready(Ok(BellPeal { strokes: *rung }))
    }

    fn toll_bell(
        &self,
        _ctx: &(),
        req: BellToll,
    ) -> impl Future<Output = Result<BellPeal, BellError>> {
        *self.rung.lock().unwrap() += 1;
        ready(Ok(BellPeal {
            strokes: req.strokes * req.rope.length,
        }))
    }
}

impl belfry_http_rest_transport::FaultHandler for FaultAsJson {
    fn on_fault(
        &self,
        fault: &belfry_service_schema::ServiceFault,
    ) -> belfry_http_rest_transport::OutgoingResponse {
        let status = belfry_http_rest_transport::DefaultFaultHandler
            .on_fault(fault)
            .status();
        belfry_http_rest_transport::OutgoingResponse::new(
            status,
            vec![("content-type".to_owned(), "application/json".to_owned())],
            serde_json::to_vec(fault).unwrap(),
        )
    }
}

/// What `answering` answers on its first poll, which a dispatcher over this back end always
/// reaches.
fn once<Answered>(answering: Answered) -> Option<Answered::Output>
where
    Answered: Future,
{
    let mut pinned = pin!(answering);
    match pinned
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(answer) => Some(answer),
        Poll::Pending => None,
    }
}

/// The request frame that carries `payload` to `operation`.
pub fn frame(operation: &str, payload: &str) -> String {
    json!({
        "kind": "request",
        "id": "ID",
        "service": "BelfryService",
        "operation": operation,
        "payload": serde_json::from_str::<Value>(payload).unwrap(),
    })
    .to_string()
}

/// The reply the Rust `ws_rpc` dispatcher writes to `payload` sent to `operation`, and how many
/// rings reached the handler.
pub fn over_ws(operation: &str, payload: &str) -> (Value, u32) {
    let belfry = Belfry::default();
    let reply = once(belfry_ws_rpc_transport::answer(
        &frame(operation, payload),
        &belfry,
        &(),
    ))
    .flatten()
    .unwrap();
    (serde_json::from_str(&reply).unwrap(), belfry.rung())
}

/// The status and body the Rust `http_rest` dispatcher answers a `POST` of `payload` to
/// `/{operation}` with, and how many rings reached the handler.
pub fn over_http(operation: &str, payload: &str) -> (u16, Value, u32) {
    let belfry = Belfry::default();
    let request = belfry_http_rest_transport::IncomingRequest::new(
        "POST".to_owned(),
        format!("/{operation}"),
        String::new(),
        vec![("content-type".to_owned(), "application/json".to_owned())],
        payload.as_bytes().to_vec(),
    );
    assert_eq!(request.query(), "");
    assert_eq!(request.header("content-type"), Some("application/json"));
    assert_eq!(request.headers().len(), 1);
    let response = once(belfry_http_rest_transport::dispatch(
        &belfry,
        &(),
        &request,
        &FaultAsJson,
    ))
    .unwrap();
    assert!(
        response
            .headers()
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case("content-type"))
    );
    (
        response.status(),
        serde_json::from_slice(response.body()).unwrap(),
        belfry.rung(),
    )
}

#[test]
fn the_rust_dispatchers_read_an_empty_message_and_refuse_a_key_beside_it() {
    let (read, rung_reading) = over_ws("ring-bell", PAYLOADS[0]);
    assert_eq!(
        read,
        json!({ "kind": "reply", "id": "ID", "service": "BelfryService", "ok": true, "value": { "strokes": 1_u8 } })
    );
    assert_eq!(rung_reading, 1);
    let (refused, rung_refusing) = over_ws("ring-bell", PAYLOADS[1]);
    assert_eq!(refused["ok"], false, "{refused}");
    assert_eq!(refused["error"]["fault"]["kind"], "failed-validation");
    assert_eq!(refused["error"]["fault"]["field"], "stray");
    assert_eq!(rung_refusing, 0);

    let routes = belfry_http_rest_transport::ROUTES;
    assert_eq!(routes.len(), 2);
    assert_eq!(
        (
            routes[0].method(),
            routes[0].path(),
            routes[0].operation(),
            routes[0].ok_status(),
            routes[0].error_statuses(),
        ),
        ("POST", "/ring-bell", "ring-bell", 200, &[422][..])
    );
    assert_eq!(
        over_http("ring-bell", PAYLOADS[0]),
        (200, json!({ "strokes": 1_u8 }), 1)
    );
    let (status, fault, rung) = over_http("ring-bell", PAYLOADS[1]);
    assert_eq!(status, 400, "{fault}");
    assert_eq!(fault["kind"], "failed-validation");
    assert_eq!(fault["field"], "stray");
    assert_eq!(rung, 0);
}

#[test]
fn the_rust_dispatchers_name_a_key_no_member_of_a_message_declares_and_a_member_left_out() {
    assert_eq!(
        over_http("toll-bell", TOLLS[0]),
        (200, json!({ "strokes": 6_u8 }), 1)
    );
    for (payload, field) in [
        (TOLLS[1], "stray"),
        (TOLLS[2], "rope.stray"),
        (TOLLS[3], "strokes"),
    ] {
        let (status, fault, rung) = over_http("toll-bell", payload);
        assert_eq!((status, rung), (400, 0), "{fault}");
        assert_eq!(fault["field"], field, "{fault}");
        let (refused, rung_refusing) = over_ws("toll-bell", payload);
        assert_eq!(refused["error"]["fault"], fault, "{refused}");
        assert_eq!(rung_refusing, 0);
    }
}
