//! A service that takes a brand over a `u32`, a brand over an `i64`, an alias of the first, a
//! struct of one slot over a `bool` and a brand over a `String` of digits in a path, a query and a
//! header, and answers a brand in a header beside its reply; its back end and the loop that joins
//! the client to the dispatcher.

use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context as PollContext, Poll, Waker};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

use crate::{http_rest_client, http_rest_transport};

#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct DockSlipNumber(pub u32);

#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct DockTicketNumber(pub i64);

#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct DockLit(pub bool);

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct DockSlipLabel(pub String);

#[model_schema()]
pub type DockSlipAlias = DockSlipNumber;

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum DockError {
    Refused,
}

#[service_schema(transports = ["http_rest"])]
pub trait DockService<Ctx> {
    #[service_schema_op(http(method = "GET", path = "/docks/{slip}"))]
    async fn find(
        &self,
        ctx: &Ctx,
        slip: DockSlipNumber,
        ticket: Option<DockTicketNumber>,
        label: Option<DockSlipLabel>,
        since: Option<DockSlipAlias>,
        slips: Option<Vec<DockSlipNumber>>,
    ) -> Result<String, DockError>;

    #[service_schema_op(http(
        method = "POST",
        path = "/moor",
        header_in("x-slip" = slip),
        header_in("x-ticket" = ticket),
        header_in("x-label" = label),
        header_in("x-lit" = lit),
        header_out("x-next"),
    ))]
    async fn moor(
        &self,
        ctx: &Ctx,
        note: String,
        slip: DockSlipNumber,
        ticket: Option<DockTicketNumber>,
        label: DockSlipLabel,
        lit: DockLit,
    ) -> Result<(String, DockTicketNumber), DockError>;
}

pub struct DockBackEnd;

impl DockService<()> for DockBackEnd {
    async fn find(
        &self,
        _ctx: &(),
        slip: DockSlipNumber,
        ticket: Option<DockTicketNumber>,
        label: Option<DockSlipLabel>,
        since: Option<DockSlipAlias>,
        slips: Option<Vec<DockSlipNumber>>,
    ) -> Result<String, DockError> {
        ready(()).await;
        let numbers: Option<Vec<u32>> = slips.map(|each| each.iter().map(|one| one.0).collect());
        Ok(format!(
            "{} {:?} {:?} {:?} {numbers:?}",
            slip.0,
            ticket.map(|each| each.0),
            label.map(|each| each.0),
            since.map(|each| each.0),
        ))
    }

    async fn moor(
        &self,
        _ctx: &(),
        note: String,
        slip: DockSlipNumber,
        ticket: Option<DockTicketNumber>,
        label: DockSlipLabel,
        lit: DockLit,
    ) -> Result<(String, DockTicketNumber), DockError> {
        ready(()).await;
        let next = ticket.map_or(0, |each| each.0.saturating_add(1));
        Ok((
            format!(
                "{note} {} {} {} {}",
                slip.0,
                label.0,
                ticket.is_some(),
                lit.0
            ),
            DockTicketNumber(next),
        ))
    }
}

/// Each request the loop carried: the line, the headers, the body, then the status and the
/// headers and the body of the answer.
type Exchanged = (
    String,
    Vec<(String, String)>,
    String,
    u16,
    Vec<(String, String)>,
    String,
);

pub struct DockLoop {
    pub exchanged: Mutex<Vec<Exchanged>>,
}

impl http_rest_client::Transport for DockLoop {
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
        let answered = http_rest_transport::dispatch(
            &DockBackEnd,
            &(),
            &incoming,
            &http_rest_transport::DefaultFaultHandler,
        )
        .await;
        self.exchanged.lock().unwrap().push((
            format!(
                "{} {}?{}",
                incoming.method(),
                incoming.path(),
                incoming.query()
            ),
            incoming.headers().to_vec(),
            String::from_utf8(request.body().to_vec()).unwrap(),
            answered.status(),
            answered.headers().to_vec(),
            String::from_utf8(answered.body().to_vec()).unwrap(),
        ));
        Ok(http_rest_client::IncomingResponse::new(
            answered.status(),
            answered.headers().to_vec(),
            answered.body().to_vec(),
        ))
    }
}

/// An owner's `FaultHandler`, which answers every fault with a status and a header of its own.
struct Refusing;

impl http_rest_transport::FaultHandler for Refusing {
    fn on_fault(
        &self,
        fault: &dock_service_schema::ServiceFault,
    ) -> http_rest_transport::OutgoingResponse {
        http_rest_transport::OutgoingResponse::new(
            499,
            headed(&[("x-fault", &format!("{}", fault.kind()))]),
            Vec::new(),
        )
    }
}

pub fn poll_once<Answered>(answering: Answered) -> Option<Answered::Output>
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

fn headed(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect()
}

