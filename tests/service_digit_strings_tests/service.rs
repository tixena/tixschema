//! A service that carries a bare `i64` in each position no member holds one in, its back end, and
//! the loops that join each client to the dispatcher of its transport.

use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context as PollContext, Poll, Waker};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

use crate::{amqp_client, amqp_transport, http_rest_client, http_rest_transport};

/// Each entry a one-way call recorded, in the order recorded.
static RECORDED: Mutex<Vec<i64>> = Mutex::new(Vec::new());

/// The one error the shift declares.
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum LedgerError {
    Overflowed,
}

/// A bare `i64` as the message, as a header beside it, as the reply and as a header beside the
/// reply; and a bare list of them as the message and an `i64` as the declared error.
#[service_schema(transports = ["amqp_rpc", "http_rest"])]
pub trait LedgerService<Ctx> {
    #[service_schema_op(one_way, http(method = "PUT", path = "/ledgers"))]
    async fn record(&self, ctx: &Ctx, entry: i64);

    #[service_schema_op(http(
        method = "GET",
        path = "/ledgers/{entry}",
        header_in("x-offset" = offset),
        header_out("x-doubled"),
    ))]
    async fn shift(
        &self,
        ctx: &Ctx,
        entry: i64,
        offset: Option<i64>,
    ) -> Result<(i64, Option<i64>), LedgerError>;

    #[service_schema_op(http(method = "POST", path = "/ledgers"))]
    async fn total(&self, ctx: &Ctx, entries: Vec<i64>) -> Result<i64, i64>;
}

/// The back end every loop dispatches to.
pub struct LedgerBackEnd;

impl LedgerService<()> for LedgerBackEnd {
    async fn record(&self, _ctx: &(), entry: i64) {
        ready(()).await;
        RECORDED.lock().unwrap().push(entry);
    }

    async fn shift(
        &self,
        _ctx: &(),
        entry: i64,
        offset: Option<i64>,
    ) -> Result<(i64, Option<i64>), LedgerError> {
        ready(()).await;
        let shifted = entry
            .checked_add(offset.unwrap_or(0))
            .ok_or(LedgerError::Overflowed)?;
        Ok((shifted, shifted.checked_mul(2)))
    }

    async fn total(&self, _ctx: &(), entries: Vec<i64>) -> Result<i64, i64> {
        ready(()).await;
        if entries.is_empty() {
            return Err(i64::MIN);
        }
        Ok(entries.iter().copied().fold(0_i64, i64::wrapping_add))
    }
}

/// What one AMQP message carried each way: its payload and headers, then the reply's.
type Carried = (String, Vec<(String, String)>, String, Vec<(String, String)>);

/// How the dispatcher settled one AMQP message: the reply's payload and its headers.
type Settled = (Vec<u8>, Vec<(String, String)>);

/// What one HTTP call is held to: the request line, its headers and body, then the response's
/// status and body.
type Seen<'call> = (
    &'call str,
    &'call [(String, String)],
    &'call str,
    u16,
    &'call str,
);

/// Captures how the dispatcher settled one AMQP message.
struct AmqpCapture {
    settled: Mutex<Option<Settled>>,
}

impl amqp_transport::Reply for AmqpCapture {
    async fn fault(&self, fault: ledger_service_schema::ServiceFault) {
        ready(()).await;
        let framed = serde_json::json!({
            "ok": false,
            "error": { "isServiceFault": true, "fault": fault },
        });
        *self.settled.lock().unwrap() = Some((serde_json::to_vec(&framed).unwrap(), Vec::new()));
    }

    async fn send<T>(&self, value: T, headers: Vec<(String, String)>)
    where
        T: Serialize + Send,
    {
        ready(()).await;
        *self.settled.lock().unwrap() = Some((serde_json::to_vec(&value).unwrap(), headers));
    }
}

/// Joins the `amqp_rpc` client to its dispatcher, recording what each call carried.
struct AmqpLoop {
    carried: Mutex<Vec<Carried>>,
}

