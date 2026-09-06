//! One service, one implementation, both transports.
//!
//! `DocumentService` declares the four operation shapes this crate's transport tasks closed
//! around: `get_version` binds a claimed request header and a declared response header and maps
//! every variant its error declares; `archive_document` answers no payload of its own and
//! overrides the bodyless default status; `get_thumbnail` answers a `bytes`-kind body and reads an
//! unclaimed field off the query string; and `sweep_documents` names no `http(...)` group at all,
//! so both transports default it. `purge_document` is one-way, beside them, so both a client's
//! notify path and a dispatcher's answer-nothing path are exercised too.
//!
//! `DocumentBackEnd` is the one implementation. [`AmqpLoop`] and [`HttpLoop`] are hand-written
//! seams joining each transport's generated client straight back into its generated dispatcher —
//! no server, no bus, no socket — so a call through either client is answered by the very
//! `DocumentBackEnd` the other loop also answers through.

#![cfg(feature = "serde")]

use crate::{amqp_client, amqp_transport, http_rest_client, http_rest_transport};
use core::future::{Future, ready};
use core::pin::pin;
use core::ptr;
use core::task::{Context as PollContext, Poll, Waker};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tixschema::{model_schema, service_schema};

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GetVersionRequest {
    pub document_id: String,
    pub version_id: String,
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VersionResponse {
    pub content: String,
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum GetVersionError {
    NotFound,
    VersionGone,
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum ArchiveError {
    AlreadyArchived,
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum ThumbnailError {
    NotFound,
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SweepReport {
    pub swept: u32,
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum SweepError {
    DbError,
}

#[service_schema(transports = ["amqp_rpc", "http_rest"])]
pub trait DocumentService<Ctx> {
    /// Answers no payload of its own, and overrides the bodyless default status.
    #[service_schema_op(http(
        method = "POST",
        path = "/documents/{document_id}/archive",
        ok_status = 202,
        error_status(AlreadyArchived = 409),
    ))]
    async fn archive_document(&self, ctx: &Ctx, document_id: String) -> Result<(), ArchiveError>;

    /// A `bytes`-kind body: the success arm is the raw bytes and their content type, not JSON.
    /// `download` binds no path placeholder, so a bodyless `GET` reads it off the query string.
    #[service_schema_op(http(
        method = "GET",
        path = "/documents/{document_id}/thumbnail",
        error_status(NotFound = 404),
        body = "bytes",
    ))]
    async fn get_thumbnail(
        &self,
        ctx: &Ctx,
        document_id: String,
        download: Option<bool>,
    ) -> Result<(Vec<u8>, String), ThumbnailError>;

    /// A header claimed off the request, a header written into the reply, and every variant its
    /// own error declares mapped to its own status.
    #[service_schema_op(http(
        method = "GET",
        path = "/documents/{document_id}/versions/{version_id}",
        ok_status = 200,
        header_in("range" = byte_range),
        header_out("etag"),
        error_status(NotFound = 404, VersionGone = 410),
    ))]
    async fn get_version(
        &self,
        ctx: &Ctx,
        req: GetVersionRequest,
        byte_range: Option<String>,
    ) -> Result<(VersionResponse, String), GetVersionError>;

    /// One-way: a client sends and neither transport expects an answer.
    #[service_schema_op(one_way, http(method = "DELETE", path = "/documents/{document_id}"))]
    async fn purge_document(&self, ctx: &Ctx, document_id: String);

    /// Names no `http(...)` group at all: both transports default it — `http_rest` to
    /// `POST /sweep-documents`, and every declared error to the fixed default-binding status.
    async fn sweep_documents(&self, ctx: &Ctx) -> Result<SweepReport, SweepError>;
}

/// The one implementation this harness proves serves both transports, unmodified between them.
///
/// `sweep_documents` takes no argument of its own, so `sweep_fails` is what a test reaches for
/// instead to drive it down its declared error arm.
pub struct DocumentBackEnd {
    reached: Mutex<Vec<String>>,
    sweep_fails: Mutex<bool>,
}

