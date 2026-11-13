//! A `FrameSession` call its caller drops, read from inside the module the client expands into,
//! the one place the session's correlation map can be read: the dropped call leaves nothing in it.

use super::{DocumentSessionClient, FrameSession};
use crate::tests::WatchRequestDraft;
use core::future::{Future as _, pending};
use core::pin::pin;
use core::task::{Context as PollContext, Waker};

/// How many requests the session holds a slot for.
fn waiting(session: &FrameSession) -> usize {
    session.inner.pending.lock().unwrap().len()
}

/// Polls a `watch` call once, while its slot is held, then drops it.
fn dropped_after_one_poll(session: &FrameSession) {
    let client = DocumentSessionClient::new(session.clone());
    let mut call = pin!(
        client.watch(
            WatchRequestDraft {
                document_id: "doc-1".to_owned(),
            }
            .validate()
            .unwrap()
        )
    );
    assert!(
        call.as_mut()
            .poll(&mut PollContext::from_waker(Waker::noop()))
            .is_pending()
    );
    assert_eq!(waiting(session), 1);
}

#[test]
fn a_call_dropped_while_it_waits_for_its_reply_takes_its_slot_back_out() {
    let session = FrameSession::new(|_| async { Ok(()) });
    dropped_after_one_poll(&session);
    assert_eq!(waiting(&session), 0);
}

#[test]
fn a_call_dropped_while_its_frame_is_still_being_sent_takes_its_slot_back_out() {
    let session = FrameSession::new(|_| pending::<Result<(), String>>());
    dropped_after_one_poll(&session);
    assert_eq!(waiting(&session), 0);
}
