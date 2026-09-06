//! A `body = "multipart"` operation called through the `http_rest` client with no dispatcher in
//! sight - a hand-written `Transport` recording the `parts` the client built from the message plus
//! the file handle the caller passed in, and answering with a JSON body to decode back.

#![cfg(feature = "serde")]

use crate::multipart_http_rest_client;
use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context as PollContext, Poll, Waker};
use serde::{Deserialize, Serialize};
use std::io::{self, Read};
use std::sync::Mutex;
use tixschema::{model_schema, service_schema};

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct UploadResponse {
    pub document_id: String,
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum UploadError {
    TooLarge,
}

#[service_schema(transports = ["http_rest"])]
pub trait UploadClientService<Ctx> {
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
        attachment: Box<dyn upload_client_service_schema::BodySource + Send>,
    ) -> Result<UploadResponse, UploadError>;
}

/// A fixed-size in-memory reader, standing in for a real file handle.
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

/// One outgoing multipart part the seam recorded, read back through `pull()` where it is a file -
/// there being no other way to compare a boxed `BodySource` for equality.
#[derive(Clone, Debug, Eq, PartialEq)]
enum RecordedPart {
    File(Vec<u8>),
    Text(String),
}

/// One request the seam recorded: the method, path and every part `OutgoingRequest::into_parts`
/// handed back, each file part already drained for comparison.
#[derive(Clone, Debug, Eq, PartialEq)]
struct RecordedRequest {
    method: String,
    parts: Vec<(String, RecordedPart)>,
    path: String,
}

/// A hand-written seam answering one prepared response and recording the request it was handed.
struct RecordingTransport {
    requests: Mutex<Vec<RecordedRequest>>,
    responses: Mutex<Vec<(u16, Vec<u8>)>>,
}

impl RecordingTransport {
    fn queued(responses: Vec<(u16, Vec<u8>)>) -> Self {
        Self {
            requests: Mutex::new(Vec::new()),
            responses: Mutex::new(responses),
        }
    }

    fn requests(&self) -> Vec<RecordedRequest> {
        self.requests.lock().unwrap().clone()
    }
}

impl multipart_http_rest_client::Transport for RecordingTransport {
    async fn send(
        &self,
        request: multipart_http_rest_client::OutgoingRequest,
    ) -> Result<multipart_http_rest_client::IncomingResponse, String> {
        ready(()).await;
        let method = request.method().to_owned();
        let path = request.path().to_owned();
        assert_eq!(
            request.query(),
            "",
            "a bodied method carries no query string"
        );
        assert_eq!(
            request.headers(),
            &[],
            "a multipart operation with no header_in binding carries no header"
        );
        assert_eq!(
            request.body(),
            b"",
            "a multipart method's content rides in `parts`, never `body`"
        );
        let parts = request
            .into_parts()
            .into_iter()
            .map(|(name, part)| {
                let recorded = match part {
                    multipart_http_rest_client::OutgoingPart::Text(text) => {
                        RecordedPart::Text(text)
                    }
                    multipart_http_rest_client::OutgoingPart::File(mut source) => {
                        let mut drained = Vec::new();
                        let mut buf = [0_u8; 64];
                        loop {
                            let read = source.pull(&mut buf).unwrap();
                            if read == 0 {
                                break;
                            }
                            drained.extend_from_slice(&buf[..read]);
                        }
                        RecordedPart::File(drained)
                    }
                };
                (name, recorded)
            })
            .collect();
        self.requests.lock().unwrap().push(RecordedRequest {
            method,
            parts,
            path,
        });
        let (status, body) = self.responses.lock().unwrap().remove(0);
        Ok(multipart_http_rest_client::IncomingResponse::new(
            status,
            Vec::new(),
            body,
        ))
    }
}

/// The contract stands on its own: implementing it takes the trait and nothing else, and nothing
/// in this binary placed a dispatcher for it.
pub struct UploadClientBackEnd;

impl UploadClientService<()> for UploadClientBackEnd {
    async fn upload_document(
        &self,
        _ctx: &(),
        folder_id: String,
        title: String,
        _description: Option<String>,
        _attachment: Box<dyn upload_client_service_schema::BodySource + Send>,
    ) -> Result<UploadResponse, UploadError> {
        ready(()).await;
        Ok(UploadResponse {
            document_id: format!("{folder_id}/{title}"),
        })
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
fn a_multipart_call_builds_one_text_part_per_field_and_one_file_part_per_binding() {
    let transport = RecordingTransport::queued(vec![(200, br#"{"document_id":"doc-1"}"#.to_vec())]);
    let client = multipart_http_rest_client::UploadClientServiceClient::new(transport);
    let answered = poll_once(client.upload_document(
        "acme".to_owned(),
        "quarterly-report".to_owned(),
        Some("Q3 numbers".to_owned()),
        Box::new(ByteSource {
            remaining: b"file-bytes".to_vec(),
        }),
    ))
    .unwrap();
    assert_eq!(
        answered,
        Ok(UploadResponse {
            document_id: "doc-1".to_owned()
        })
    );
    assert_eq!(
        client.transport().requests(),
        vec![RecordedRequest {
            method: "POST".to_owned(),
            path: "/folders/acme/documents".to_owned(),
            parts: vec![
                (
                    "title".to_owned(),
                    RecordedPart::Text("quarterly-report".to_owned())
                ),
                (
                    "description".to_owned(),
                    RecordedPart::Text("Q3 numbers".to_owned())
                ),
                (
                    "file".to_owned(),
                    RecordedPart::File(b"file-bytes".to_vec())
                ),
            ],
        }],
        "one text part per carried field, then one file part per `part` binding"
    );
}

#[test]
fn an_absent_optional_field_omits_its_own_text_part() {
    let transport = RecordingTransport::queued(vec![(200, br#"{"document_id":"doc-2"}"#.to_vec())]);
    let client = multipart_http_rest_client::UploadClientServiceClient::new(transport);
    poll_once(client.upload_document(
        "acme".to_owned(),
        "no-description".to_owned(),
        None,
        Box::new(ByteSource {
            remaining: b"bytes".to_vec(),
        }),
    ))
    .unwrap()
    .unwrap();
    let requests = client.transport().requests();
    assert_eq!(
        requests[0].parts,
        vec![
            (
                "title".to_owned(),
                RecordedPart::Text("no-description".to_owned())
            ),
            ("file".to_owned(), RecordedPart::File(b"bytes".to_vec())),
        ],
        "a `None` optional field carries no text part at all"
    );
}

/// `IncomingResponse` reads a header back case-insensitively - exercised directly since this
/// operation declares no `header_out` of its own to reach the accessor through.
#[test]
fn incoming_response_reads_back_a_header_case_insensitively() {
    let response = multipart_http_rest_client::IncomingResponse::new(
        200,
        vec![("ETag".to_owned(), "v1".to_owned())],
        Vec::new(),
    );
    assert_eq!(response.header("etag"), Some("v1"));
}

/// The contract stands on its own: implementing it takes the trait and nothing else, and nothing
/// in this binary placed a dispatcher for it.
#[test]
fn the_contract_is_implementable_where_no_dispatcher_was_placed() {
    let answered = poll_once(UploadClientBackEnd.upload_document(
        &(),
        "acme".to_owned(),
        "report".to_owned(),
        None,
        Box::new(ByteSource {
            remaining: Vec::new(),
        }),
    ))
    .unwrap();
    assert_eq!(
        answered,
        Ok(UploadResponse {
            document_id: "acme/report".to_owned()
        })
    );
}