impl DocumentService<()> for DocumentBackEnd {
    async fn archive_document(&self, _ctx: &(), document_id: String) -> Result<(), ArchiveError> {
        ready(()).await;
        self.reach(format!("archive_document {document_id}"));
        if document_id == "already" {
            return Err(ArchiveError::AlreadyArchived);
        }
        Ok(())
    }

    async fn get_thumbnail(
        &self,
        _ctx: &(),
        document_id: String,
        download: Option<bool>,
    ) -> Result<(Vec<u8>, String), ThumbnailError> {
        ready(()).await;
        self.reach(format!("get_thumbnail {document_id} {download:?}"));
        if document_id == "missing" {
            return Err(ThumbnailError::NotFound);
        }
        Ok((vec![0x89, 0x50, 0x4e, 0x47], "image/png".to_owned()))
    }

    async fn get_version(
        &self,
        _ctx: &(),
        req: GetVersionRequest,
        byte_range: Option<String>,
    ) -> Result<(VersionResponse, String), GetVersionError> {
        ready(()).await;
        self.reach(format!(
            "get_version {} {} {byte_range:?}",
            req.document_id, req.version_id
        ));
        if req.document_id == "missing" {
            return Err(GetVersionError::NotFound);
        }
        if req.document_id == "gone" {
            return Err(GetVersionError::VersionGone);
        }
        Ok((
            VersionResponse {
                content: format!("{}@{}", req.document_id, req.version_id),
            },
            "v7".to_owned(),
        ))
    }

    async fn purge_document(&self, _ctx: &(), document_id: String) {
        ready(()).await;
        self.reach(format!("purge_document {document_id}"));
    }

    async fn sweep_documents(&self, _ctx: &()) -> Result<SweepReport, SweepError> {
        ready(()).await;
        self.reach("sweep_documents".to_owned());
        if *self.sweep_fails.lock().unwrap() {
            return Err(SweepError::DbError);
        }
        Ok(SweepReport { swept: 3 })
    }
}

impl DocumentBackEnd {
    fn fail_next_sweep(&self) {
        *self.sweep_fails.lock().unwrap() = true;
    }

    fn new() -> Self {
        Self {
            reached: Mutex::new(Vec::new()),
            sweep_fails: Mutex::new(false),
        }
    }

    fn reach(&self, what: String) {
        self.reached.lock().unwrap().push(what);
    }

    fn reached(&self) -> Vec<String> {
        self.reached.lock().unwrap().clone()
    }
}

/// What one AMQP reply carries: the bytes a real transport would put on the wire, and the headers
/// a `header_out` binding wrote beside them.
type AmqpAnswer = (Vec<u8>, Vec<(String, String)>);

/// Captures how the dispatcher settled one AMQP message.
pub struct AmqpCapture {
    settled: Mutex<Option<AmqpAnswer>>,
}

impl amqp_transport::Reply for AmqpCapture {
    async fn fault(&self, fault: document_service_schema::ServiceFault) {
        ready(()).await;
        // The framing is the transport's own business: a fault rides tagged inside the failure
        // arm, the shape a caller in either language narrows on.
        let framed = serde_json::json!({
            "ok": false,
            "error": { "isServiceFault": true, "fault": fault },
        });
        self.settle(serde_json::to_vec(&framed).unwrap(), Vec::new());
    }

    async fn send<T>(&self, value: T, headers: Vec<(String, String)>)
    where
        T: Serialize + Send,
    {
        ready(()).await;
        self.settle(serde_json::to_vec(&value).unwrap(), headers);
    }
}

impl AmqpCapture {
    fn new() -> Self {
        Self {
            settled: Mutex::new(None),
        }
    }

    fn settle(&self, body: Vec<u8>, headers: Vec<(String, String)>) {
        *self.settled.lock().unwrap() = Some((body, headers));
    }

    fn take(&self) -> AmqpAnswer {
        self.settled.lock().unwrap().take().unwrap()
    }
}

/// Joins the `amqp_rpc` client and dispatcher through the headers-capable seam: a call is turned
/// straight into a dispatch against the implementation this harness proves serves both
/// transports, and the answer handed back is exactly what the dispatcher wrote.
pub struct AmqpLoop<'back_end> {
    service: &'back_end DocumentBackEnd,
}

