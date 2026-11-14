//! Bodied operations whose message reads fields out of the JSON body: one no placeholder binds, so
//! nothing is inserted into the parsed body, beside two a placeholder binds. Driven through the
//! `http_rest` dispatcher by hand, no server. The placed dispatcher is the proof that the first
//! one's arm builds with no `unused_mut` under `-D warnings`.

#![cfg(feature = "serde")]

use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context as PollContext, Poll, Waker};
use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

use crate::bodied_fields_http_rest_transport::{
    self, DefaultFaultHandler, FaultHandler, IncomingRequest, OutgoingResponse, Route, dispatch,
};
use crate::bodied_fields_service_schema::ServiceFault;

/// The document every operation here answers with.
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BodiedDocument {
    pub id: String,
    pub tags: Vec<String>,
    pub title: String,
}

/// The error `create_document` declares.
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum BodiedDocumentError {
    TitleTaken,
}

/// A message the author declared, whose `document_id` the path binds.
#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct ReplaceDocumentRequest {
    pub document_id: String,
    pub title: String,
}

/// `create_document` leaves every field to the parsed body; `rename_document` and
/// `replace_document` insert the path's `document_id` into it.
#[service_schema(transports = ["http_rest"])]
pub trait BodiedFieldsService<Ctx> {
    #[service_schema_op(http(
        method = "POST",
        path = "/documents",
        error_status(TitleTaken = 409)
    ))]
    async fn create_document(
        &self,
        ctx: &Ctx,
        title: String,
        tags: Vec<String>,
    ) -> Result<BodiedDocument, BodiedDocumentError>;

    #[service_schema_op(http(
        method = "PATCH",
        path = "/documents/{document_id}",
        error_status(TitleTaken = 409)
    ))]
    async fn rename_document(
        &self,
        ctx: &Ctx,
        document_id: String,
        title: String,
    ) -> Result<BodiedDocument, BodiedDocumentError>;

    #[service_schema_op(http(
        method = "PUT",
        path = "/documents/{document_id}",
        error_status(TitleTaken = 409)
    ))]
    async fn replace_document(
        &self,
        ctx: &Ctx,
        req: ReplaceDocumentRequest,
    ) -> Result<BodiedDocument, BodiedDocumentError>;
}

/// The implementation of `BodiedFieldsService` these tests dispatch to.
pub struct BodiedBackEnd;

impl BodiedFieldsService<()> for BodiedBackEnd {
    async fn create_document(
        &self,
        _ctx: &(),
        title: String,
        tags: Vec<String>,
    ) -> Result<BodiedDocument, BodiedDocumentError> {
        ready(()).await;
        if title == "taken" {
            return Err(BodiedDocumentError::TitleTaken);
        }
        Ok(BodiedDocumentDraft {
            id: "doc-1".to_owned(),
            tags,
            title,
        }
        .into())
    }

    async fn rename_document(
        &self,
        _ctx: &(),
        document_id: String,
        title: String,
    ) -> Result<BodiedDocument, BodiedDocumentError> {
        ready(()).await;
        Ok(BodiedDocumentDraft {
            id: document_id,
            tags: Vec::new(),
            title,
        }
        .into())
    }

    async fn replace_document(
        &self,
        _ctx: &(),
        req: ReplaceDocumentRequest,
    ) -> Result<BodiedDocument, BodiedDocumentError> {
        ready(()).await;
        Ok(BodiedDocumentDraft {
            id: req.document_id.clone(),
            tags: Vec::new(),
            title: req.title.clone(),
        }
        .into())
    }
}

/// An owner-installed `FaultHandler`, using `OutgoingResponse::new` and `headers()` directly.
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

fn dispatched(method: &str, path: &str, body: &[u8]) -> (u16, Vec<u8>) {
    let request = IncomingRequest::new(
        method.to_owned(),
        path.to_owned(),
        String::new(),
        Vec::new(),
        body.to_vec(),
    );
    let response = poll_once(dispatch(
        &BodiedBackEnd,
        &(),
        &request,
        &DefaultFaultHandler,
    ))
    .unwrap();
    (response.status(), response.body().to_vec())
}

#[test]
fn a_message_no_placeholder_binds_reads_every_field_from_the_body() {
    let (status, body) = dispatched(
        "POST",
        "/documents",
        br#"{"title":"Board meeting","tags":["minutes"]}"#,
    );
    assert_eq!(status, 200);
    assert_eq!(
        body,
        br#"{"id":"doc-1","tags":["minutes"],"title":"Board meeting"}"#
    );
}

#[test]
fn a_message_no_placeholder_binds_still_answers_its_declared_error() {
    let (status, body) = dispatched("POST", "/documents", br#"{"title":"taken","tags":[]}"#);
    assert_eq!(status, 409);
    assert_eq!(body, br#"{"errorCode":"title-taken"}"#);
}

/// A body that is not a JSON object leaves the message with no field, which the read refuses.
#[test]
fn a_message_no_placeholder_binds_refuses_a_body_that_is_not_an_object() {
    for unreadable in [&b"not json"[..], &b"[1, 2]"[..]] {
        let (status, body) = dispatched("POST", "/documents", unreadable);
        assert_eq!(status, 400);
        let said = String::from_utf8(body).unwrap();
        assert!(said.contains("missing field"), "got: {said}");
    }
}

/// The path's value is inserted over the one the body carries, under the same wire key.
#[test]
fn a_generated_message_takes_the_field_a_placeholder_binds_from_the_path() {
    let (status, body) = dispatched(
        "PATCH",
        "/documents/d1",
        br#"{"documentId":"elsewhere","title":"Minutes"}"#,
    );
    assert_eq!(status, 200);
    assert_eq!(body, br#"{"id":"d1","tags":[],"title":"Minutes"}"#);
}

#[test]
fn a_declared_message_takes_the_field_a_placeholder_binds_from_the_path() {
    let (status, body) = dispatched(
        "PUT",
        "/documents/d1",
        br#"{"document_id":"elsewhere","title":"Minutes"}"#,
    );
    assert_eq!(status, 200);
    assert_eq!(body, br#"{"id":"d1","tags":[],"title":"Minutes"}"#);
}

#[test]
fn the_route_table_lists_every_route() {
    let routes = bodied_fields_http_rest_transport::ROUTES;
    let listed: Vec<(&str, &str, &str, u16, &[u16])> = routes
        .iter()
        .map(|route: &Route| {
            (
                route.method(),
                route.path(),
                route.operation(),
                route.ok_status(),
                route.error_statuses(),
            )
        })
        .collect();
    let conflict: &[u16] = &[409];
    assert_eq!(
        listed,
        vec![
            ("POST", "/documents", "create-document", 200, conflict),
            (
                "PATCH",
                "/documents/{document_id}",
                "rename-document",
                200,
                conflict
            ),
            (
                "PUT",
                "/documents/{document_id}",
                "replace-document",
                200,
                conflict
            ),
        ]
    );
}

#[test]
fn an_incoming_request_reads_back_its_body_headers_and_query() {
    let request = IncomingRequest::new(
        "POST".to_owned(),
        "/documents".to_owned(),
        "unused=1".to_owned(),
        vec![("x-trace".to_owned(), "abc".to_owned())],
        b"{}".to_vec(),
    );
    assert_eq!(request.body(), b"{}");
    assert_eq!(request.query(), "unused=1");
    assert_eq!(request.header("x-trace"), Some("abc"));
    assert_eq!(
        request.headers(),
        &[("x-trace".to_owned(), "abc".to_owned())]
    );
}

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
        &BodiedBackEnd,
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
