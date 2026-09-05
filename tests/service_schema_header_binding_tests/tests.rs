//! One service asking for both `amqp_rpc` and `http_rest`, with one operation binding a claimed
//! request header and a declared response header — the shape every other harness in this crate
//! deliberately declares none of.
//!
//! `http_rest` is named beside `amqp_rpc` and builds nothing of its own; naming it is what proves
//! a bound operation is legal on a dual-transport service without this crate depending on anything
//! that transport does not yet emit.

#![cfg(feature = "serde")]

use crate::{amqp_client, amqp_transport};
use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context as PollContext, Poll, Waker};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tixschema::{model_schema, service_schema};

#[model_schema()]
#[derive(Deserialize, Serialize)]
pub struct GetVersionRequest {
    pub document_id: String,
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VersionResponse {
    pub content: String,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum DocumentError {
    NotFound,
}

#[service_schema(transports = ["amqp_rpc", "http_rest"])]
pub trait DocumentService<Ctx> {
    #[service_schema_op(http(
        method = "GET",
        path = "/documents/{document_id}",
        ok_status = 200,
        header_in("range" = byte_range),
        header_out("etag"),
        error_status(NotFound = 404),
    ))]
    async fn get_version(
        &self,
        ctx: &Ctx,
        req: GetVersionRequest,
        byte_range: Option<String>,
    ) -> Result<(VersionResponse, String), DocumentError>;

    /// One-way, so `header_in` is the only binding it can carry — `header_out` has no reply to
    /// ride in on a one-way operation. Its own message is the macro's own `TouchRequest`, declared
    /// from `document_id` the same way every operation with no named message gets one.
    #[service_schema_op(
        one_way,
        http(
            method = "POST",
            path = "/documents/{document_id}/touch",
            header_in("if-match" = expected_etag),
        )
    )]
    async fn touch(&self, ctx: &Ctx, document_id: String, expected_etag: Option<String>);
}

/// Records every `(document_id, byte_range)` pair `get_version` was called with, and every
/// `(document_id, expected_etag)` pair `touch` was, so a test can say the value a `header_in`
/// binding decoded is what the implementation actually received.
pub struct DocumentBackEnd {
    reached: Mutex<Vec<(String, Option<String>)>>,
    touched: Mutex<Vec<(String, Option<String>)>>,
}

impl DocumentService<()> for DocumentBackEnd {
    async fn get_version(
        &self,
        _ctx: &(),
        req: GetVersionRequest,
        byte_range: Option<String>,
    ) -> Result<(VersionResponse, String), DocumentError> {
        ready(()).await;
        self.reached
            .lock()
            .unwrap()
            .push((req.document_id.clone(), byte_range));
        if req.document_id == "missing" {
            return Err(DocumentError::NotFound);
        }
        Ok((
            VersionResponse {
                content: "hello".to_owned(),
            },
            "abc123".to_owned(),
        ))
    }

    async fn touch(&self, _ctx: &(), document_id: String, expected_etag: Option<String>) {
        ready(()).await;
        self.touched
            .lock()
            .unwrap()
            .push((document_id, expected_etag));
    }
}

impl DocumentBackEnd {
    fn new() -> Self {
        Self {
            reached: Mutex::new(Vec::new()),
            touched: Mutex::new(Vec::new()),
        }
    }

    fn reached(&self) -> Vec<(String, Option<String>)> {
        self.reached.lock().unwrap().clone()
    }

    fn touched(&self) -> Vec<(String, Option<String>)> {
        self.touched.lock().unwrap().clone()
    }
}

/// One of the two ways an arm answers, keeping the headers a bound reply carried beside its value
/// — the whole of what a `header_out` binding writes there.
#[derive(Clone, Debug)]
pub enum Settled {
    Fault(document_service_schema::ServiceFault),
    Sent {
        headers: Vec<(String, String)>,
        value: String,
    },
}

/// Records how each message was settled instead of publishing anything.
pub struct ProbeReply {
    settled: Mutex<Vec<Settled>>,
}

impl amqp_transport::Reply for ProbeReply {
    async fn fault(&self, fault: document_service_schema::ServiceFault) {
        ready(()).await;
        self.record(Settled::Fault(fault));
    }

    async fn send<T>(&self, value: T, headers: Vec<(String, String)>)
    where
        T: Serialize + Send,
    {
        ready(()).await;
        self.record(Settled::Sent {
            headers,
            value: serde_json::to_string(&value).unwrap(),
        });
    }
}