impl amqp_client::Transport for AmqpLoop {
    async fn notify<T>(
        &self,
        operation: &str,
        payload: T,
        headers: Vec<(String, String)>,
    ) -> Result<(), String>
    where
        T: Serialize + Send,
    {
        let sent = serde_json::to_vec(&payload).unwrap();
        let capture = AmqpCapture {
            settled: Mutex::new(None),
        };
        amqp_transport::dispatch(
            &LedgerBackEnd,
            &(),
            &amqp_transport::IncomingMessage::new(
                operation.to_owned(),
                sent.clone(),
                headers.clone(),
            ),
            &capture,
        )
        .await;
        self.carried.lock().unwrap().push((
            String::from_utf8(sent).unwrap(),
            headers,
            String::new(),
            Vec::new(),
        ));
        Ok(())
    }

    async fn request<T>(
        &self,
        operation: &str,
        payload: T,
        headers: Vec<(String, String)>,
    ) -> Result<(Vec<u8>, Vec<(String, String)>), String>
    where
        T: Serialize + Send,
    {
        let sent = serde_json::to_vec(&payload).unwrap();
        let capture = AmqpCapture {
            settled: Mutex::new(None),
        };
        amqp_transport::dispatch(
            &LedgerBackEnd,
            &(),
            &amqp_transport::IncomingMessage::new(
                operation.to_owned(),
                sent.clone(),
                headers.clone(),
            ),
            &capture,
        )
        .await;
        let (answered, replied) = capture.settled.lock().unwrap().take().unwrap();
        self.carried.lock().unwrap().push((
            String::from_utf8(sent).unwrap(),
            headers,
            String::from_utf8(answered.clone()).unwrap(),
            replied.clone(),
        ));
        Ok((answered, replied))
    }
}

/// What one HTTP call carried each way: the request line, its headers and body, then the
/// response's status, headers and body.
type Exchanged = (
    String,
    Vec<(String, String)>,
    String,
    u16,
    Vec<(String, String)>,
    String,
);

/// Joins the `http_rest` client to its dispatcher, recording what each call carried.
struct HttpLoop {
    exchanged: Mutex<Vec<Exchanged>>,
}

impl http_rest_client::Transport for HttpLoop {
    async fn send(
        &self,
        request: http_rest_client::OutgoingRequest,
    ) -> Result<http_rest_client::IncomingResponse, String> {
        let incoming = http_rest_transport::IncomingRequest::new(
            request.method().to_owned(),
            request.path().to_owned(),
            request.query().to_owned(),
            request.headers().to_vec(),
            request.body().to_vec(),
        );
        let answered = http_rest_transport::dispatch(
            &LedgerBackEnd,
            &(),
            &incoming,
            &http_rest_transport::DefaultFaultHandler,
        )
        .await;
        self.exchanged.lock().unwrap().push((
            format!(
                "{} {}{}",
                incoming.method(),
                incoming.path(),
                incoming.query()
            ),
            incoming.headers().to_vec(),
            String::from_utf8(request.body().to_vec()).unwrap(),
            answered.status(),
            answered.headers().to_vec(),
            String::from_utf8(answered.body().to_vec()).unwrap(),
        ));
        Ok(http_rest_client::IncomingResponse::new(
            answered.status(),
            answered.headers().to_vec(),
            answered.body().to_vec(),
        ))
    }
}

/// An owner's `FaultHandler`, which answers every fault with a status and a header of its own.
struct Refusing;

impl http_rest_transport::FaultHandler for Refusing {
    fn on_fault(
        &self,
        fault: &ledger_service_schema::ServiceFault,
    ) -> http_rest_transport::OutgoingResponse {
        http_rest_transport::OutgoingResponse::new(
            499,
            vec![("x-fault".to_owned(), format!("{}", fault.kind()))],
            Vec::new(),
        )
    }
}

/// The answer of `answering` after one poll, or `None` where it is left pending.
fn poll_once<Answered>(answering: Answered) -> Option<Answered::Output>
where
    Answered: Future,
{
    let mut pinned = pin!(answering);
    let mut polling = PollContext::from_waker(Waker::noop());
    match pinned.as_mut().poll(&mut polling) {
        Poll::Ready(answer) => Some(answer),
        Poll::Pending => None,
    }
}

/// `pairs` as owned header pairs.
fn headed(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect()
}

