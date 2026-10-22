//! A service whose message is a `NaiveDateTime`, whose headers in are a `char` and a
//! `DateTime<Utc>`, and whose reply and header out are each a `DateTime<Utc>`, with the exchange
//! the Rust dispatchers answer for it: what each client sends of the values is read back here
//! through those dispatchers, and what they write is what each client is handed to read.

#![cfg(all(
    feature = "chrono",
    any(
        all(feature = "typescript", feature = "zod"),
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    )
))]

use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context as PollContext, Poll, Waker};
use std::sync::Mutex;

use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::{model_schema, service_schema};

use super::{sundial_http_rest_transport, sundial_ws_rpc_transport};

/// The message every client is handed.
pub const REQ: &str = "2024-01-05T10:20:30";

/// The `char` every client is handed for `x-letter`.
pub const LETTER: &str = "q";

/// The `DateTime<Utc>` every client is handed for `x-at`, which the back end answers twice.
pub const AT: &str = "2024-01-05T10:20:30.250Z";

/// The `x-letter` a client refuses before it sends anything: two characters.
pub const NO_LETTER: &str = "ab";

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum SundialError {
    Overcast,
}

#[service_schema(transports = ["http_rest", "ws_rpc"])]
pub trait SundialClientService<Ctx> {
    #[service_schema_op(http(
        method = "POST",
        path = "/read",
        header_in("x-letter" = letter),
        header_in("x-at" = at),
        header_out("x-seen"),
    ))]
    async fn read_dial(
        &self,
        ctx: &Ctx,
        req: NaiveDateTime,
        letter: char,
        at: DateTime<Utc>,
    ) -> Result<(DateTime<Utc>, DateTime<Utc>), SundialError>;
}

/// An HTTP response: its status, its headers and its body as text.
pub type Answered = (u16, Vec<(String, String)>, String);

/// What the back end was handed by one call.
pub type Handed = (NaiveDateTime, char, DateTime<Utc>);

/// A back end that holds what each call handed it and answers `at` as the value and as `x-seen`.
#[derive(Default)]
pub struct SundialBackEnd {
    handed: Mutex<Vec<Handed>>,
}

impl SundialClientService<()> for SundialBackEnd {
    async fn read_dial(
        &self,
        _ctx: &(),
        req: NaiveDateTime,
        letter: char,
        at: DateTime<Utc>,
    ) -> Result<(DateTime<Utc>, DateTime<Utc>), SundialError> {
        ready(()).await;
        self.handed.lock().unwrap().push((req, letter, at));
        Ok((at, at))
    }
}

/// A fault answered under a status no default writes, as a caller's own handler answers it.
struct AnsweredAs499;

impl sundial_http_rest_transport::FaultHandler for AnsweredAs499 {
    fn on_fault(
        &self,
        fault: &sundial_client_service_schema::ServiceFault,
    ) -> sundial_http_rest_transport::OutgoingResponse {
        sundial_http_rest_transport::OutgoingResponse::new(
            499,
            vec![("x-fault-kind".to_owned(), format!("{}", fault.kind()))],
            Vec::new(),
        )
    }
}

/// What each client is handed, as the back end reads it.
pub fn canonical() -> Handed {
    (REQ.parse().unwrap(), 'q', AT.parse().unwrap())
}

/// The back end never suspends, so one poll answers it.
fn poll_once<Answering>(answering: Answering) -> Option<Answering::Output>
where
    Answering: Future,
{
    let mut pinned = pin!(answering);
    match pinned
        .as_mut()
        .poll(&mut PollContext::from_waker(Waker::noop()))
    {
        Poll::Ready(answer) => Some(answer),
        Poll::Pending => None,
    }
}

