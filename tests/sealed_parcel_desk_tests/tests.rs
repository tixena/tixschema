use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context as PollContext, Poll, Waker};
use std::sync::Mutex;

use sealed_parcel_desk::parcel_amqp_server::{Context, IncomingMessage, Reply};
use sealed_parcel_desk::parcel_http_dispatcher::{
    DefaultFaultHandler, IncomingRequest, OutgoingResponse, dispatch,
};
use sealed_parcel_desk::{Shelves, answered_by_default, parcel_amqp_server, parcel_desk_schema};
use serde::Serialize;

/// Records what one message was answered with, in place of a broker.
#[derive(Default)]
struct ProbeReply {
    sent: Mutex<Vec<String>>,
}

impl Reply for ProbeReply {
    fn fault(&self, fault: parcel_desk_schema::ServiceFault) -> impl Future<Output = ()> + Send {
        self.sent.lock().unwrap().push(format!("fault: {fault:?}"));
        ready(())
    }

    fn send<T>(&self, value: T, _headers: Vec<(String, String)>) -> impl Future<Output = ()> + Send
    where
        T: Serialize + Send,
    {
        self.sent
            .lock()
            .unwrap()
            .push(serde_json::to_string(&value).unwrap());
        ready(())
    }
}

/// The dispatchers never suspend here, so one poll answers them; `None` says an assumption about
/// them stopped holding.
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

fn posted(path: &str, body: &str) -> IncomingRequest {
    IncomingRequest::new(
        "POST".to_owned(),
        path.to_owned(),
        String::new(),
        vec![("content-type".to_owned(), "application/json".to_owned())],
        body.as_bytes().to_vec(),
    )
}

fn answered(request: &IncomingRequest, handler: &DefaultFaultHandler) -> OutgoingResponse {
    poll_once(dispatch(&Shelves, &(), request, handler)).unwrap()
}

#[test]
fn another_crate_builds_the_sealed_default_fault_handler_through_new_and_default() {
    let counted = posted("/count-parcels", r#"{"shelf":"north"}"#);
    let unrouted = posted("/weigh-parcels", "{}");
    for handler in [DefaultFaultHandler::new(), DefaultFaultHandler::default()] {
        let answer = answered(&counted, &handler);
        assert_eq!(answer.status(), 200);
        assert_eq!(answer.body(), br#"{"parcels":3}"#);
        assert_eq!(answered(&unrouted, &handler).status(), 404);
    }
}

#[test]
fn the_crate_that_placed_the_dispatcher_still_writes_the_handler_bare() {
    let counted = posted("/count-parcels", r#"{"shelf":"north"}"#);
    let answer = poll_once(answered_by_default(&counted)).unwrap();
    assert_eq!(answer.status(), 200);
    assert_eq!(
        poll_once(answered_by_default(&posted("/weigh-parcels", "{}")))
            .unwrap()
            .status(),
        404
    );
}

#[test]
fn another_crate_builds_the_sealed_server_context_through_new() {
    let reply = ProbeReply::default();
    let ctx = Context::new(tracing::Span::none());
    assert!(ctx.logger.is_none());
    poll_once(parcel_amqp_server::dispatch(
        &Shelves,
        &ctx,
        &IncomingMessage::new(
            "count-parcels".to_owned(),
            br#"{"shelf":"north"}"#.to_vec(),
            Vec::new(),
        ),
        &reply,
    ))
    .unwrap();
    assert_eq!(
        *reply.sent.lock().unwrap(),
        [r#"{"ok":true,"value":{"parcels":3}}"#]
    );
}