impl ProbeReply {
    fn new() -> Self {
        Self {
            settled: Mutex::new(Vec::new()),
        }
    }

    fn record(&self, what: Settled) {
        self.settled.lock().unwrap().push(what);
    }

    fn settled(&self) -> Vec<Settled> {
        self.settled.lock().unwrap().clone()
    }
}

/// One call this transport carried: the operation named, and the headers beside it.
type Call = (String, Vec<(String, String)>);

/// A transport that hands back one prepared reply and records the operation and the headers it was
/// asked to carry outbound.
pub struct ProbeTransport {
    calls: Mutex<Vec<Call>>,
}

impl amqp_client::Transport for ProbeTransport {
    async fn notify<T>(
        &self,
        operation: &str,
        _payload: T,
        headers: Vec<(String, String)>,
    ) -> Result<(), String>
    where
        T: Serialize + Send,
    {
        ready(()).await;
        self.calls
            .lock()
            .unwrap()
            .push((operation.to_owned(), headers));
        Ok(())
    }

    async fn request<T>(
        &self,
        operation: &str,
        _payload: T,
        headers: Vec<(String, String)>,
    ) -> Result<(Vec<u8>, Vec<(String, String)>), String>
    where
        T: Serialize + Send,
    {
        ready(()).await;
        self.calls
            .lock()
            .unwrap()
            .push((operation.to_owned(), headers));
        let body = serde_json::to_vec(&serde_json::json!({
            "ok": true,
            "value": { "content": "hello" },
        }))
        .unwrap();
        Ok((
            body,
            vec![(
                "etag".to_owned(),
                serde_json::to_string(&"abc123".to_owned()).unwrap(),
            )],
        ))
    }
}

impl ProbeTransport {
    fn calls(&self) -> Vec<Call> {
        self.calls.lock().unwrap().clone()
    }

    fn new() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
        }
    }
}

/// The one fault a settlement list holds, or nothing when it holds something else.
fn only_fault(settled: &[Settled]) -> Option<&document_service_schema::ServiceFault> {
    match settled {
        [Settled::Fault(reported)] => Some(reported),
        _ => None,
    }
}

/// The one value and headers a settlement list holds, or nothing when it holds something else.
fn only_sent(settled: &[Settled]) -> Option<(String, Vec<(String, String)>)> {
    match settled {
        [Settled::Sent { value, headers }] => Some((value.clone(), headers.clone())),
        _ => None,
    }
}

/// The probe never suspends, so one poll answers it; `None` says an assumption about the bodies
/// above stopped holding rather than that the runtime is missing.
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

