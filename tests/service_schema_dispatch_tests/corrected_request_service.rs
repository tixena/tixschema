//! A request whose message names a `preprocess`, on the type and on a field: the dispatcher reads
//! it through both functions - driven through the `http_rest` dispatcher by hand, no server.

#![cfg(feature = "serde")]

use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context as PollContext, Poll, Waker};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::{model_schema, service_schema};

use crate::corrected_request_http_rest_transport::{
    self, DefaultFaultHandler, FaultHandler, IncomingRequest, OutgoingResponse, Route, dispatch,
};
use crate::corrected_request_service_schema::ServiceFault;

/// The parcel a request stores: its type's function gives it the list a request leaves out, and
/// its label's takes the spaces off the ends.
#[model_schema(preprocess = ["withoutFragile"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CorrectedParcel {
    pub fragile_item_codes: Vec<String>,
    #[model_schema_prop(preprocess = ["trimmed"])]
    pub label: String,
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum StoreParcelError {
    DockClosed,
}

/// One operation, whose one argument is the message.
#[service_schema(transports = ["http_rest"])]
pub trait CorrectedRequestService<Ctx> {
    #[service_schema_op(http(method = "POST", path = "/parcels", error_status(DockClosed = 409),))]
    async fn store_parcel(
        &self,
        ctx: &Ctx,
        parcel: CorrectedParcel,
    ) -> Result<CorrectedParcel, StoreParcelError>;
}

/// The implementation of `CorrectedRequestService` these tests dispatch to: it answers the
/// parcel it was handed.
pub struct StoringBackEnd;

impl CorrectedRequestService<()> for StoringBackEnd {
    async fn store_parcel(
        &self,
        _ctx: &(),
        parcel: CorrectedParcel,
    ) -> Result<CorrectedParcel, StoreParcelError> {
        ready(()).await;
        if parcel.label == "closed" {
            return Err(StoreParcelError::DockClosed);
        }
        Ok(parcel)
    }
}

/// An owner-installed `FaultHandler`, which answers what the fault says of a request.
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

fn trimmed(payload: Value) -> Value {
    if let Value::String(text) = payload {
        Value::String(text.trim().to_owned())
    } else {
        payload
    }
}

fn without_fragile(mut payload: Value) -> Value {
    if let Some(object) = payload.as_object_mut() {
        object
            .entry("fragileItemCodes")
            .or_insert_with(|| Value::Array(Vec::new()));
    }
    payload
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

fn posted(body: &[u8]) -> IncomingRequest {
    IncomingRequest::new(
        "POST".to_owned(),
        "/parcels".to_owned(),
        "dock=north".to_owned(),
        vec![("x-trace".to_owned(), "abc".to_owned())],
        body.to_vec(),
    )
}

fn dispatched(body: &[u8]) -> (u16, Vec<u8>) {
    let request = posted(body);
    let response = poll_once(dispatch(
        &StoringBackEnd,
        &(),
        &request,
        &DefaultFaultHandler,
    ))
    .unwrap();
    (response.status(), response.body().to_vec())
}

/// The handler is handed the message both functions corrected: the type's first, which gives it
/// the list, then the label's.
#[test]
fn a_request_is_read_through_the_functions_its_message_names() {
    let (status, body) = dispatched(br#"{"label":"  A  "}"#);
    assert_eq!(status, 200);
    assert_eq!(body, br#"{"fragileItemCodes":[],"label":"A"}"#);
}

/// The handler judges the corrected label, never the one the request wrote.
#[test]
fn a_corrected_request_still_answers_its_declared_error() {
    let (status, body) = dispatched(br#"{"label":" closed "}"#);
    assert_eq!(status, 409);
    assert_eq!(body, br#"{"errorCode":"dock-closed"}"#);
}

/// A function answers a payload it cannot correct as it is, and the read then refuses it in
/// serde's own words.
#[test]
fn a_payload_the_functions_leave_unreadable_is_a_fault() {
    let request = posted(br#"{"label":5}"#);
    let response = poll_once(dispatch(
        &StoringBackEnd,
        &(),
        &request,
        &RecordingFaultHandler,
    ))
    .unwrap();
    assert_eq!(response.status(), 499);
    assert_eq!(
        response.headers(),
        &[("x-fault-kind".to_owned(), "failed validation".to_owned())]
    );
    assert_eq!(
        String::from_utf8_lossy(response.body()),
        "handled: invalid type: integer `5`, expected a string"
    );
}

#[test]
fn the_route_table_lists_the_one_route() {
    let routes = corrected_request_http_rest_transport::ROUTES;
    let paths: Vec<&str> = routes.iter().map(Route::path).collect();
    assert_eq!(routes.len(), 1, "got: {paths:?}");
    let route = &routes[0];
    assert_eq!(route.method(), "POST");
    assert_eq!(route.path(), "/parcels");
    assert_eq!(route.operation(), "store-parcel");
    assert_eq!(route.ok_status(), 200);
    assert_eq!(route.error_statuses(), &[409]);
}

#[test]
fn a_request_reads_back_what_the_functions_were_not_handed() {
    let request = posted(b"{}");
    assert_eq!(request.query(), "dock=north");
    assert_eq!(request.header("x-trace"), Some("abc"));
    assert_eq!(
        request.headers(),
        &[("x-trace".to_owned(), "abc".to_owned())]
    );
}
