//! What each layout of a fault answers over `http_rest` and over `mcp`.

use crate::http_service::status_service_schema::ServiceFault;
use crate::http_service::{DraftedAbove, PlainAbove, StatusBackEnd};
use crate::http_transport::{
    self, DefaultFaultHandler, FaultHandler, IncomingRequest, OutgoingResponse, Route, dispatch,
};
use crate::mcp_service::{
    AskingServiceSchema, DraftedAsksAbove, DraftedAsksAboveDraft, DraftedAsksBelow,
    DraftedAsksBelowDraft, PlainAsksAbove, PlainAsksBelow,
};
use core::future::Future;
use core::pin::pin;
use core::task::{Context as PollContext, Poll, Waker};
use fault_variants_apart::{DraftedApart, DraftedApartDraft, PlainApart};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// An owner-installed `FaultHandler`, which builds an `OutgoingResponse` by hand.
struct RecordingFaultHandler;

impl FaultHandler for RecordingFaultHandler {
    fn on_fault(&self, fault: &ServiceFault) -> OutgoingResponse {
        OutgoingResponse::new(
            499,
            vec![("x-fault-kind".to_owned(), format!("{}", fault.kind()))],
            b"handled".to_vec(),
        )
    }
}

/// The faults of the example the guide shows.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ShipFault {
    Closed,
    NotFound,
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

fn status_of(path: &str) -> u16 {
    let request = IncomingRequest::new(
        "GET".to_owned(),
        path.to_owned(),
        String::new(),
        Vec::new(),
        Vec::new(),
    );
    let mut pinned = pin!(dispatch(
        &StatusBackEnd,
        &(),
        &request,
        &DefaultFaultHandler
    ));
    let mut polling = PollContext::from_waker(Waker::noop());
    match pinned.as_mut().poll(&mut polling) {
        Poll::Ready(response) => response.status(),
        Poll::Pending => 0,
    }
}

fn answers_by_name(operation: &str) {
    assert_eq!(
        status_of(&format!("/{operation}/not-found")),
        404,
        "{operation}"
    );
    assert_eq!(
        status_of(&format!("/{operation}/closed")),
        410,
        "{operation}"
    );
}

#[test]
fn a_fault_with_a_draft_declared_above_the_service_answers_by_variant_name() {
    answers_by_name("drafted-above");
}

#[test]
fn a_fault_with_a_draft_declared_below_the_service_answers_by_variant_name() {
    answers_by_name("drafted-below");
}

#[test]
fn a_fault_with_a_draft_declared_in_another_crate_answers_by_variant_name() {
    answers_by_name("drafted-apart");
}

#[test]
fn a_plain_fault_declared_above_the_service_answers_by_variant_name() {
    answers_by_name("plain-above");
}

#[test]
fn a_plain_fault_declared_below_the_service_answers_by_variant_name() {
    answers_by_name("plain-below");
}

#[test]
fn a_plain_fault_declared_in_another_crate_answers_by_variant_name() {
    answers_by_name("plain-apart");
}

#[test]
fn a_variant_is_named_as_rust_declares_it() {
    assert_eq!(DraftedAbove::NotFound.model_schema_variant(), "NotFound");
    assert_eq!(DraftedAbove::MODEL_SCHEMA_VARIANTS, ["Closed", "NotFound"]);
    assert_eq!(PlainAbove::Closed.model_schema_variant(), "Closed");
    assert_eq!(PlainAbove::MODEL_SCHEMA_VARIANTS, ["Closed", "NotFound"]);
}

fn drafted_apart_asking() -> DraftedApart {
    DraftedApartDraft::NeedsDrive {
        message: "connect".to_owned(),
        url: "https://example.com/connect".to_owned(),
    }
    .into()
}

#[test]
fn an_arm_that_asks_with_a_link_is_read_whatever_the_order_of_the_declarations() {
    let above: DraftedAsksAbove = DraftedAsksAboveDraft::NeedsDrive {
        message: "connect".to_owned(),
        url: "https://example.com/connect".to_owned(),
    }
    .into();
    let below: DraftedAsksBelow = DraftedAsksBelowDraft::NeedsDrive {
        message: "connect".to_owned(),
        url: "https://example.com/connect".to_owned(),
    }
    .into();
    let link = Some(("https://example.com/connect", "connect"));
    assert_eq!(AskingServiceSchema::mcp_asks_link_above(&above), link);
    assert_eq!(AskingServiceSchema::mcp_asks_link_below(&below), link);
    assert_eq!(
        AskingServiceSchema::mcp_asks_link_apart(&drafted_apart_asking()),
        link
    );
    assert_eq!(
        AskingServiceSchema::mcp_asks_link_above(&DraftedAsksAbove::NotFound),
        None
    );
    assert_eq!(
        AskingServiceSchema::mcp_asks_link_below(&DraftedAsksBelow::NotFound),
        None
    );
    assert_eq!(
        AskingServiceSchema::mcp_asks_link_apart(&DraftedApart::NotFound),
        None
    );
}