#[test]
fn the_dispatcher_decodes_a_claimed_header_before_calling_the_implementation_and_writes_one_back() {
    let service = DocumentBackEnd::new();
    let reply = ProbeReply::new();
    let range = serde_json::to_string(&Some("bytes=0-10".to_owned())).unwrap();
    poll_once(amqp_transport::dispatch(
        &service,
        &(),
        &amqp_transport::IncomingMessage::new(
            "get-version".to_owned(),
            br#"{"document_id":"doc-1"}"#.to_vec(),
            vec![("range".to_owned(), range)],
        ),
        &reply,
    ))
    .unwrap();
    assert_eq!(
        service.reached(),
        vec![("doc-1".to_owned(), Some("bytes=0-10".to_owned()))],
        "the header_in binding decoded the claimed header into the operation's own argument"
    );
    let settled = reply.settled();
    let (value, headers) = only_sent(&settled).unwrap();
    assert_eq!(value, r#"{"ok":true,"value":{"content":"hello"}}"#);
    assert_eq!(
        headers,
        vec![("etag".to_owned(), "\"abc123\"".to_owned())],
        "the header_out binding wrote the second tuple element into the reply's own headers, \
         JSON-encoded, rather than into the response body"
    );
}

#[test]
fn a_header_in_binding_nothing_carried_decodes_as_the_argument_s_own_absent_value() {
    let service = DocumentBackEnd::new();
    let reply = ProbeReply::new();
    poll_once(amqp_transport::dispatch(
        &service,
        &(),
        &amqp_transport::IncomingMessage::new(
            "get-version".to_owned(),
            br#"{"document_id":"doc-2"}"#.to_vec(),
            Vec::new(),
        ),
        &reply,
    ))
    .unwrap();
    assert_eq!(
        service.reached(),
        vec![("doc-2".to_owned(), None)],
        "a header nothing carried decodes as JSON null, which an Option<String> argument admits \
         as None"
    );
}

#[test]
fn a_header_in_value_that_will_not_decode_fails_before_the_implementation_is_called() {
    let service = DocumentBackEnd::new();
    let reply = ProbeReply::new();
    poll_once(amqp_transport::dispatch(
        &service,
        &(),
        &amqp_transport::IncomingMessage::new(
            "get-version".to_owned(),
            br#"{"document_id":"doc-3"}"#.to_vec(),
            vec![("range".to_owned(), "not json".to_owned())],
        ),
        &reply,
    ))
    .unwrap();
    assert!(
        service.reached().is_empty(),
        "an undecodable header must not reach the implementation"
    );
    let settled = reply.settled();
    let fault = only_fault(&settled).unwrap();
    assert_eq!(
        fault.kind(),
        document_service_schema::ServiceFaultKind::FailedValidation
    );
    assert_eq!(
        fault.field(),
        Some("range"),
        "the caller has to be told which header it got wrong"
    );
}

#[test]
fn the_operations_own_error_carries_no_header_out_value_and_still_answers_normally() {
    let service = DocumentBackEnd::new();
    let reply = ProbeReply::new();
    poll_once(amqp_transport::dispatch(
        &service,
        &(),
        &amqp_transport::IncomingMessage::new(
            "get-version".to_owned(),
            br#"{"document_id":"missing"}"#.to_vec(),
            Vec::new(),
        ),
        &reply,
    ))
    .unwrap();
    let settled = reply.settled();
    let (value, headers) = only_sent(&settled).unwrap();
    assert_eq!(value, r#"{"error":{"errorCode":"not-found"},"ok":false}"#);
    assert!(
        headers.is_empty(),
        "a declared error carries no header_out value. Got: {headers:?}"
    );
}

#[test]
fn the_client_encodes_a_claimed_header_outbound_and_decodes_a_declared_one_from_the_reply() {
    let transport = ProbeTransport::new();
    let client = amqp_client::DocumentServiceClient::new(transport);
    let answered = poll_once(client.get_version(
        GetVersionRequest {
            document_id: "doc-1".to_owned(),
        },
        Some("bytes=0-10".to_owned()),
    ))
    .unwrap()
    .unwrap();
    assert_eq!(
        answered,
        (
            VersionResponse {
                content: "hello".to_owned(),
            },
            "abc123".to_owned(),
        ),
        "the response rides in the payload and the header_out value is rejoined from the reply's \
         own headers"
    );
    assert_eq!(
        client.transport().calls(),
        vec![(
            "get-version".to_owned(),
            vec![(
                "range".to_owned(),
                serde_json::to_string(&Some("bytes=0-10".to_owned())).unwrap(),
            )],
        )],
        "the header_in argument was JSON-encoded into the outbound headers list"
    );
}

#[test]
fn a_one_way_operation_still_decodes_its_claimed_header_before_calling_the_implementation() {
    let service = DocumentBackEnd::new();
    let reply = ProbeReply::new();
    let if_match = serde_json::to_string(&Some("etag-xyz".to_owned())).unwrap();
    poll_once(amqp_transport::dispatch(
        &service,
        &(),
        &amqp_transport::IncomingMessage::new(
            // `touch`'s one carried argument, `document_id`, already is the whole message: with
            // `expected_etag` claimed by `header_in`, one argument is left, and `OperationInputs`
            // uses that argument's own type as the message rather than declaring a struct for it.
            "touch".to_owned(),
            br#""doc-9""#.to_vec(),
            vec![("if-match".to_owned(), if_match)],
        ),
        &reply,
    ))
    .unwrap();
    assert_eq!(
        service.touched(),
        vec![("doc-9".to_owned(), Some("etag-xyz".to_owned()))],
        "a one-way operation still reads its header_in binding before running"
    );
    assert!(
        reply.settled().is_empty(),
        "a one-way operation answers nothing, header_in included"
    );
}

#[test]
fn a_one_way_client_method_encodes_its_claimed_header_into_the_notify_call() {
    let transport = ProbeTransport::new();
    let client = amqp_client::DocumentServiceClient::new(transport);
    poll_once(client.touch("doc-9".to_owned(), Some("etag-xyz".to_owned())))
        .unwrap()
        .unwrap();
    assert_eq!(
        client.transport().calls(),
        vec![(
            "touch".to_owned(),
            vec![(
                "if-match".to_owned(),
                serde_json::to_string(&Some("etag-xyz".to_owned())).unwrap(),
            )],
        )],
        "the header_in argument was JSON-encoded into the outbound headers list on the notify \
         path too"
    );
}