/// The client writes each brand as the text of its value in a path, a query and a header, the
/// dispatcher reads it back as the value, and a header beside the reply is read back too.
#[test]
fn a_brand_over_a_number_travels_as_the_text_of_the_number_through_the_client_and_dispatcher() {
    let client = http_rest_client::DockServiceClient::new(DockLoop {
        exchanged: Mutex::new(Vec::new()),
    });
    assert_eq!(
        poll_once(client.find(
            DockSlipNumber(7),
            Some(DockTicketNumber(i64::MAX)),
            Some(DockSlipLabel("007".to_owned())),
            Some(DockSlipNumber(9)),
            Some(vec![DockSlipNumber(1), DockSlipNumber(2)]),
        )),
        Some(Ok(format!(
            "7 Some({}) Some(\"007\") Some(9) Some([1, 2])",
            i64::MAX
        ))),
        "find"
    );
    assert_eq!(
        poll_once(client.moor(
            "n".to_owned(),
            DockSlipNumber(7),
            Some(DockTicketNumber(i64::MIN + 1)),
            DockSlipLabel("42".to_owned()),
            DockLit(true),
        )),
        Some(Ok((
            "n 7 42 true true".to_owned(),
            DockTicketNumber(i64::MIN + 2)
        ))),
        "moor"
    );
    let exchanged = client.transport().exchanged.lock().unwrap().clone();
    assert!(
        exchanged[0].0.starts_with("GET /docks/7?"),
        "{}",
        exchanged[0].0
    );
    for text in ["ticket=9223372036854775807", "label=007", "since=9"] {
        assert!(exchanged[0].0.contains(text), "{}", exchanged[0].0);
    }
    assert_eq!(
        exchanged[1].1,
        headed(&[
            ("x-slip", "7"),
            ("x-ticket", "-9223372036854775807"),
            ("x-label", "42"),
            ("x-lit", "true"),
        ])
    );
    assert!(
        exchanged[1]
            .4
            .contains(&("x-next".to_owned(), "-9223372036854775806".to_owned())),
        "{:?}",
        exchanged[1].4
    );
}

/// A brand over a `String` of digits is text, so the dispatcher hands it the digits as text, and
/// a brand over a number refuses text that is no number.
#[test]
fn a_brand_over_text_stays_text_and_a_brand_over_a_number_refuses_what_is_no_number() {
    let request = |path: &str, query: &str| {
        let incoming = http_rest_transport::IncomingRequest::new(
            "GET".to_owned(),
            path.to_owned(),
            query.to_owned(),
            Vec::new(),
            Vec::new(),
        );
        let answered = poll_once(http_rest_transport::dispatch(
            &DockBackEnd,
            &(),
            &incoming,
            &http_rest_transport::DefaultFaultHandler,
        ))
        .unwrap();
        (answered.status(), answered.body().to_vec())
    };
    let (found, body) = request("/docks/12", "ticket=-3&label=00123&slips=4%2C5");
    assert_eq!(
        (found, String::from_utf8(body).unwrap()),
        (
            200,
            "\"12 Some(-3) Some(\\\"00123\\\") None Some([4, 5])\"".to_owned()
        )
    );
    let (refused_slip, _) = request("/docks/abc", "ticket=1&label=a&slips=");
    assert_eq!(refused_slip, 400, "a slip that is no number");
}

/// The route table lists both routes, and a request reads back what it was built with.
#[test]
fn the_route_table_lists_each_route_and_a_request_reads_back_what_it_was_built_with() {
    let routes: Vec<(&str, &str, &str, u16, &[u16])> = http_rest_transport::ROUTES
        .iter()
        .map(|route| {
            (
                route.operation(),
                route.method(),
                route.path(),
                route.ok_status(),
                route.error_statuses(),
            )
        })
        .collect();
    assert_eq!(
        routes,
        [
            ("find", "GET", "/docks/{slip}", 200, [422_u16].as_slice()),
            ("moor", "POST", "/moor", 200, [422_u16].as_slice()),
        ]
    );
    let incoming = http_rest_transport::IncomingRequest::new(
        "GET".to_owned(),
        "/nowhere".to_owned(),
        "q=1".to_owned(),
        headed(&[("x-slip", "7")]),
        b"b".to_vec(),
    );
    assert_eq!(
        (incoming.body(), incoming.query(), incoming.header("x-slip")),
        (&b"b"[..], "q=1", Some("7"))
    );
    let answered = poll_once(http_rest_transport::dispatch(
        &DockBackEnd,
        &(),
        &http_rest_transport::IncomingRequest::new(
            "POST".to_owned(),
            "/moor".to_owned(),
            String::new(),
            headed(&[
                ("x-slip", "7"),
                ("x-label", "9"),
                ("x-ticket", "x"),
                ("x-lit", "true"),
            ]),
            br#"{"note":"n"}"#.to_vec(),
        ),
        &http_rest_transport::DefaultFaultHandler,
    ))
    .unwrap();
    assert_eq!(answered.status(), 400, "a ticket that is no number");

    let refused = poll_once(http_rest_transport::dispatch(
        &DockBackEnd,
        &(),
        &incoming,
        &Refusing,
    ))
    .unwrap();
    let faulted = headed(&[("x-fault", "unknown operation")]);
    assert_eq!(
        (refused.status(), refused.headers()),
        (499, faulted.as_slice())
    );
}
