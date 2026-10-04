//! A service with one request-and-reply operation and one one-way operation, driven through the
//! server macro's own `dispatch` — the same items the dispatcher macro emits, reused unchanged —
//! and `serve_until` named at a concrete type, since a real `lapin::Channel` cannot be built
//! without a connection. The loop `serve_until` runs is driven through `serve_deliveries`, which
//! takes everything the broker would supply as plain streams and futures.

#![cfg(feature = "serde")]

use crate::amqp_server;
use core::future::{Future, Ready, pending, ready};
use core::mem::size_of_val;
use core::num::NonZeroU16;
use core::pin::pin;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::task::{Context as PollContext, Poll, Waker};
use core::time::Duration;
use futures::StreamExt as _;
use futures::stream;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tixschema::{model_schema, service_schema};
use tokio::sync::{Barrier, Notify, oneshot};
use tokio::task::yield_now;
use tokio::time::timeout;

/// How long a test waits before calling a hang a failure: a driver serving one delivery at a time
/// never reaches a barrier the whole limit has to meet at, and would otherwise wait forever.
const PATIENCE: Duration = Duration::from_secs(5);

#[model_schema()]
#[derive(Deserialize, Serialize)]
pub struct PingRequest {
    pub organization_id: String,
}

#[model_schema()]
#[derive(Deserialize, Serialize)]
pub struct PingResponse {
    pub credits: u32,
}

#[model_schema()]
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum PingError {
    DbError,
}

#[service_schema(transports = ["amqp_rpc"])]
pub trait PingService<Ctx> {
    /// No reply, so the arm still has to settle the delivery without publishing anything.
    #[service_schema_op(one_way)]
    async fn note(&self, ctx: &Ctx, req: PingRequest);

    /// A request-and-reply operation, dispatched with the span the consumer loop opens entered.
    async fn ping(&self, ctx: &Ctx, req: PingRequest) -> Result<PingResponse, PingError>;
}

/// Answers the contract against the server macro's own `Context`, the same struct
/// [`amqp_server::serve_until`] builds one of per delivery.
pub struct PingBackEnd;

impl PingService<amqp_server::Context> for PingBackEnd {
    async fn note(&self, ctx: &amqp_server::Context, req: PingRequest) {
        let _entered = ctx.logger.enter();
        let _read = ready(req.organization_id.len()).await;
    }

    async fn ping(
        &self,
        ctx: &amqp_server::Context,
        req: PingRequest,
    ) -> Result<PingResponse, PingError> {
        let _entered = ctx.logger.enter();
        ready(()).await;
        if req.organization_id == "unlucky" {
            Err(PingError::DbError)
        } else {
            Ok(PingResponse { credits: 7 })
        }
    }
}

/// One of the two ways an arm answers. Exactly one lands per request-and-reply dispatch, and none
/// at all where the one-way operation reached its implementation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Settled {
    Fault(ping_service_schema::ServiceFault),
    Sent(String),
}

/// Records how each message was settled instead of publishing anything, standing in for the
/// `ReplyHandle` a real delivery would be answered through.
pub struct ProbeReply {
    settled: Mutex<Vec<Settled>>,
}

impl amqp_server::Reply for ProbeReply {
    async fn fault(&self, fault: ping_service_schema::ServiceFault) {
        ready(()).await;
        self.record(Settled::Fault(fault));
    }

    async fn send<T>(&self, value: T, _headers: Vec<(String, String)>)
    where
        T: Serialize + Send,
    {
        ready(()).await;
        self.record(Settled::Sent(serde_json::to_string(&value).unwrap()));
    }
}

impl ProbeReply {
    fn new() -> Self {
        Self {
            settled: Mutex::new(Vec::new()),
        }
    }

    fn record(&self, what: Settled) {
        self.settled.lock().unwrap().push(what);
    }

    fn settled(&self) -> Vec<Settled> {
        self.settled.lock().unwrap().clone()
    }
}

/// The probe never suspends, so one poll answers it; `None` says an assumption about the bodies
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

fn dispatched(operation: &str, payload: &str) -> Vec<Settled> {
    let service = PingBackEnd;
    let reply = ProbeReply::new();
    poll_once(amqp_server::dispatch(
        &service,
        &amqp_server::Context {
            logger: tracing::Span::none(),
        },
        &amqp_server::IncomingMessage::new(
            operation.to_owned(),
            payload.as_bytes().to_vec(),
            Vec::new(),
        ),
        &reply,
    ))
    .unwrap();
    reply.settled()
}

