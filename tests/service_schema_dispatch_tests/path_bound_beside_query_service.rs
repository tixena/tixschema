//! The same fully path-bound `GET` as `solo_path_bound_service`, this time beside a second
//! operation whose `limit` the path leaves unbound - so this service's dispatcher does reach for
//! `parse_query`, just not in `get_document`'s own arm.

#![cfg(feature = "serde")]

use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context as PollContext, Poll, Waker};
use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

use crate::path_bound_beside_query_http_rest_transport::{
    self, DefaultFaultHandler, FaultHandler, IncomingRequest, OutgoingResponse, Route, dispatch,
};
use crate::path_bound_beside_query_service_schema::ServiceFault;

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

#[service_schema(transports = ["http_rest"])]
pub trait PathBoundBesideQueryService<Ctx> {
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

    #[service_schema_op(http(method = "GET", path = "/orgs/{org}/documents"))]
    async fn list_documents(
        &self,
        ctx: &Ctx,
        org: String,
        limit: Option<u32>,
    ) -> Result<PathBoundDocument, PathBoundError>;
}

pub struct BesideQueryBackEnd;

impl PathBoundBesideQueryService<()> for BesideQueryBackEnd {
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

    async fn list_documents(
        &self,
        _ctx: &(),
        org: String,
        limit: Option<u32>,
    ) -> Result<PathBoundDocument, PathBoundError> {
        ready(()).await;
        Ok(PathBoundDocument {
            title: format!("{org} limit={limit:?}"),
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

fn dispatched(path: &str, query: &str) -> (u16, Vec<u8>) {
    let request = IncomingRequest::new(
        "GET".to_owned(),
        path.to_owned(),
        query.to_owned(),
        Vec::new(),
        Vec::new(),
    );
    let response = poll_once(dispatch(
        &BesideQueryBackEnd,
        &(),
        &request,
        &DefaultFaultHandler,
    ))
    .unwrap();
    (response.status(), response.body().to_vec())
}

/// The same fully path-bound operation beside a query-reading one: both dispatch, with no
/// `unused variable` warning from either arm.
#[test]
fn a_path_bound_operation_beside_a_query_reader_still_dispatches_from_the_path_alone() {
    let (status, body) = dispatched("/orgs/acme/documents/d1", "");
    assert_eq!(status, 200);
    assert_eq!(body, br#"{"title":"acme/d1"}"#);
}

#[test]
fn the_query_reader_beside_it_still_reads_its_query() {
    let (status, body) = dispatched("/orgs/acme/documents", "limit=5");
    assert_eq!(status, 200);
    assert_eq!(body, br#"{"title":"acme limit=Some(5)"}"#);
}

/// The route table an adapter iterates to register a handler: one row per operation.
#[test]
fn the_route_table_lists_both_routes() {
    let routes = path_bound_beside_query_http_rest_transport::ROUTES;
    let paths: Vec<&str> = routes.iter().map(Route::path).collect();
    assert_eq!(routes.len(), 2, "got: {paths:?}");
    assert_eq!(routes[0].method(), "GET");
    assert_eq!(routes[0].path(), "/orgs/{org}/documents/{id}");
    assert_eq!(routes[0].operation(), "get-document");
    assert_eq!(routes[0].ok_status(), 200);
    assert_eq!(routes[0].error_statuses(), &[404]);
    assert_eq!(routes[1].method(), "GET");
    assert_eq!(routes[1].path(), "/orgs/{org}/documents");
    assert_eq!(routes[1].operation(), "list-documents");
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
        &BesideQueryBackEnd,
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