impl<'back_end> AmqpLoop<'back_end> {
    fn new(service: &'back_end DocumentBackEnd) -> Self {
        Self { service }
    }
}

impl amqp_client::Transport for AmqpLoop<'_> {
    async fn notify<T>(
        &self,
        operation: &str,
        payload: T,
        headers: Vec<(String, String)>,
    ) -> Result<(), String>
    where
        T: Serialize + Send,
    {
        let capture = AmqpCapture::new();
        amqp_transport::dispatch(
            self.service,
            &(),
            &amqp_transport::IncomingMessage::new(
                operation.to_owned(),
                serde_json::to_vec(&payload).unwrap(),
                headers,
            ),
            &capture,
        )
        .await;
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
        let capture = AmqpCapture::new();
        amqp_transport::dispatch(
            self.service,
            &(),
            &amqp_transport::IncomingMessage::new(
                operation.to_owned(),
                serde_json::to_vec(&payload).unwrap(),
                headers,
            ),
            &capture,
        )
        .await;
        Ok(capture.take())
    }
}

/// Joins the `http_rest` client and dispatcher through an in-memory plain-terms loop: a client
/// request record is handed straight to `dispatch`, and the answer handed back is exactly the
/// `OutgoingResponse` the dispatcher — and whichever `FaultHandler` this loop was built with —
/// wrote.
pub struct HttpLoop<'back_end, Handler> {
    handler: Handler,
    service: &'back_end DocumentBackEnd,
}

impl<'back_end, Handler> HttpLoop<'back_end, Handler> {
    fn new(service: &'back_end DocumentBackEnd, handler: Handler) -> Self {
        Self { handler, service }
    }
}

impl<Handler> http_rest_client::Transport for HttpLoop<'_, Handler>
where
    Handler: http_rest_transport::FaultHandler + Sync,
{
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
        let answered =
            http_rest_transport::dispatch(self.service, &(), &incoming, &self.handler).await;
        Ok(http_rest_client::IncomingResponse::new(
            answered.status(),
            answered.headers().to_vec(),
            answered.body().to_vec(),
        ))
    }
}

/// A `FaultHandler` that answers every kind with its own status and a marker header, rather than
/// the default's fixed status and small fault JSON — proving an owner's override is what a loop
/// built on it actually answers with, not only that installing one compiles.
struct RecordingFaultHandler;

impl http_rest_transport::FaultHandler for RecordingFaultHandler {
    fn on_fault(
        &self,
        fault: &document_service_schema::ServiceFault,
    ) -> http_rest_transport::OutgoingResponse {
        use document_service_schema::ServiceFaultKind;
        let status = match fault.kind() {
            ServiceFaultKind::UnknownOperation => 490,
            ServiceFaultKind::FailedValidation | ServiceFaultKind::UndeserializablePayload => 491,
            ServiceFaultKind::HandlerPanic => 492,
            ServiceFaultKind::TransportFailure => 493,
        };
        http_rest_transport::OutgoingResponse::new(
            status,
            vec![("x-fault-kind".to_owned(), format!("{}", fault.kind()))],
            format!("recorded: {}", fault.detail()).into_bytes(),
        )
    }
}

/// An `amqp_client::Transport` that answers every `request` with fixed bytes and never reaches a
/// dispatcher, for pinning what the client's own decode does with an envelope no real dispatcher
/// would ever write.
struct StubTransport {
    body: Vec<u8>,
}

impl StubTransport {
    fn answering(value: &serde_json::Value) -> Self {
        Self {
            body: serde_json::to_vec(value).unwrap(),
        }
    }
}

impl amqp_client::Transport for StubTransport {
    async fn notify<T>(
        &self,
        _operation: &str,
        _payload: T,
        _headers: Vec<(String, String)>,
    ) -> Result<(), String>
    where
        T: Serialize + Send,
    {
        ready(()).await;
        Ok(())
    }

    async fn request<T>(
        &self,
        _operation: &str,
        _payload: T,
        _headers: Vec<(String, String)>,
    ) -> Result<(Vec<u8>, Vec<(String, String)>), String>
    where
        T: Serialize + Send,
    {
        ready(()).await;
        Ok((self.body.clone(), Vec::new()))
    }
}

/// The probes never suspend, so one poll answers them; `None` says an assumption about the bodies
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