#[test]
fn an_arm_that_asks_with_a_form_is_read_whatever_the_order_of_the_declarations() {
    assert!(AskingServiceSchema::mcp_asks_form_above(
        &PlainAsksAbove::NeedsConfirmation
    ));
    assert!(!AskingServiceSchema::mcp_asks_form_above(
        &PlainAsksAbove::NotFound
    ));
    assert!(AskingServiceSchema::mcp_asks_form_below(
        &PlainAsksBelow::NeedsConfirmation
    ));
    assert!(!AskingServiceSchema::mcp_asks_form_below(
        &PlainAsksBelow::NotFound
    ));
    assert!(AskingServiceSchema::mcp_asks_form_apart(
        &PlainApart::Closed
    ));
    assert!(!AskingServiceSchema::mcp_asks_form_apart(
        &PlainApart::NotFound
    ));
    let closed: DraftedApart = DraftedApartDraft::Closed {
        reason: "shut".to_owned(),
    }
    .into();
    assert!(AskingServiceSchema::mcp_asks_form_drafted_apart(&closed));
    assert!(!AskingServiceSchema::mcp_asks_form_drafted_apart(
        &DraftedApart::NotFound
    ));
}

/// The route table an adapter iterates to register a handler: one row per fault.
#[test]
fn the_route_table_lists_the_route_of_each_fault() {
    let routes = http_transport::ROUTES;
    let paths: Vec<&str> = routes.iter().map(Route::path).collect();
    assert_eq!(routes.len(), 6, "got: {paths:?}");
    for route in routes {
        assert_eq!(route.method(), "GET");
        assert_eq!(route.ok_status(), 200);
        assert!(route.operation().contains('-'), "{}", route.operation());
        assert!(route.error_statuses().contains(&404), "{}", route.path());
    }
}

#[test]
fn an_incoming_request_reads_back_its_body_headers_and_query() {
    let request = IncomingRequest::new(
        "GET".to_owned(),
        "/plain-above/closed".to_owned(),
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

#[test]
fn an_installed_fault_handler_builds_an_outgoing_response_by_hand() {
    let request = IncomingRequest::new(
        "GET".to_owned(),
        "/nowhere".to_owned(),
        String::new(),
        Vec::new(),
        Vec::new(),
    );
    let response = poll_once(dispatch(
        &StatusBackEnd,
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
    assert_eq!(response.body(), b"handled");
}

/// Every operation of the asking service answers its fault.
#[test]
fn the_asking_service_answers_each_fault() {
    use crate::mcp_service::{AskingBackEnd, AskingService as _};

    fn answered<Answer>(answering: impl Future<Output = Answer>) -> Answer {
        poll_once(answering).unwrap()
    }

    let document = || "d".to_owned();
    assert!(matches!(
        answered(AskingBackEnd.link_above(&(), document())),
        Err(DraftedAsksAbove::NotFound)
    ));
    assert!(matches!(
        answered(AskingBackEnd.link_apart(&(), document())),
        Err(DraftedApart::NotFound)
    ));
    assert!(matches!(
        answered(AskingBackEnd.link_below(&(), document())),
        Err(DraftedAsksBelow::NotFound)
    ));
    assert!(matches!(
        answered(AskingBackEnd.form_above(&(), document(), None)),
        Err(PlainAsksAbove::NotFound)
    ));
    assert!(matches!(
        answered(AskingBackEnd.form_apart(&(), document(), None)),
        Err(PlainApart::NotFound)
    ));
    assert!(matches!(
        answered(AskingBackEnd.form_below(&(), document(), None)),
        Err(PlainAsksBelow::NotFound)
    ));
    assert!(matches!(
        answered(AskingBackEnd.form_drafted_apart(&(), document(), None)),
        Err(DraftedApart::NotFound)
    ));
}

/// The guide shows the lines this test runs.
#[test]
fn the_guide_shows_how_a_variant_is_read_without_a_match() {
    let fault: ShipFault = ShipFault::NotFound;
    assert_eq!(fault.model_schema_variant(), "NotFound");
    assert_eq!(ShipFault::MODEL_SCHEMA_VARIANTS, ["Closed", "NotFound"]);
    let guide = include_str!("../../docs/guide/models.md");
    for shown in [
        "let fault: ShipFault = ShipFault::NotFound;",
        "assert_eq!(fault.model_schema_variant(), \"NotFound\");",
        "assert_eq!(ShipFault::MODEL_SCHEMA_VARIANTS, [\"Closed\", \"NotFound\"]);",
    ] {
        assert!(guide.contains(shown), "the guide does not show: {shown}");
    }
}
