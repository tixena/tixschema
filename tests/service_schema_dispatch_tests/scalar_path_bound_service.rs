//! A single scalar argument that already is the whole message, bound whole by the path's one
//! placeholder of its own name - driven through the `http_rest` dispatcher by hand, no server.

#![cfg(feature = "serde")]

use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context as PollContext, Poll, Waker};
use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

use crate::scalar_path_bound_http_rest_transport::{
    self, DefaultFaultHandler, FaultHandler, IncomingRequest, OutgoingResponse, Route, dispatch,
};
use crate::scalar_path_bound_service_schema::ServiceFault;

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DocumentSummary {
    pub id: String,
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum GetDocumentError {
    NotFound,
}

/// The one operation's one argument already is the message: a scalar the path's one placeholder
/// binds whole, under its own name.
#[service_schema(transports = ["http_rest"])]
pub trait ScalarPathBoundService<Ctx> {
    #[service_schema_op(http(
        method = "GET",
        path = "/documents/{id}",
        error_status(NotFound = 404),
    ))]
    async fn get(&self, ctx: &Ctx, id: String) -> Result<DocumentSummary, GetDocumentError>;
}

pub struct ScalarBackEnd;

impl ScalarPathBoundService<()> for ScalarBackEnd {
    async fn get(&self, _ctx: &(), id: String) -> Result<DocumentSummary, GetDocumentError> {
        ready(()).await;
        if id == "missing" {
            return Err(GetDocumentError::NotFound);
        }
        Ok(DocumentSummary { id })
    }
}

/// An owner-installed `FaultHandler`, exercising `OutgoingResponse::new` and its own `headers()`
/// accessor directly - the same construction path a JSON service's own override uses.
struct RecordingFaultHandler;

impl FaultHandler for RecordingFaultHandler {
    fn on_fault(&self, fault: &ServiceFault) -> OutgoingResponse {
        OutgoingResponse::new(
            499,
            vec![("x-fault-kind".to_owned(), format!("{}", fault.kind()))],
            format!("handled: {}", fault.detail()).into_bytes(),
        )
    }
}

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

fn dispatched(path: &str) -> (u16, Vec<u8>) {
    let request = IncomingRequest::new(
        "GET".to_owned(),
        path.to_owned(),
        String::new(),
        Vec::new(),
        Vec::new(),
    );
    let response = poll_once(dispatch(
        &ScalarBackEnd,
        &(),
        &request,
        &DefaultFaultHandler,
    ))
    .unwrap();
    (response.status(), response.body().to_vec())
}

/// The reproduction: a single scalar argument bound whole by the path's one placeholder of its
/// own name compiles, and dispatches with its value read from the placeholder alone - never a
/// body, never a query string.
#[test]
fn a_scalar_argument_bound_whole_by_its_own_placeholder_dispatches() {
    let (status, body) = dispatched("/documents/abc");
    assert_eq!(status, 200);
    assert_eq!(body, br#"{"id":"abc"}"#);
}

#[test]
fn a_scalar_argument_bound_whole_still_answers_its_declared_error() {
    let (status, body) = dispatched("/documents/missing");
    assert_eq!(status, 404);
    assert_eq!(body, br#"{"errorCode":"not-found"}"#);
}

/// The route table an adapter iterates to register a handler: one row, no query it could bind.
#[test]
fn the_route_table_lists_the_one_route() {
    let routes = scalar_path_bound_http_rest_transport::ROUTES;
    let paths: Vec<&str> = routes.iter().map(Route::path).collect();
    assert_eq!(routes.len(), 1, "got: {paths:?}");
    let route = &routes[0];
    assert_eq!(route.method(), "GET");
    assert_eq!(route.path(), "/documents/{id}");
    assert_eq!(route.operation(), "get");
    assert_eq!(route.ok_status(), 200);
    assert_eq!(route.error_statuses(), &[404]);
}

/// `IncomingRequest` reads back everything it was built with, exercised here for this
/// dispatcher's own expansion.
#[test]
fn an_incoming_request_reads_back_its_body_headers_and_query() {
    let request = IncomingRequest::new(
        "GET".to_owned(),
        "/documents/abc".to_owned(),
        "unused=1".to_owned(),
        vec![("x-trace".to_owned(), "abc".to_owned())],
        b"ignored".to_vec(),
    );
    assert_eq!(request.body(), b"ignored");
    assert_eq!(request.query(), "unused=1");
    assert_eq!(request.header("x-trace"), Some("abc"));
    assert_eq!(
        request.headers(),
        &[("x-trace".to_owned(), "abc".to_owned())]
    );
}

/// An owner-installed `FaultHandler` still builds an `OutgoingResponse` by hand on this
/// dispatcher, exercising `OutgoingResponse::new` and its `headers()` accessor directly.
#[test]
fn an_installed_fault_handler_still_builds_an_outgoing_response_by_hand() {
    let request = IncomingRequest::new(
        "GET".to_owned(),
        "/nowhere".to_owned(),
        String::new(),
        Vec::new(),
        Vec::new(),
    );
    let response = poll_once(dispatch(
        &ScalarBackEnd,
        &(),
        &request,
        &RecordingFaultHandler,
    ))
    .unwrap();
    assert_eq!(response.status(), 499);
    assert_eq!(
        response.headers(),
        &[("x-fault-kind".to_owned(), "unknown operation".to_owned())]
    );
    assert_eq!(
        response.body(),
        b"handled: the service answers to no operation by that name"
    );
}
