//! A bodyless `GET` whose every argument the path binds, and no other operation in the service to
//! read a query either - driven through the `http_rest` dispatcher by hand, no server.

#![cfg(feature = "serde")]

use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context as PollContext, Poll, Waker};
use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

use crate::solo_path_bound_http_rest_transport::{
    self, DefaultFaultHandler, FaultHandler, IncomingRequest, OutgoingResponse, Route, dispatch,
};
use crate::solo_path_bound_service_schema::ServiceFault;

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PathBoundDocument {
    pub title: String,
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum PathBoundError {
    NotFound,
}

/// One operation, both arguments bound by the path - nothing left for the dispatcher to read off
/// the query string, and no other operation in the service to read one either.
#[service_schema(transports = ["http_rest"])]
pub trait SoloPathBoundService<Ctx> {
    #[service_schema_op(http(
        method = "GET",
        path = "/orgs/{org}/documents/{id}",
        error_status(NotFound = 404),
    ))]
    async fn get_document(
        &self,
        ctx: &Ctx,
        org: String,
        id: String,
    ) -> Result<PathBoundDocument, PathBoundError>;
}

pub struct SoloBackEnd;

impl SoloPathBoundService<()> for SoloBackEnd {
    async fn get_document(
        &self,
        _ctx: &(),
        org: String,
        id: String,
    ) -> Result<PathBoundDocument, PathBoundError> {
        ready(()).await;
        if id == "missing" {
            return Err(PathBoundError::NotFound);
        }
        Ok(PathBoundDocument {
            title: format!("{org}/{id}"),
        })
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
    let response = poll_once(dispatch(&SoloBackEnd, &(), &request, &DefaultFaultHandler)).unwrap();
    (response.status(), response.body().to_vec())
}

/// The reproduction: a service whose only operation binds every argument in the path compiles
/// with no warning, and dispatches with each argument decoded from its placeholder.
#[test]
fn a_solo_path_bound_operation_dispatches_with_each_argument_from_its_placeholder() {
    let (status, body) = dispatched("/orgs/acme/documents/d1");
    assert_eq!(status, 200);
    assert_eq!(body, br#"{"title":"acme/d1"}"#);
}

#[test]
fn a_solo_path_bound_operation_still_answers_its_declared_error() {
    let (status, body) = dispatched("/orgs/acme/documents/missing");
    assert_eq!(status, 404);
    assert_eq!(body, br#"{"errorCode":"not-found"}"#);
}

/// The route table an adapter iterates to register a handler: one row, no query it could bind.
#[test]
fn the_route_table_lists_the_one_route() {
    let routes = solo_path_bound_http_rest_transport::ROUTES;
    let paths: Vec<&str> = routes.iter().map(Route::path).collect();
    assert_eq!(routes.len(), 1, "got: {paths:?}");
    let route = &routes[0];
    assert_eq!(route.method(), "GET");
    assert_eq!(route.path(), "/orgs/{org}/documents/{id}");
    assert_eq!(route.operation(), "get-document");
    assert_eq!(route.ok_status(), 200);
    assert_eq!(route.error_statuses(), &[404]);
}

/// `IncomingRequest` reads back everything it was built with, exercised here for this
/// dispatcher's own expansion.
#[test]
fn an_incoming_request_reads_back_its_body_headers_and_query() {
    let request = IncomingRequest::new(
        "GET".to_owned(),
        "/orgs/acme/documents/d1".to_owned(),
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
        &SoloBackEnd,
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
