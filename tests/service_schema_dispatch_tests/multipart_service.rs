//! A `body = "multipart"` operation driven through the `http_rest` dispatcher by hand — no server
//! — round-tripping scalar fields decoded into the generated message and a file part handed
//! through as a `BodySource` handle, undecoded, before `validate()` runs.

#![cfg(feature = "serde")]

use crate::multipart_http_rest_transport;
use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context as PollContext, Poll, Waker};
use serde::{Deserialize, Serialize};
use std::io::{self, Read};
use std::sync::Mutex;
use tixschema::{model_schema, service_schema};

/// What one call recorded: the path-bound field, the scalar parts, and the file part's whole
/// content, drained through `BodySource::pull`.
type Reached = (String, String, Option<String>, Vec<u8>);

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct UploadResponse {
    pub document_id: String,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum UploadError {
    TooLarge,
}

#[service_schema(transports = ["http_rest"])]
pub trait UploadService<Ctx> {
    #[service_schema_op(http(
        method = "POST",
        path = "/folders/{folder_id}/documents",
        body = "multipart",
        part("file" = attachment),
        error_status(TooLarge = 413),
    ))]
    async fn upload_document(
        &self,
        ctx: &Ctx,
        folder_id: String,
        title: String,
        description: Option<String>,
        attachment: Box<dyn upload_service_schema::BodySource + Send>,
    ) -> Result<UploadResponse, UploadError>;
}

/// A fixed-size in-memory reader, standing in for a real chunked upload body.
struct ByteSource {
    remaining: Vec<u8>,
}

impl Read for ByteSource {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let take = buf.len().min(self.remaining.len());
        let rest = self.remaining.split_off(take);
        buf[..take].copy_from_slice(&self.remaining);
        self.remaining = rest;
        Ok(take)
    }
}

/// Records what the implementation actually received.
pub struct UploadBackEnd {
    reached: Mutex<Vec<Reached>>,
}

impl UploadService<()> for UploadBackEnd {
    async fn upload_document(
        &self,
        _ctx: &(),
        folder_id: String,
        title: String,
        description: Option<String>,
        mut attachment: Box<dyn upload_service_schema::BodySource + Send>,
    ) -> Result<UploadResponse, UploadError> {
        ready(()).await;
        let mut drained = Vec::new();
        let mut buf = [0_u8; 64];
        loop {
            let read = attachment.pull(&mut buf).unwrap();
            if read == 0 {
                break;
            }
            drained.extend_from_slice(&buf[..read]);
        }
        self.reached
            .lock()
            .unwrap()
            .push((folder_id, title.clone(), description, drained));
        if title == "toolarge" {
            return Err(UploadError::TooLarge);
        }
        Ok(UploadResponse {
            document_id: format!("doc-{title}"),
        })
    }
}

impl UploadBackEnd {
    fn new() -> Self {
        Self {
            reached: Mutex::new(Vec::new()),
        }
    }

    fn reached(&self) -> Vec<Reached> {
        self.reached.lock().unwrap().clone()
    }
}

/// An owner-installed `FaultHandler`, exercising `OutgoingResponse::new` and its own `headers()`
/// accessor directly - the same construction path a JSON or bytes service's own override uses,
/// unaffected by the extra `parts` argument a multipart operation's own dispatcher takes.
struct RecordingFaultHandler;

