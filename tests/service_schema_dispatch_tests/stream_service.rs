//! A `body = "stream"` operation driven through the `http_rest` dispatcher by hand — no server —
//! with a hand-rolled chunked reader proving genuine incremental pulling through `BodySource`, and
//! a `206` range answer expressing `content-range` alongside the streamed body.

#![cfg(feature = "serde")]

use crate::stream_http_rest_transport;
use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context as PollContext, Poll, Waker};
use serde::{Deserialize, Serialize};
use std::io::{self, Read};
use tixschema::{model_schema, service_schema};

/// The full body every test range-slices or streams whole. Long enough that draining it through a
/// reader capped at [`CHUNK_CAP`] bytes takes several `pull()` calls, not one buffered copy.
const CONTENT: &[u8] = b"the quick brown fox jumps over the lazy dog";

/// The most [`ChunkedSlice::read`] ever returns in one call, regardless of the caller's own buffer
/// size — what makes draining [`CONTENT`] genuinely incremental.
const CHUNK_CAP: usize = 5;

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum ContentError {
    NotFound,
    RangeNotSatisfiable,
}

#[service_schema(transports = ["http_rest"])]
pub trait ContentService<Ctx> {
    #[service_schema_op(http(
        method = "GET",
        path = "/documents/{document_id}/content",
        header_in("range" = byte_range),
        body = "stream",
        error_status(NotFound = 404, RangeNotSatisfiable = 416),
    ))]
    async fn get_content(
        &self,
        ctx: &Ctx,
        document_id: String,
        byte_range: Option<String>,
    ) -> Result<content_service_schema::StreamedAnswer, ContentError>;

    /// The same content, with a declared `header_out` composed onto the streamed answer - proving
    /// the composition parse.rs now permits reaches both `StreamedAnswer` arms, `Full` and
    /// `Partial` alike, rather than only the JSON and bytes kinds.
    #[service_schema_op(http(
        method = "GET",
        path = "/documents/{document_id}/content-with-etag",
        header_in("range" = byte_range),
        header_out("etag"),
        body = "stream",
        error_status(NotFound = 404, RangeNotSatisfiable = 416),
    ))]
    async fn get_content_with_etag(
        &self,
        ctx: &Ctx,
        document_id: String,
        byte_range: Option<String>,
    ) -> Result<(content_service_schema::StreamedAnswer, String), ContentError>;
}

/// A chunked [`Read`] source: every call answers at most [`CHUNK_CAP`] bytes, whatever the
/// caller's own buffer offers. [`BodySource`](content_service_schema::BodySource) is blanket
/// implemented for every `Read`, so this satisfies the seam for free.
struct ChunkedSlice {
    remaining: Vec<u8>,
}

/// The author's own implementation: a fixed document, answered whole or range-sliced.
pub struct DocumentStore;

/// An owner-installed `FaultHandler` still reaches `OutgoingResponse::new` on a streamed
/// service's dispatcher - the same construction path a JSON or bytes service's own override uses,
/// unaffected by `OutgoingResponse` carrying `OutgoingBody` instead of a bare `Vec<u8>` here.
struct RecordingFaultHandler;

impl ChunkedSlice {
    fn new(content: &[u8]) -> Self {
        Self {
            remaining: content.to_vec(),
        }
    }
}

impl Read for ChunkedSlice {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let take = buf.len().min(self.remaining.len()).min(CHUNK_CAP);
        let rest = self.remaining.split_off(take);
        buf[..take].copy_from_slice(&self.remaining);
        self.remaining = rest;
        Ok(take)
    }
}

impl ContentService<()> for DocumentStore {
    async fn get_content(
        &self,
        _ctx: &(),
        document_id: String,
        byte_range: Option<String>,
    ) -> Result<content_service_schema::StreamedAnswer, ContentError> {
        ready(()).await;
        if document_id == "missing" {
            return Err(ContentError::NotFound);
        }
        let Some(range) = byte_range else {
            return Ok(content_service_schema::StreamedAnswer::Full(Box::new(
                ChunkedSlice::new(CONTENT),
            )));
        };
        let Some((start, end)) = parse_range(&range, CONTENT.len()) else {
            return Err(ContentError::RangeNotSatisfiable);
        };
        Ok(content_service_schema::StreamedAnswer::Partial {
            source: Box::new(ChunkedSlice::new(&CONTENT[start..=end])),
            content_range: format!("bytes {start}-{end}/{}", CONTENT.len()),
        })
    }

    async fn get_content_with_etag(
        &self,
        ctx: &(),
        document_id: String,
        byte_range: Option<String>,
    ) -> Result<(content_service_schema::StreamedAnswer, String), ContentError> {
        let answered = self.get_content(ctx, document_id, byte_range).await?;
        Ok((answered, "v9".to_owned()))
    }
}

