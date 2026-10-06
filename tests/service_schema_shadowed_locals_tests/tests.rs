use crate::shadow_service_schema::ServiceFault;
use crate::{http_rest_client, http_rest_dispatcher};
use core::future::{Future, ready};
use core::pin::pin;
use core::sync::atomic::{AtomicU32, Ordering};
use core::task::{Context as PollContext, Poll, Waker};
use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum ShadowFailure {
    Empty,
}

/// What the handler was handed, argument by argument.
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ShadowHanded {
    pub handed: Vec<String>,
}

#[service_schema(transports = ["http_rest"])]
pub trait ShadowService<Ctx> {
    #[service_schema_op(http(
        method = "POST",
        path = "/paths/{sending}",
        header_in("x-headers" = headers),
        header_in("x-path" = path),
        header_in("x-query" = query),
    ))]
    async fn shadow_path(
        &self,
        ctx: &Ctx,
        sending: String,
        headers: String,
        path: String,
        query: Option<String>,
    ) -> Result<ShadowHanded, ShadowFailure>;

    #[service_schema_op(http(
        method = "POST",
        path = "/requests/{message}/{captured}",
        header_in("x-body" = body),
        header_in("x-handler" = handler),
        header_in("x-request" = request),
    ))]
    async fn shadow_request(
        &self,
        ctx: &Ctx,
        message: String,
        captured: String,
        body: String,
        handler: String,
        request: String,
    ) -> Result<ShadowHanded, ShadowFailure>;
}

pub struct ShadowBackEnd;

impl ShadowService<()> for ShadowBackEnd {
    async fn shadow_path(
        &self,
        _ctx: &(),
        sending: String,
        headers: String,
        path: String,
        query: Option<String>,
    ) -> Result<ShadowHanded, ShadowFailure> {
        ready(()).await;
        if sending.is_empty() {
            return Err(ShadowFailure::Empty);
        }
        Ok(ShadowHanded {
            handed: [Some(sending), Some(headers), Some(path), query]
                .into_iter()
                .flatten()
                .collect(),
        })
    }

    async fn shadow_request(
        &self,
        _ctx: &(),
        message: String,
        captured: String,
        body: String,
        handler: String,
        request: String,
    ) -> Result<ShadowHanded, ShadowFailure> {
        ready(()).await;
        if message.is_empty() {
            return Err(ShadowFailure::Empty);
        }
        Ok(ShadowHanded {
            handed: vec![message, captured, body, handler, request],
        })
    }
}

/// Hands each request the client builds to the dispatcher, and the dispatcher's answer back.
struct Looped {
    sent: AtomicU32,
}

impl http_rest_client::Transport for Looped {
    async fn send(
        &self,
        request: http_rest_client::OutgoingRequest,
    ) -> Result<http_rest_client::IncomingResponse, String> {
        self.sent.fetch_add(1, Ordering::Relaxed);
        let incoming = http_rest_dispatcher::IncomingRequest::new(
            request.method().to_owned(),
            request.path().to_owned(),
            request.query().to_owned(),
            request.headers().to_vec(),
            request.body().to_vec(),
        );
        let answered = http_rest_dispatcher::dispatch(
            &ShadowBackEnd,
            &(),
            &incoming,
            &http_rest_dispatcher::DefaultFaultHandler,
        )
        .await;
        Ok(http_rest_client::IncomingResponse::new(
            answered.status(),
            answered.headers().to_vec(),
            answered.body().to_vec(),
        ))
    }
}

/// Answers every fault `499`, under a header naming its kind.
struct RefusedAs499;

impl http_rest_dispatcher::FaultHandler for RefusedAs499 {
    fn on_fault(&self, fault: &ServiceFault) -> http_rest_dispatcher::OutgoingResponse {
        http_rest_dispatcher::OutgoingResponse::new(
            499,
            vec![("x-fault-kind".to_owned(), format!("{}", fault.kind()))],
            b"refused".to_vec(),
        )
    }
}

/// Nothing above suspends, so one poll answers it.
fn poll_once<Answering>(answering: Answering) -> Option<Answering::Output>
where
    Answering: Future,
{
    let mut pinned = pin!(answering);
    match pinned
        .as_mut()
        .poll(&mut PollContext::from_waker(Waker::noop()))
    {
        Poll::Ready(answer) => Some(answer),
        Poll::Pending => None,
    }
}

#[test]
fn an_argument_named_after_a_transports_own_local_reaches_the_handler_as_it_was_passed() {
    let client = http_rest_client::ShadowServiceClient::new(Looped {
        sent: AtomicU32::new(0),
    });
    let paths = poll_once(client.shadow_path(
        "s".to_owned(),
        "hs".to_owned(),
        "p".to_owned(),
        Some("q".to_owned()),
    ))
    .unwrap();
    assert_eq!(
        paths.map(|handed| handed.handed.join(" ")),
        Ok("s hs p q".to_owned())
    );
    let requests = poll_once(client.shadow_request(
        "m".to_owned(),
        "c".to_owned(),
        "b".to_owned(),
        "h".to_owned(),
        "r".to_owned(),
    ))
    .unwrap();
    assert_eq!(
        requests.map(|handed| handed.handed.join(" ")),
        Ok("m c b h r".to_owned())
    );
    assert_eq!(client.transport().sent.load(Ordering::Relaxed), 2);
}

#[test]
fn the_route_table_and_a_refused_request_read_back() {
    let routes: Vec<_> = http_rest_dispatcher::ROUTES
        .iter()
        .map(|route| (route.method(), route.path(), route.operation()))
        .collect();
    assert_eq!(
        routes,
        [
            ("POST", "/paths/{sending}", "shadow-path"),
            ("POST", "/requests/{message}/{captured}", "shadow-request"),
        ]
    );
    let route = &http_rest_dispatcher::ROUTES[0];
    assert_eq!(
        (route.ok_status(), route.error_statuses()),
        (200, [422].as_slice())
    );
    let incoming = http_rest_dispatcher::IncomingRequest::new(
        "POST".to_owned(),
        "/paths/s".to_owned(),
        String::new(),
        Vec::new(),
        b"{}".to_vec(),
    );
    assert_eq!(incoming.headers(), []);
    assert_eq!(incoming.query(), "");
    let refused = poll_once(http_rest_dispatcher::dispatch(
        &ShadowBackEnd,
        &(),
        &incoming,
        &RefusedAs499,
    ))
    .unwrap();
    assert_eq!(refused.status(), 499, "a required header was not carried");
    let carried = http_rest_client::IncomingResponse::new(
        refused.status(),
        refused.headers().to_vec(),
        refused.body().to_vec(),
    );
    assert_eq!(carried.header("x-fault-kind"), Some("failed validation"));
}
