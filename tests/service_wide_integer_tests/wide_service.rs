//! A service that carries a bare `u64`, in a path, a header and a body, read by the `http_rest`
//! dispatcher exactly beyond `i64::MAX`.

use core::future::{Future as _, ready};
use core::pin::pin;
use core::task::{Context as PollContext, Poll, Waker};

use tixschema::service_schema;

use crate::wide_http_rest_transport;

/// A bare `u64` bound whole by its placeholder beside a header of one, and a bare `u64` as the
/// body.
#[service_schema(transports = ["http_rest"])]
pub trait PurgeService<Ctx> {
    #[service_schema_op(http(method = "POST", path = "/purges"))]
    async fn count(&self, ctx: &Ctx, count: u64) -> Result<u64, String>;

    #[service_schema_op(http(
        method = "GET",
        path = "/purges/{id}",
        header_in("x-before" = before),
    ))]
    async fn purge(&self, ctx: &Ctx, id: u64, before: u64) -> Result<u64, String>;
}

/// The back end, which answers the larger of the two integers it is handed.
pub struct PurgeBackEnd;

impl PurgeService<()> for PurgeBackEnd {
    async fn count(&self, _ctx: &(), count: u64) -> Result<u64, String> {
        ready(()).await;
        Ok(count)
    }

    async fn purge(&self, _ctx: &(), id: u64, before: u64) -> Result<u64, String> {
        ready(()).await;
        Ok(id.max(before))
    }
}

/// An owner's `FaultHandler`, which answers every fault with a status and a header of its own.
struct Refusing;

impl wide_http_rest_transport::FaultHandler for Refusing {
    fn on_fault(
        &self,
        fault: &purge_service_schema::ServiceFault,
    ) -> wide_http_rest_transport::OutgoingResponse {
        wide_http_rest_transport::OutgoingResponse::new(
            499,
            vec![("x-fault".to_owned(), format!("{}", fault.kind()))],
            Vec::new(),
        )
    }
}

/// What the dispatcher answers a request of `method`, `path`, `headers` and `body` with: its
/// status and its body, or `None` where the dispatch is left pending.
fn dispatched(
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) -> Option<(u16, String)> {
    let request = wide_http_rest_transport::IncomingRequest::new(
        method.to_owned(),
        path.to_owned(),
        String::new(),
        headers
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect(),
        body.to_vec(),
    );
    let mut pinned = pin!(wide_http_rest_transport::dispatch(
        &PurgeBackEnd,
        &(),
        &request,
        &wide_http_rest_transport::DefaultFaultHandler,
    ));
    let Poll::Ready(response) = pinned
        .as_mut()
        .poll(&mut PollContext::from_waker(Waker::noop()))
    else {
        return None;
    };
    Some((
        response.status(),
        String::from_utf8(response.body().to_vec()).unwrap(),
    ))
}

/// A path placeholder, a header and a body of `u64::MAX` are read exactly, and the reply is
/// written as the JSON number, and a body of its digits as a string is refused, as serde refuses
/// it.
#[test]
fn the_http_dispatcher_reads_a_bare_u64_beyond_an_i64_exactly() {
    assert_eq!(
        dispatched(
            "GET",
            "/purges/18446744073709551615",
            &[("x-before", "1")],
            b""
        ),
        Some((200, "18446744073709551615".to_owned())),
        "a placeholder"
    );
    assert_eq!(
        dispatched(
            "GET",
            "/purges/1",
            &[("x-before", "18446744073709551615")],
            b""
        ),
        Some((200, "18446744073709551615".to_owned())),
        "a header"
    );
    assert_eq!(
        dispatched("POST", "/purges", &[], b"18446744073709551615"),
        Some((200, "18446744073709551615".to_owned())),
        "a body"
    );
    assert_eq!(
        dispatched("POST", "/purges", &[], br#""18446744073709551615""#).map(|(status, _)| status),
        Some(400),
        "a body of the digits as a string"
    );
}

/// Each route the dispatcher answers, with its statuses, and a placeholder that is no `u64`
/// answered through an owner's own handler.
#[test]
fn the_route_table_lists_each_route_and_a_refused_placeholder_reaches_the_owners_handler() {
    let routes: Vec<(&str, &str, &str, u16, &[u16])> = wide_http_rest_transport::ROUTES
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
            ("count", "POST", "/purges", 200, [422_u16].as_slice()),
            ("purge", "GET", "/purges/{id}", 200, [422_u16].as_slice()),
        ],
        "the routes"
    );
    let request = wide_http_rest_transport::IncomingRequest::new(
        "GET".to_owned(),
        "/purges/seven".to_owned(),
        String::new(),
        vec![("x-before".to_owned(), "1".to_owned())],
        Vec::new(),
    );
    assert_eq!(
        (request.query(), request.headers()),
        ("", [("x-before".to_owned(), "1".to_owned())].as_slice()),
        "the request"
    );
    let mut pinned = pin!(wide_http_rest_transport::dispatch(
        &PurgeBackEnd,
        &(),
        &request,
        &Refusing,
    ));
    let answered = match pinned
        .as_mut()
        .poll(&mut PollContext::from_waker(Waker::noop()))
    {
        Poll::Ready(response) => Some((response.status(), response.headers().to_vec())),
        Poll::Pending => None,
    };
    assert_eq!(
        answered,
        Some((
            499,
            vec![("x-fault".to_owned(), "failed validation".to_owned())]
        )),
        "the owner's answer"
    );
}