#[test]
fn the_server_macro_s_dispatch_answers_the_same_way_the_dispatcher_macro_s_does() {
    let settled = dispatched("ping", r#"{"organization_id":"acme"}"#);
    assert_eq!(
        settled,
        vec![Settled::Sent(
            r#"{"ok":true,"value":{"credits":7}}"#.to_owned()
        )],
        "got: {settled:?}"
    );
}

#[test]
fn the_operation_s_own_error_rides_in_the_failure_arm_rather_than_becoming_a_fault() {
    let settled = dispatched("ping", r#"{"organization_id":"unlucky"}"#);
    assert_eq!(
        settled,
        vec![Settled::Sent(
            r#"{"error":{"errorCode":"db-error"},"ok":false}"#.to_owned()
        )],
        "got: {settled:?}"
    );
}

#[test]
fn a_one_way_operation_settles_without_publishing_anything() {
    let settled = dispatched("note", r#"{"organization_id":"acme"}"#);
    assert!(settled.is_empty(), "got: {settled:?}");
}

#[test]
fn a_name_nothing_answers_to_becomes_a_fault_through_the_same_handle() {
    let settled = dispatched("get-the-balance", r#"{"organization_id":"acme"}"#);
    assert_eq!(settled.len(), 1, "got: {settled:?}");
    assert!(
        matches!(&settled[0], Settled::Fault(reported)
            if reported.kind() == ping_service_schema::ServiceFaultKind::UnknownOperation),
        "got: {settled:?}"
    );
}

/// A real `lapin::Channel` cannot be built without a connection, so `serve_until` is named at a
/// concrete service and shutdown future rather than run. Compiling this line is what proves every
/// `::lapin`, `::tokio` and `::futures` path the consumer loop names resolves, and what keeps the
/// loop and the framing it calls from being dead code in this binary.
#[test]
fn serve_until_is_reachable_at_a_concrete_service_and_shutdown_future() {
    let named = amqp_server::serve_until::<PingBackEnd, Ready<()>>;
    assert_eq!(
        size_of_val(&named),
        0,
        "a bare fn item names nothing to store"
    );
}

/// A caller runs the loop on a multi-threaded runtime, which spawns only a `Send` future: this
/// compiles only while the one `serve_until` answers is.
fn spawnable<'serving>(
    channel: &'serving lapin::Channel,
    service: &'serving PingBackEnd,
) -> impl Future<Output = Result<amqp_server::Stopped, lapin::Error>> + Send + 'serving {
    amqp_server::serve_until(channel, "ping", service, NonZeroU16::MIN, ready(()))
}

#[test]
fn serve_until_answers_a_future_a_multi_threaded_runtime_can_spawn() {
    let named = spawnable;
    assert_eq!(
        size_of_val(&named),
        0,
        "a bare fn item names nothing to store"
    );
}

fn record(events: &Mutex<Vec<String>>, event: &str) {
    events.lock().unwrap().push(event.to_owned());
}

fn recorded(events: &Mutex<Vec<String>>) -> Vec<String> {
    events.lock().unwrap().clone()
}

/// Ten deliveries under a limit of ten are all in progress at once: each handler waits at a
/// barrier only all ten together can pass.
#[tokio::test]
async fn as_many_deliveries_as_the_limit_are_served_at_once() {
    let barrier = &Barrier::new(10);
    let handled = &AtomicUsize::new(0);
    let served = timeout(
        PATIENCE,
        amqp_server::serve_deliveries(
            stream::iter(0_u8..10),
            NonZeroU16::new(10).unwrap(),
            pending(),
            ready(()),
            move |_| async move {
                barrier.wait().await;
                handled.fetch_add(1, Ordering::SeqCst);
            },
        ),
    )
    .await;
    assert_eq!(
        served,
        Ok(amqp_server::Stopped::ConsumerClosed),
        "the deliveries were not all in progress at once"
    );
    assert_eq!(
        handled.load(Ordering::SeqCst),
        10,
        "every delivery is handled"
    );
}

/// Nine deliveries under a limit of three meet three at a time at a barrier of three, and no more
/// than three are ever in progress.
#[tokio::test]
async fn never_more_deliveries_than_the_limit_are_served_at_once() {
    let barrier = &Barrier::new(3);
    let running = &AtomicUsize::new(0);
    let peak = &AtomicUsize::new(0);
    let served = timeout(
        PATIENCE,
        amqp_server::serve_deliveries(
            stream::iter(0_u8..9),
            NonZeroU16::new(3).unwrap(),
            pending(),
            ready(()),
            move |_| async move {
                let now = running.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(now, Ordering::SeqCst);
                barrier.wait().await;
                running.fetch_sub(1, Ordering::SeqCst);
            },
        ),
    )
    .await;
    assert_eq!(
        served,
        Ok(amqp_server::Stopped::ConsumerClosed),
        "the deliveries were not served three at a time"
    );
    assert_eq!(
        peak.load(Ordering::SeqCst),
        3,
        "the limit is reached and never passed"
    );
}

/// The first of two deliveries waits for the second to finish, so the second is finished, and
/// acknowledged, first.
#[tokio::test]
async fn a_later_delivery_finishes_before_an_earlier_one() {
    let second_finished = &Notify::new();
    let events = &Mutex::new(Vec::new());
    let served = timeout(
        PATIENCE,
        amqp_server::serve_deliveries(
            stream::iter(0_u8..2),
            NonZeroU16::new(2).unwrap(),
            pending(),
            ready(()),
            move |item| async move {
                if item == 0 {
                    second_finished.notified().await;
                } else {
                    second_finished.notify_one();
                }
                record(events, &format!("finish {item}"));
            },
        ),
    )
    .await;
    assert_eq!(
        served,
        Ok(amqp_server::Stopped::ConsumerClosed),
        "the first delivery held up the second"
    );
    assert_eq!(
        recorded(events),
        ["finish 1", "finish 0"],
        "each finishes in its own time"
    );
}

/// Shutdown arrives while both slots are busy and a third delivery is waiting: the consumer is
/// stopped at once, the third is never started, and the call returns only once both started
/// deliveries have finished.
#[tokio::test]
async fn shutdown_cancels_at_once_starts_nothing_new_and_drains_what_started() {
    let events = &Mutex::new(Vec::new());
    let both_started = &Barrier::new(3);
    let releases = &[Notify::new(), Notify::new()];
    let stopped = &Notify::new();
    let (shut, shutdown) = oneshot::channel::<()>();
    let serving = async move {
        let served = amqp_server::serve_deliveries(
            stream::iter(0_u8..3).chain(stream::pending()),
            NonZeroU16::new(2).unwrap(),
            async move {
                shutdown.await.unwrap();
            },
            async move {
                record(events, "stop");
                stopped.notify_one();
            },
            move |item| async move {
                record(events, &format!("start {item}"));
                both_started.wait().await;
                releases[usize::from(item)].notified().await;
                record(events, &format!("finish {item}"));
            },
        )
        .await;
        record(events, "returned");
        served
    };
    let shutting_down = async move {
        both_started.wait().await;
        shut.send(()).unwrap();
        stopped.notified().await;
        assert_eq!(
            recorded(events),
            ["start 0", "start 1", "stop"],
            "the consumer is stopped before either started delivery finishes"
        );
        releases[0].notify_one();
        releases[1].notify_one();
    };
    let (served, ()) = timeout(PATIENCE, async { tokio::join!(serving, shutting_down) })
        .await
        .unwrap();
    assert_eq!(
        served,
        amqp_server::Stopped::ShutdownRequested,
        "shutdown is what stopped it"
    );
    let mut seen = recorded(events);
    seen[3..5].sort();
    assert_eq!(
        seen,
        [
            "start 0", "start 1", "stop", "finish 0", "finish 1", "returned"
        ],
        "the third delivery never starts, and the call returns after both started ones finish"
    );
}

/// The deliveries end while three are still in progress: all three finish before the call returns
/// `ConsumerClosed`, and the consumer, which the broker already closed, is not stopped.
#[tokio::test]
async fn a_consumer_that_closes_drains_what_started_and_is_not_stopped() {
    let events = &Mutex::new(Vec::new());
    let served = timeout(PATIENCE, async move {
        let served = amqp_server::serve_deliveries(
            stream::iter(0_u8..3),
            NonZeroU16::new(3).unwrap(),
            pending(),
            async move { record(events, "stop") },
            move |item| async move {
                yield_now().await;
                record(events, &format!("finish {item}"));
            },
        )
        .await;
        record(events, "returned");
        served
    })
    .await;
    assert_eq!(
        served,
        Ok(amqp_server::Stopped::ConsumerClosed),
        "the deliveries ending is what stopped it"
    );
    let mut seen = recorded(events);
    seen[..3].sort();
    assert_eq!(
        seen,
        ["finish 0", "finish 1", "finish 2", "returned"],
        "every started delivery finishes first, and nothing is stopped"
    );
}