/// The `amqp_rpc` client sends a bare `i64` message and header as their digits, and reads the
/// digits of the reply, its header and its declared error back into the integers.
#[test]
fn an_i64_carried_bare_travels_as_its_digits_over_amqp() {
    let client = amqp_client::LedgerServiceClient::new(AmqpLoop {
        carried: Mutex::new(Vec::new()),
    });
    assert_eq!(
        poll_once(client.shift(i64::MAX - 1, Some(1))),
        Some(Ok((i64::MAX, None))),
        "the shift"
    );
    assert_eq!(
        poll_once(client.total(vec![i64::MIN + 1, -1])),
        Some(Ok(i64::MIN)),
        "the total"
    );
    assert_eq!(
        poll_once(client.total(Vec::new())),
        Some(Err(ledger_service_schema::CallError::Operation(i64::MIN))),
        "the declared error"
    );
    assert_eq!(
        client.transport().carried.lock().unwrap().clone(),
        [
            (
                r#""9223372036854775806""#.to_owned(),
                headed(&[("x-offset", r#""1""#)]),
                r#"{"ok":true,"value":"9223372036854775807"}"#.to_owned(),
                Vec::new(),
            ),
            (
                r#"["-9223372036854775807","-1"]"#.to_owned(),
                Vec::new(),
                r#"{"ok":true,"value":"-9223372036854775808"}"#.to_owned(),
                Vec::new(),
            ),
            (
                "[]".to_owned(),
                Vec::new(),
                r#"{"error":"-9223372036854775808","ok":false}"#.to_owned(),
                Vec::new(),
            ),
        ],
        "what each call carried"
    );
}

/// A header beside the reply is written as its digits, and the dispatcher reads a message and a
/// header written as a number, as serde reads them.
#[test]
fn the_amqp_dispatcher_writes_a_reply_header_as_its_digits_and_reads_a_number() {
    let capture = AmqpCapture {
        settled: Mutex::new(None),
    };
    assert_eq!(
        poll_once(amqp_transport::dispatch(
            &LedgerBackEnd,
            &(),
            &amqp_transport::IncomingMessage::new(
                "shift".to_owned(),
                b"4611686018427387903".to_vec(),
                headed(&[("x-offset", "0")]),
            ),
            &capture,
        )),
        Some(()),
        "the dispatch"
    );
    let (answered, replied) = capture.settled.into_inner().unwrap().unwrap();
    assert_eq!(
        String::from_utf8(answered).unwrap(),
        r#"{"ok":true,"value":"4611686018427387903"}"#,
        "the reply"
    );
    assert_eq!(
        replied,
        headed(&[("x-doubled", r#""9223372036854775806""#)]),
        "the header beside it"
    );
    let client = amqp_client::LedgerServiceClient::new(AmqpLoop {
        carried: Mutex::new(Vec::new()),
    });
    assert_eq!(
        poll_once(client.shift(i64::MAX / 2, None)),
        Some(Ok((i64::MAX / 2, Some(i64::MAX - 1)))),
        "the client reads the header back"
    );
}

/// The `http_rest` client writes a bare `i64` message in the path, a header beside it in a header
/// and a list of them as the body, the dispatcher answers the reply, its header and its declared
/// error as their digits, and the client reads each back into the integers.
#[test]
fn an_i64_carried_bare_travels_as_its_digits_over_http() {
    let client = http_rest_client::LedgerServiceClient::new(HttpLoop {
        exchanged: Mutex::new(Vec::new()),
    });
    assert_eq!(
        poll_once(client.shift(i64::MIN + 1, Some(-1))),
        Some(Ok((i64::MIN, None))),
        "the shift"
    );
    assert_eq!(
        poll_once(client.shift(i64::MAX / 2, None)),
        Some(Ok((i64::MAX / 2, Some(i64::MAX - 1)))),
        "the shift with a header beside its reply"
    );
    assert_eq!(
        poll_once(client.total(vec![i64::MAX])),
        Some(Ok(i64::MAX)),
        "the total"
    );
    assert_eq!(
        poll_once(client.total(Vec::new())),
        Some(Err(ledger_service_schema::CallError::Operation(i64::MIN))),
        "the declared error"
    );
    let exchanged = client.transport().exchanged.lock().unwrap().clone();
    let carried: Vec<Seen<'_>> = exchanged
        .iter()
        .map(|(line, sent, body, status, _replied, answered)| {
            (
                line.as_str(),
                sent.as_slice(),
                body.as_str(),
                *status,
                answered.as_str(),
            )
        })
        .collect();
    assert_eq!(
        carried,
        [
            (
                "GET /ledgers/-9223372036854775807",
                headed(&[("x-offset", "-1")]).as_slice(),
                "",
                200,
                r#""-9223372036854775808""#,
            ),
            (
                "GET /ledgers/4611686018427387903",
                [].as_slice(),
                "",
                200,
                r#""4611686018427387903""#,
            ),
            (
                "POST /ledgers",
                [].as_slice(),
                r#"["9223372036854775807"]"#,
                200,
                r#""9223372036854775807""#,
            ),
            (
                "POST /ledgers",
                [].as_slice(),
                "[]",
                422,
                r#""-9223372036854775808""#,
            ),
        ],
        "what each call carried"
    );
    assert!(
        exchanged[1]
            .4
            .contains(&("x-doubled".to_owned(), "9223372036854775806".to_owned())),
        "the header beside the reply: {:?}",
        exchanged[1].4
    );
}

/// A one-way call sends its bare `i64` as its digits over each transport.
#[test]
fn a_one_way_call_sends_a_bare_i64_as_its_digits() {
    let amqp = amqp_client::LedgerServiceClient::new(AmqpLoop {
        carried: Mutex::new(Vec::new()),
    });
    assert_eq!(poll_once(amqp.record(i64::MIN)), Some(Ok(())), "over amqp");
    assert_eq!(
        amqp.transport().carried.lock().unwrap()[0].0,
        r#""-9223372036854775808""#,
        "what amqp carried"
    );
    let http = http_rest_client::LedgerServiceClient::new(HttpLoop {
        exchanged: Mutex::new(Vec::new()),
    });
    assert_eq!(poll_once(http.record(i64::MAX)), Some(Ok(())), "over http");
    assert_eq!(
        http.transport().exchanged.lock().unwrap()[0].2,
        r#""9223372036854775807""#,
        "what http carried"
    );
    let recorded = RECORDED.lock().unwrap().clone();
    assert!(
        recorded.contains(&i64::MIN) && recorded.contains(&i64::MAX),
        "the back end recorded both: {recorded:?}"
    );
}

/// Each route the `http_rest` dispatcher answers, with its statuses, and a fault answered through
/// an owner's own handler.
#[test]
fn the_route_table_lists_each_route() {
    let routes: Vec<(&str, &str, &str, u16, &[u16])> = http_rest_transport::ROUTES
        .iter()
        .map(|route| {
            (
                route.operation(),
                route.method(),
                route.path(),
                route.ok_status(),
                route.error_statuses(),
            )
        })
        .collect();
    assert_eq!(
        routes,
        [
            ("record", "PUT", "/ledgers", 204, [].as_slice()),
            (
                "shift",
                "GET",
                "/ledgers/{entry}",
                200,
                [422_u16].as_slice()
            ),
            ("total", "POST", "/ledgers", 200, [422_u16].as_slice()),
        ],
        "the routes"
    );
    let answered = poll_once(http_rest_transport::dispatch(
        &LedgerBackEnd,
        &(),
        &http_rest_transport::IncomingRequest::new(
            "GET".to_owned(),
            "/ledgers/seven".to_owned(),
            String::new(),
            Vec::new(),
            Vec::new(),
        ),
        &Refusing,
    ))
    .unwrap();
    assert_eq!(answered.status(), 499, "the owner's status");
    assert_eq!(
        answered.headers(),
        [("x-fault".to_owned(), "failed validation".to_owned())],
        "the owner's header"
    );
}

/// The `http_rest` dispatcher reads a bare `i64` body written as a number, as serde reads it.
#[test]
fn the_http_dispatcher_reads_a_body_written_as_numbers() {
    let answered = poll_once(http_rest_transport::dispatch(
        &LedgerBackEnd,
        &(),
        &http_rest_transport::IncomingRequest::new(
            "POST".to_owned(),
            "/ledgers".to_owned(),
            String::new(),
            Vec::new(),
            b"[9223372036854775806,1]".to_vec(),
        ),
        &http_rest_transport::DefaultFaultHandler,
    ))
    .unwrap();
    assert_eq!(answered.status(), 200, "the status");
    assert_eq!(
        answered.body(),
        br#""9223372036854775807""#,
        "the reply written as its digits"
    );
}