/// What the Rust `http_rest` dispatcher answers `request`, a JSON object of the `method`, `path`,
/// `query`, `headers` pairs and `body` text a client sent, and what the back end was handed.
pub fn over_http(request: &Value) -> (Answered, Vec<Handed>) {
    let text = |key: &str| request[key].as_str().unwrap().to_owned();
    let headers = request["headers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|pair| {
            (
                pair[0].as_str().unwrap().to_owned(),
                pair[1].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    let incoming = sundial_http_rest_transport::IncomingRequest::new(
        text("method"),
        text("path"),
        text("query"),
        headers,
        text("body").into_bytes(),
    );
    let backend = SundialBackEnd::default();
    let response = poll_once(sundial_http_rest_transport::dispatch(
        &backend,
        &(),
        &incoming,
        &sundial_http_rest_transport::DefaultFaultHandler,
    ))
    .unwrap();
    let answered = (
        response.status(),
        response.headers().to_vec(),
        String::from_utf8(response.body().to_vec()).unwrap(),
    );
    (answered, backend.handed.into_inner().unwrap())
}

/// What the Rust `ws_rpc` dispatcher answers `frame`, and what the back end was handed.
pub fn over_ws(frame: &str) -> (Option<String>, Vec<Handed>) {
    let backend = SundialBackEnd::default();
    let replied = poll_once(sundial_ws_rpc_transport::answer(frame, &backend, &())).unwrap();
    (replied, backend.handed.into_inner().unwrap())
}

/// The response the Rust `http_rest` dispatcher answers the call with the canonical values.
pub fn http_answer() -> Answered {
    let (answered, handed) = over_http(&json!({
        "method": "POST",
        "path": "/read",
        "query": "",
        "headers": [["x-letter", LETTER], ["x-at", AT]],
        "body": serde_json::to_string(REQ).unwrap(),
    }));
    assert_eq!(
        handed,
        [canonical()],
        "the dispatcher hands on other values"
    );
    answered
}

/// The reply frame the Rust `ws_rpc` dispatcher answers the call with the canonical values, under
/// the id `ID`.
pub fn ws_reply() -> String {
    let frame = json!({
        "kind": "request",
        "id": "ID",
        "service": "SundialClientService",
        "operation": "read-dial",
        "payload": REQ,
        "headers": { "x-letter": LETTER, "x-at": AT },
    });
    let (replied, handed) = over_ws(&frame.to_string());
    assert_eq!(
        handed,
        [canonical()],
        "the dispatcher hands on other values"
    );
    replied.unwrap()
}

/// Holds the six lines a client printed: the request it sent over `http_rest`, what it read of the
/// answer, `refused` for the call with [`NO_LETTER`], then the same three over `ws_rpc`, the
/// request being the frame. Each request is handed to the Rust dispatcher of its transport, which
/// reads the canonical values from it, and each read is [`AT`] twice.
pub fn held(printed: &str) {
    let lines: Vec<Value> = printed
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let read = json!([AT, AT]);
    let refused = json!("refused");
    assert_eq!(lines.len(), 6, "the client printed other lines:\n{printed}");
    let (answered, handed) = over_http(&lines[0]);
    assert_eq!(
        (answered.0, handed),
        (200, vec![canonical()]),
        "the Rust dispatcher reads other values of the request {}: {}",
        lines[0],
        answered.2
    );
    let frame = lines[3].as_str().unwrap();
    let (replied, framed) = over_ws(frame);
    assert!(
        replied.is_some_and(|reply| reply.contains("\"ok\":true")) && framed == [canonical()],
        "the Rust dispatcher reads other values of the frame {frame}"
    );
    assert_eq!(
        [&lines[1], &lines[2], &lines[4], &lines[5]],
        [&read, &refused, &read, &refused],
        "the client reads another value, or sends what it refuses:\n{printed}"
    );
}

/// The route table holds the operation as it is declared, a request reads back what it was built
/// with, and a `char` header of two characters is refused through the handler, the back end handed
/// nothing.
#[test]
fn the_route_table_and_a_refused_request_hold_what_the_service_declares() {
    let routes: Vec<_> = sundial_http_rest_transport::ROUTES
        .iter()
        .map(|route| {
            (
                route.method(),
                route.path(),
                route.operation(),
                route.ok_status(),
                route.error_statuses(),
            )
        })
        .collect();
    let declared: &[u16] = &[422];
    assert_eq!(
        routes,
        [("POST", "/read", "read-dial", 200, declared)],
        "the route table holds another operation"
    );
    let request = sundial_http_rest_transport::IncomingRequest::new(
        "POST".to_owned(),
        "/read".to_owned(),
        "unused=1".to_owned(),
        vec![
            ("x-letter".to_owned(), NO_LETTER.to_owned()),
            ("x-at".to_owned(), AT.to_owned()),
        ],
        serde_json::to_vec(REQ).unwrap(),
    );
    assert_eq!(
        (request.query(), request.headers().len()),
        ("unused=1", 2),
        "the request reads back other parts"
    );
    let backend = SundialBackEnd::default();
    let refused = poll_once(sundial_http_rest_transport::dispatch(
        &backend,
        &(),
        &request,
        &AnsweredAs499,
    ))
    .unwrap();
    assert_eq!(
        (refused.status(), refused.headers()),
        (
            499,
            &[("x-fault-kind".to_owned(), "failed validation".to_owned())][..]
        ),
        "the dispatcher reads a header of two characters as a `char`"
    );
    assert!(
        backend.handed.into_inner().unwrap().is_empty(),
        "the back end was handed a refused call"
    );
}