impl stream_http_rest_transport::FaultHandler for RecordingFaultHandler {
    fn on_fault(
        &self,
        fault: &content_service_schema::ServiceFault,
    ) -> stream_http_rest_transport::OutgoingResponse {
        stream_http_rest_transport::OutgoingResponse::new(
            499,
            vec![("x-fault-kind".to_owned(), format!("{}", fault.kind()))],
            stream_http_rest_transport::OutgoingBody::Bytes(
                format!("handled: {}", fault.detail()).into_bytes(),
            ),
        )
    }
}

/// Parses `bytes=START-END` into an inclusive, in-bounds `(start, end)`, or `None` where the range
/// does not fit within `len` - the handler's own job, `header_in` having wired the raw header
/// straight through as `Option<String>`.
fn parse_range(range: &str, len: usize) -> Option<(usize, usize)> {
    let spec = range.strip_prefix("bytes=")?;
    let (lower_text, upper_text) = spec.split_once('-')?;
    let lower: usize = lower_text.parse().ok()?;
    let upper: usize = upper_text.parse().ok()?;
    if lower > upper || upper >= len {
        return None;
    }
    Some((lower, upper))
}

/// Drains an `OutgoingBody` fully, standing in for a transport adapter's own pull loop - not part
/// of anything `#[service_schema]` emits, exactly the boundary the seam draws. Answers the bytes
/// and how many `pull()` calls it took, so a test can tell a genuinely incremental drain from one
/// buffered copy.
fn drain(body: stream_http_rest_transport::OutgoingBody) -> (Vec<u8>, u32) {
    match body {
        stream_http_rest_transport::OutgoingBody::Bytes(bytes) => (bytes, 1),
        stream_http_rest_transport::OutgoingBody::Stream(mut source) => {
            let mut drained = Vec::new();
            let mut pulls = 0_u32;
            let mut buf = [0_u8; 64];
            loop {
                let read = source.pull(&mut buf).unwrap();
                if read == 0 {
                    break;
                }
                pulls += 1;
                drained.extend_from_slice(&buf[..read]);
            }
            (drained, pulls)
        }
    }
}

/// Dispatches one plain-terms HTTP request by hand - no server - and answers with the response the
/// dispatcher wrote back, drained and counted, faults included through the default `FaultHandler`.
fn dispatched(
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
) -> (u16, Vec<(String, String)>, Vec<u8>, u32) {
    let request = stream_http_rest_transport::IncomingRequest::new(
        method.to_owned(),
        path.to_owned(),
        String::new(),
        headers
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect(),
        Vec::new(),
    );
    let response = poll_once(stream_http_rest_transport::dispatch(
        &DocumentStore,
        &(),
        &request,
        &stream_http_rest_transport::DefaultFaultHandler,
    ))
    .unwrap();
    let status = response.status();
    let response_headers = response.headers().to_vec();
    let (body, pulls) = drain(response.into_body());
    (status, response_headers, body, pulls)
}

/// The transports never suspend, so one poll answers them; `None` says an assumption about the
/// bodies above stopped holding rather than that the runtime is missing.
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

/// The whole body, streamed through the seam in genuinely incremental chunks - more than one
/// `pull()` call, not one buffered copy dressed up as a stream.
#[test]
fn a_full_body_streams_through_the_seam_in_more_than_one_pull() {
    let (status, headers, body, pulls) = dispatched("GET", "/documents/present/content", &[]);
    assert_eq!(status, 200);
    assert_eq!(body, CONTENT);
    assert!(
        pulls > 1,
        "a {}-byte body capped at {CHUNK_CAP} bytes per read should take several pulls, got {pulls}",
        CONTENT.len()
    );
    assert!(
        !headers.iter().any(|(name, _)| name == "content-range"),
        "a full body carries no content-range. got: {headers:?}"
    );
}

/// A `Range` header answers `206` with `content-range` composed alongside the streamed slice -
/// the response header and the undrained body travelling together on one `OutgoingResponse`.
#[test]
fn a_range_header_answers_206_with_content_range_and_the_sliced_body() {
    let (status, headers, body, pulls) = dispatched(
        "GET",
        "/documents/present/content",
        &[("range", "bytes=4-8")],
    );
    assert_eq!(status, 206);
    assert_eq!(body, b"quick");
    assert!(pulls >= 1, "got {pulls} pulls");
    assert_eq!(
        headers
            .iter()
            .find(|(name, _)| name == "content-range")
            .map(|(_, value)| value.as_str()),
        Some(format!("bytes 4-8/{}", CONTENT.len())).as_deref(),
        "got: {headers:?}"
    );
}