// -------------------------------------------------------------------------------------------
// The `http_rest` loop
// -------------------------------------------------------------------------------------------

#[test]
fn the_http_loop_round_trips_the_header_bound_operation() {
    let service = DocumentBackEnd::new();
    let client = http_rest_client::DocumentServiceClient::new(HttpLoop::new(
        &service,
        http_rest_transport::DefaultFaultHandler,
    ));
    let answered = poll_once(client.get_version(
        GetVersionRequest {
            document_id: "doc-1".to_owned(),
            version_id: "v1".to_owned(),
        },
        Some("bytes=0-10".to_owned()),
    ))
    .unwrap();
    assert_eq!(
        answered,
        Ok((
            VersionResponse {
                content: "doc-1@v1".to_owned(),
            },
            "v7".to_owned(),
        )),
        "the response rides in the body and the header_out value is rejoined from the reply's own \
         headers"
    );
    assert_eq!(
        service.reached(),
        vec![r#"get_version doc-1 v1 Some("bytes=0-10")"#.to_owned()],
        "the header_in binding decoded the claimed request header into the implementation's own \
         argument"
    );
}

#[test]
fn the_http_loop_answers_the_header_bound_operations_complete_error_mapping() {
    for (document_id, declared) in [
        ("missing", GetVersionError::NotFound),
        ("gone", GetVersionError::VersionGone),
    ] {
        let service = DocumentBackEnd::new();
        let client = http_rest_client::DocumentServiceClient::new(HttpLoop::new(
            &service,
            http_rest_transport::DefaultFaultHandler,
        ));
        let answered = poll_once(client.get_version(
            GetVersionRequest {
                document_id: document_id.to_owned(),
                version_id: "v1".to_owned(),
            },
            None,
        ))
        .unwrap();
        assert_eq!(
            answered,
            Err(document_service_schema::CallError::Operation(declared)),
            "`{document_id}` must decode through the status this variant declared"
        );
    }
}

#[test]
fn the_http_loop_round_trips_the_no_payload_operation_at_its_overridden_ok_status() {
    let service = DocumentBackEnd::new();
    let client = http_rest_client::DocumentServiceClient::new(HttpLoop::new(
        &service,
        http_rest_transport::DefaultFaultHandler,
    ));
    poll_once(client.archive_document("doc-1".to_owned()))
        .unwrap()
        .unwrap();
    assert_eq!(service.reached(), vec!["archive_document doc-1".to_owned()]);

    let answered = poll_once(client.archive_document("already".to_owned())).unwrap();
    assert_eq!(
        answered,
        Err(document_service_schema::CallError::Operation(
            ArchiveError::AlreadyArchived
        )),
        "the mapped error still answers even though the operation carries no payload of its own"
    );
}

#[test]
fn the_http_loop_round_trips_the_bytes_operation_and_its_content_type() {
    let service = DocumentBackEnd::new();
    let client = http_rest_client::DocumentServiceClient::new(HttpLoop::new(
        &service,
        http_rest_transport::DefaultFaultHandler,
    ));
    let answered = poll_once(client.get_thumbnail("doc-1".to_owned(), Some(true))).unwrap();
    assert_eq!(
        answered,
        Ok((vec![0x89, 0x50, 0x4e, 0x47], "image/png".to_owned())),
        "the bytes body and its content type cross with no JSON decode on the success path"
    );
    assert_eq!(
        service.reached(),
        vec!["get_thumbnail doc-1 Some(true)".to_owned()],
        "the field the path left unclaimed rode the query string both ways"
    );

    let missing = poll_once(client.get_thumbnail("missing".to_owned(), None)).unwrap();
    assert_eq!(
        missing,
        Err(document_service_schema::CallError::Operation(
            ThumbnailError::NotFound
        )),
        "a bytes operation's declared error still answers JSON, exactly like any other operation's"
    );
}

#[test]
fn the_http_loop_round_trips_the_defaulted_operation_at_its_default_path() {
    let service = DocumentBackEnd::new();
    let client = http_rest_client::DocumentServiceClient::new(HttpLoop::new(
        &service,
        http_rest_transport::DefaultFaultHandler,
    ));
    let answered = poll_once(client.sweep_documents()).unwrap();
    assert_eq!(answered, Ok(SweepReport { swept: 3 }));
    assert_eq!(service.reached(), vec!["sweep_documents".to_owned()]);

    service.fail_next_sweep();
    let refused = poll_once(client.sweep_documents()).unwrap();
    assert_eq!(
        refused,
        Err(document_service_schema::CallError::Operation(
            SweepError::DbError
        )),
        "an operation naming no http(...) group still answers its declared error, at the fixed \
         default-binding status"
    );
}

#[test]
fn the_route_table_defaults_the_unannotated_operation_to_post_and_its_wire_name() {
    let route = http_rest_transport::ROUTES
        .iter()
        .find(|route| route.operation() == "sweep-documents")
        .unwrap();
    assert_eq!(route.method(), "POST");
    assert_eq!(route.path(), "/sweep-documents");
    assert_eq!(route.ok_status(), 200);
    assert_eq!(
        route.error_statuses(),
        &[422],
        "every declared error of an operation naming no http(...) group answers at the fixed \
         default-binding status"
    );
}

#[test]
fn a_custom_fault_handler_installed_on_the_http_loop_answers_an_unmatched_route_in_its_own_words() {
    let service = DocumentBackEnd::new();
    let request = http_rest_transport::IncomingRequest::new(
        "GET".to_owned(),
        "/nowhere".to_owned(),
        String::new(),
        Vec::new(),
        Vec::new(),
    );
    let response = poll_once(http_rest_transport::dispatch(
        &service,
        &(),
        &request,
        &RecordingFaultHandler,
    ))
    .unwrap();
    assert_eq!(
        response.status(),
        490,
        "the installed handler's own status answered, not the fixed default's 404"
    );
    assert_eq!(
        response
            .headers()
            .iter()
            .find(|(name, _)| name == "x-fault-kind")
            .map(|(_, value)| value.as_str()),
        Some("unknown operation"),
        "got: {:?}",
        response.headers()
    );
    assert!(
        response.body().starts_with(b"recorded: "),
        "got: {}",
        String::from_utf8_lossy(response.body())
    );
}

#[test]
fn a_custom_fault_handler_installed_on_the_http_loop_answers_a_validation_failure_in_its_own_words()
{
    let service = DocumentBackEnd::new();
    let request = http_rest_transport::IncomingRequest::new(
        "POST".to_owned(),
        "/sweep-documents".to_owned(),
        String::new(),
        Vec::new(),
        b"not json".to_vec(),
    );
    let response = poll_once(http_rest_transport::dispatch(
        &service,
        &(),
        &request,
        &RecordingFaultHandler,
    ))
    .unwrap();
    assert_eq!(response.status(), 491);
    assert!(
        service.reached().is_empty(),
        "the implementation must not run over a payload that never validated"
    );
}

// -------------------------------------------------------------------------------------------
// The `amqp_rpc` loop
// -------------------------------------------------------------------------------------------

#[test]
fn the_amqp_loop_round_trips_the_header_bound_operation_through_the_headers_channel() {
    let service = DocumentBackEnd::new();
    let client = amqp_client::DocumentServiceClient::new(AmqpLoop::new(&service));
    let answered = poll_once(client.get_version(
        GetVersionRequest {
            document_id: "doc-1".to_owned(),
            version_id: "v1".to_owned(),
        },
        Some("bytes=0-10".to_owned()),
    ))
    .unwrap()
    .unwrap();
    assert_eq!(
        answered,
        (
            VersionResponse {
                content: "doc-1@v1".to_owned(),
            },
            "v7".to_owned(),
        ),
        "the header_in value rode the message headers table into the implementation, and the \
         header_out value rode the reply headers back out"
    );
    assert_eq!(
        service.reached(),
        vec![r#"get_version doc-1 v1 Some("bytes=0-10")"#.to_owned()]
    );
}

#[test]
fn the_amqp_loop_answers_the_header_bound_operations_complete_error_mapping() {
    for (document_id, declared) in [
        ("missing", GetVersionError::NotFound),
        ("gone", GetVersionError::VersionGone),
    ] {
        let service = DocumentBackEnd::new();
        let client = amqp_client::DocumentServiceClient::new(AmqpLoop::new(&service));
        let answered = poll_once(client.get_version(
            GetVersionRequest {
                document_id: document_id.to_owned(),
                version_id: "v1".to_owned(),
            },
            None,
        ))
        .unwrap();
        assert_eq!(
            answered,
            Err(document_service_schema::CallError::Operation(declared)),
            "`{document_id}`"
        );
    }
}

/// The mapped-error arm of the no-payload operation round-trips over `amqp_rpc` exactly like any
/// other declared error: the failure arm never carries the envelope's `value` at all, so the unit
/// success fix below (which is about the success arm alone) never touches it.
#[test]
fn the_amqp_loop_answers_the_no_payload_operations_mapped_error() {
    let service = DocumentBackEnd::new();
    let client = amqp_client::DocumentServiceClient::new(AmqpLoop::new(&service));
    let answered = poll_once(client.archive_document("already".to_owned())).unwrap();
    assert_eq!(
        answered,
        Err(document_service_schema::CallError::Operation(
            ArchiveError::AlreadyArchived
        ))
    );
    assert_eq!(
        service.reached(),
        vec!["archive_document already".to_owned()]
    );
}

/// A unit success round-trips over `amqp_rpc`: the implementation answers `Ok(())`, the dispatcher
/// writes `{"ok":true,"value":null}` through `Answered::answering` unconditionally (`arm`,
/// `src/service_schema/transport/amqp_rpc.rs`), and the client reads it back as `Ok(())`.
///
/// The wire bytes are exactly what any success answers — `Answered::value` stays `Option<T>` and
/// `()` still serializes to `null`, indistinguishable there from an absent value for any `T`. What
/// changed is the read: an operation whose declared success is the unit type reads through
/// `read_unit_answer`, which asks the envelope's `ok` flag alone, `()` needing nothing carried to
/// exist. Every other success type still reads through `read_answer`, which still demands a
/// carried value — pinned by
/// `a_non_unit_success_with_a_genuinely_absent_value_still_faults`, below.
#[test]
fn the_amqp_loop_round_trips_the_no_payload_operation_s_unit_success() {
    let service = DocumentBackEnd::new();
    let client = amqp_client::DocumentServiceClient::new(AmqpLoop::new(&service));
    let answered = poll_once(client.archive_document("doc-1".to_owned())).unwrap();
    assert_eq!(
        answered,
        Ok(()),
        "a unit success is carried over amqp_rpc's own envelope, not lost as an undeserializable \
         payload"
    );
    assert_eq!(
        service.reached(),
        vec!["archive_document doc-1".to_owned()],
        "the implementation ran and its Ok(()) is what the client read back"
    );
}

/// The mirror of the fix above: a success type that is not the unit type still demands a carried
/// value, so an envelope answering `ok` with no `value` at all is still the fault it always was.
///
/// A real dispatcher never writes that envelope for a non-unit success — `sweep_documents`
/// answers `SweepReport`, and `Answered::answering` always carries `Some(value)` on the `Ok` arm
/// — so [`StubTransport`] stands in for the wire, answering the bytes directly and proving the
/// client's own read, not a dispatcher that could never produce them.
#[test]
fn a_non_unit_success_with_a_genuinely_absent_value_still_faults() {
    let client = amqp_client::DocumentServiceClient::new(StubTransport::answering(
        &serde_json::json!({ "ok": true }),
    ));
    let answered = poll_once(client.sweep_documents()).unwrap();
    let reported = match &answered {
        Err(document_service_schema::CallError::Fault(reported)) => Some(reported),
        Ok(_) | Err(document_service_schema::CallError::Operation(_)) => None,
    }
    .unwrap();
    assert_eq!(
        reported.kind(),
        document_service_schema::ServiceFaultKind::UndeserializablePayload
    );
    assert_eq!(
        reported.detail(),
        "the answer said `ok` and carried no value",
        "a non-unit success still demands a carried value; only the unit type reads an absent one \
         as Ok(())"
    );
}

#[test]
fn the_amqp_loop_round_trips_the_bytes_operation() {
    let service = DocumentBackEnd::new();
    let client = amqp_client::DocumentServiceClient::new(AmqpLoop::new(&service));
    let answered = poll_once(client.get_thumbnail("doc-1".to_owned(), Some(true)))
        .unwrap()
        .unwrap();
    assert_eq!(
        answered,
        (vec![0x89, 0x50, 0x4e, 0x47], "image/png".to_owned()),
        "a `bytes`-kind success type is carried like any other value over amqp_rpc's own generic \
         envelope"
    );
    assert_eq!(
        service.reached(),
        vec!["get_thumbnail doc-1 Some(true)".to_owned()]
    );
}

#[test]
fn the_amqp_loop_round_trips_the_defaulted_operation() {
    let service = DocumentBackEnd::new();
    let client = amqp_client::DocumentServiceClient::new(AmqpLoop::new(&service));
    let answered = poll_once(client.sweep_documents()).unwrap();
    assert_eq!(answered, Ok(SweepReport { swept: 3 }));

    service.fail_next_sweep();
    let refused = poll_once(client.sweep_documents()).unwrap();
    assert_eq!(
        refused,
        Err(document_service_schema::CallError::Operation(
            SweepError::DbError
        ))
    );
}

#[test]
fn the_http_loop_delivers_the_one_way_operation_and_answers_nothing() {
    let service = DocumentBackEnd::new();
    let client = http_rest_client::DocumentServiceClient::new(HttpLoop::new(
        &service,
        http_rest_transport::DefaultFaultHandler,
    ));
    poll_once(client.purge_document("doc-1".to_owned()))
        .unwrap()
        .unwrap();
    assert_eq!(service.reached(), vec!["purge_document doc-1".to_owned()]);
}

/// `IncomingRequest` reads back every header and the query string it was built with, not only the
/// ones this service's own operations happen to read.
#[test]
fn an_incoming_request_reads_back_every_header_and_the_query_string_it_was_built_with() {
    let request = http_rest_transport::IncomingRequest::new(
        "GET".to_owned(),
        "/documents/x/thumbnail".to_owned(),
        "download=true".to_owned(),
        vec![("Accept".to_owned(), "image/png".to_owned())],
        Vec::new(),
    );
    assert_eq!(
        request.headers(),
        &[("Accept".to_owned(), "image/png".to_owned())]
    );
    assert_eq!(request.query(), "download=true");
}

// -------------------------------------------------------------------------------------------
// The `amqp_rpc` loop: the one-way operation
// -------------------------------------------------------------------------------------------

#[test]
fn the_amqp_loop_delivers_the_one_way_operation_through_the_client_s_notify_path() {
    let service = DocumentBackEnd::new();
    let client = amqp_client::DocumentServiceClient::new(AmqpLoop::new(&service));
    poll_once(client.purge_document("doc-1".to_owned()))
        .unwrap()
        .unwrap();
    assert_eq!(service.reached(), vec!["purge_document doc-1".to_owned()]);
}

// -------------------------------------------------------------------------------------------
// One implementation, both transports
// -------------------------------------------------------------------------------------------

#[test]
fn the_same_back_end_instance_answers_the_same_operation_over_both_transports_unmodified() {
    let service = DocumentBackEnd::new();
    let amqp_client = amqp_client::DocumentServiceClient::new(AmqpLoop::new(&service));
    poll_once(amqp_client.purge_document("doc-amqp".to_owned()))
        .unwrap()
        .unwrap();
    assert!(
        ptr::eq(amqp_client.transport().service, &raw const service),
        "the amqp loop this client is bound to answers through the very instance under test"
    );

    let http_client = http_rest_client::DocumentServiceClient::new(HttpLoop::new(
        &service,
        http_rest_transport::DefaultFaultHandler,
    ));
    poll_once(http_client.purge_document("doc-http".to_owned()))
        .unwrap()
        .unwrap();
    assert!(
        ptr::eq(http_client.transport().service, &raw const service),
        "the http loop this client is bound to answers through the same instance too"
    );

    assert_eq!(
        service.reached(),
        vec![
            "purge_document doc-amqp".to_owned(),
            "purge_document doc-http".to_owned(),
        ],
        "one `DocumentBackEnd`, reached once per transport, with no trait method touched between \
         the two calls"
    );
}