impl multipart_http_rest_transport::FaultHandler for RecordingFaultHandler {
    fn on_fault(
        &self,
        fault: &upload_service_schema::ServiceFault,
    ) -> multipart_http_rest_transport::OutgoingResponse {
        multipart_http_rest_transport::OutgoingResponse::new(
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

#[test]
fn a_multipart_operation_decodes_scalar_fields_and_hands_the_file_part_through() {
    let service = UploadBackEnd::new();
    let request = multipart_http_rest_transport::IncomingRequest::new(
        "POST".to_owned(),
        "/folders/acme/documents".to_owned(),
        String::new(),
        Vec::new(),
        Vec::new(),
    );
    let parts = vec![
        (
            "title".to_owned(),
            multipart_http_rest_transport::IncomingPart::Text("quarterly-report".to_owned()),
        ),
        (
            "description".to_owned(),
            multipart_http_rest_transport::IncomingPart::Text("Q3 numbers".to_owned()),
        ),
        (
            "file".to_owned(),
            multipart_http_rest_transport::IncomingPart::File(Box::new(ByteSource {
                remaining: b"file-bytes".to_vec(),
            })),
        ),
    ];
    let response = poll_once(multipart_http_rest_transport::dispatch(
        &service,
        &(),
        &request,
        parts,
        &multipart_http_rest_transport::DefaultFaultHandler,
    ))
    .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(
        response.body(),
        br#"{"document_id":"doc-quarterly-report"}"#
    );
    assert_eq!(
        service.reached(),
        vec![(
            "acme".to_owned(),
            "quarterly-report".to_owned(),
            Some("Q3 numbers".to_owned()),
            b"file-bytes".to_vec()
        )]
    );
}

#[test]
fn an_absent_optional_scalar_part_decodes_as_none() {
    let service = UploadBackEnd::new();
    let request = multipart_http_rest_transport::IncomingRequest::new(
        "POST".to_owned(),
        "/folders/acme/documents".to_owned(),
        String::new(),
        Vec::new(),
        Vec::new(),
    );
    let parts = vec![
        (
            "title".to_owned(),
            multipart_http_rest_transport::IncomingPart::Text("no-description".to_owned()),
        ),
        (
            "file".to_owned(),
            multipart_http_rest_transport::IncomingPart::File(Box::new(ByteSource {
                remaining: b"bytes".to_vec(),
            })),
        ),
    ];
    poll_once(multipart_http_rest_transport::dispatch(
        &service,
        &(),
        &request,
        parts,
        &multipart_http_rest_transport::DefaultFaultHandler,
    ))
    .unwrap();
    assert_eq!(
        service.reached(),
        vec![(
            "acme".to_owned(),
            "no-description".to_owned(),
            None,
            b"bytes".to_vec()
        )]
    );
}

#[test]
fn a_missing_required_file_part_answers_a_fault_rather_than_reaching_the_implementation() {
    let service = UploadBackEnd::new();
    let request = multipart_http_rest_transport::IncomingRequest::new(
        "POST".to_owned(),
        "/folders/acme/documents".to_owned(),
        String::new(),
        Vec::new(),
        Vec::new(),
    );
    let parts = vec![(
        "title".to_owned(),
        multipart_http_rest_transport::IncomingPart::Text("no-file".to_owned()),
    )];
    let response = poll_once(multipart_http_rest_transport::dispatch(
        &service,
        &(),
        &request,
        parts,
        &multipart_http_rest_transport::DefaultFaultHandler,
    ))
    .unwrap();
    assert!(
        service.reached().is_empty(),
        "the handler must not run without its file part"
    );
    assert_eq!(response.status(), 400);
    let fault: serde_json::Value = serde_json::from_slice(response.body()).unwrap();
    assert_eq!(fault["kind"], "failed-validation");
    assert_eq!(fault["field"], "file");
}

#[test]
fn a_declared_error_still_answers_its_mapped_status() {
    let service = UploadBackEnd::new();
    let request = multipart_http_rest_transport::IncomingRequest::new(
        "POST".to_owned(),
        "/folders/acme/documents".to_owned(),
        String::new(),
        Vec::new(),
        Vec::new(),
    );
    let parts = vec![
        (
            "title".to_owned(),
            multipart_http_rest_transport::IncomingPart::Text("toolarge".to_owned()),
        ),
        (
            "file".to_owned(),
            multipart_http_rest_transport::IncomingPart::File(Box::new(ByteSource {
                remaining: b"bytes".to_vec(),
            })),
        ),
    ];
    let response = poll_once(multipart_http_rest_transport::dispatch(
        &service,
        &(),
        &request,
        parts,
        &multipart_http_rest_transport::DefaultFaultHandler,
    ))
    .unwrap();
    assert_eq!(response.status(), 413);
    assert_eq!(response.body(), br#"{"errorCode":"too-large"}"#);
}

/// `IncomingRequest` reads back everything it was built with, exercised here for a multipart
/// operation's own dispatcher expansion - the same accessors every other body kind already
/// reaches for.
#[test]
fn an_incoming_request_reads_back_its_body_headers_and_query() {
    let request = multipart_http_rest_transport::IncomingRequest::new(
        "POST".to_owned(),
        "/folders/acme/documents".to_owned(),
        "unused=1".to_owned(),
        vec![("x-trace".to_owned(), "abc".to_owned())],
        b"ignored".to_vec(),
    );
    assert_eq!(request.body(), b"ignored");
    assert_eq!(request.query(), "unused=1");
    assert_eq!(
        request.headers(),
        &[("x-trace".to_owned(), "abc".to_owned())]
    );
    assert_eq!(request.header("x-trace"), Some("abc"));
}

/// The route table an adapter iterates to register a handler: one row for the one multipart
/// operation, its statuses included.
#[test]
fn the_route_table_lists_the_one_multipart_route() {
    let routes = multipart_http_rest_transport::ROUTES;
    assert_eq!(
        routes.len(),
        1,
        "got: {:?}",
        routes
            .iter()
            .map(multipart_http_rest_transport::Route::path)
            .collect::<Vec<_>>()
    );
    assert_eq!(routes[0].method(), "POST");
    assert_eq!(routes[0].path(), "/folders/{folder_id}/documents");
    assert_eq!(routes[0].operation(), "upload-document");
    assert_eq!(routes[0].ok_status(), 200);
    assert_eq!(routes[0].error_statuses(), &[413]);
}

/// An owner-installed `FaultHandler` still reaches `OutgoingResponse::new` and its own `headers()`
/// on a multipart service's dispatcher, unaffected by the extra `parts` argument `dispatch` takes.
#[test]
fn an_installed_fault_handler_still_builds_an_outgoing_response_by_hand() {
    let request = multipart_http_rest_transport::IncomingRequest::new(
        "GET".to_owned(),
        "/nowhere".to_owned(),
        String::new(),
        Vec::new(),
        Vec::new(),
    );
    let response = poll_once(multipart_http_rest_transport::dispatch(
        &UploadBackEnd::new(),
        &(),
        &request,
        Vec::new(),
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