/// A range past the end of the body is answered through the declared error, mapped to its own
/// status exactly like any other declared error - `416`, not a fault.
#[test]
fn a_range_past_the_end_answers_the_declared_416() {
    let (status, _headers, body, _pulls) = dispatched(
        "GET",
        "/documents/present/content",
        &[("range", "bytes=0-999")],
    );
    assert_eq!(status, 416);
    assert_eq!(body, br#"{"errorCode":"range-not-satisfiable"}"#);
}

/// An unknown document still answers the declared `404`, exactly like any other declared error on
/// a streamed operation.
#[test]
fn an_unknown_document_answers_the_declared_404() {
    let (status, _headers, body, _pulls) = dispatched("GET", "/documents/missing/content", &[]);
    assert_eq!(status, 404);
    assert_eq!(body, br#"{"errorCode":"not-found"}"#);
}

/// The route table an adapter iterates to register a handler: one row per streamed operation, its
/// statuses included - mirrors the same claim `DocumentService`'s own harness makes, for a service
/// whose operations stream instead of answering JSON.
#[test]
fn the_route_table_lists_both_streamed_routes() {
    let routes = stream_http_rest_transport::ROUTES;
    let paths: Vec<&str> = routes
        .iter()
        .map(stream_http_rest_transport::Route::path)
        .collect();
    assert_eq!(routes.len(), 2, "got: {paths:?}");
    assert_eq!(routes[0].method(), "GET");
    assert_eq!(routes[0].path(), "/documents/{document_id}/content");
    assert_eq!(routes[0].operation(), "get-content");
    assert_eq!(routes[0].ok_status(), 200);
    assert_eq!(routes[0].error_statuses(), &[404, 416]);
}

/// A declared `header_out` composed onto a streamed answer rides beside `content-range` on `206`
/// and alone on the full `200` answer - the same composition the bytes kind now carries, reached
/// through both `StreamedAnswer` arms rather than only one.
#[test]
fn a_header_out_entry_composes_onto_both_streamed_answer_arms() {
    let (full_status, full_headers, full_body, _full_pulls) =
        dispatched("GET", "/documents/present/content-with-etag", &[]);
    assert_eq!(full_status, 200);
    assert_eq!(full_body, CONTENT);
    assert_eq!(
        full_headers
            .iter()
            .find(|(name, _)| name == "etag")
            .map(|(_, value)| value.as_str()),
        Some("v9"),
        "got: {full_headers:?}"
    );

    let (partial_status, partial_headers, partial_body, _partial_pulls) = dispatched(
        "GET",
        "/documents/present/content-with-etag",
        &[("range", "bytes=4-8")],
    );
    assert_eq!(partial_status, 206);
    assert_eq!(partial_body, b"quick");
    assert_eq!(
        partial_headers
            .iter()
            .find(|(name, _)| name == "content-range")
            .map(|(_, value)| value.as_str()),
        Some(format!("bytes 4-8/{}", CONTENT.len())).as_deref(),
        "content-range still rides beside the declared header. got: {partial_headers:?}"
    );
    assert_eq!(
        partial_headers
            .iter()
            .find(|(name, _)| name == "etag")
            .map(|(_, value)| value.as_str()),
        Some("v9"),
        "got: {partial_headers:?}"
    );
}

/// `IncomingRequest` reads back everything it was built with - the same accessors every other
/// operation's arm already reaches for, exercised here for the streamed operation's own dispatcher
/// expansion.
#[test]
fn an_incoming_request_reads_back_its_body_headers_and_query() {
    let request = stream_http_rest_transport::IncomingRequest::new(
        "GET".to_owned(),
        "/documents/present/content".to_owned(),
        "unused=1".to_owned(),
        vec![("range".to_owned(), "bytes=0-3".to_owned())],
        b"ignored".to_vec(),
    );
    assert_eq!(request.body(), b"ignored");
    assert_eq!(request.query(), "unused=1");
    assert_eq!(
        request.headers(),
        &[("range".to_owned(), "bytes=0-3".to_owned())]
    );
}

/// An owner-installed `FaultHandler` still builds an `OutgoingResponse` by hand on a streamed
/// service's dispatcher, exercising `OutgoingResponse::new` and `OutgoingBody::Bytes` directly.
#[test]
fn an_installed_fault_handler_still_builds_an_outgoing_response_by_hand() {
    let request = stream_http_rest_transport::IncomingRequest::new(
        "GET".to_owned(),
        "/nowhere".to_owned(),
        String::new(),
        Vec::new(),
        Vec::new(),
    );
    let response = poll_once(stream_http_rest_transport::dispatch(
        &DocumentStore,
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
    let (body, _pulls) = drain(response.into_body());
    assert_eq!(
        body,
        b"handled: the service answers to no operation by that name"
    );
}
