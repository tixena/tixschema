//! The `http_rest` dispatcher, expanded out of the transport's macro into a module this harness
//! names.
//!
//! Unlike `amqp_rpc`'s dispatcher, which never spells an operation's error type by name (it answers
//! generically through `Answered<T, E>`), `http_rest`'s dispatcher builds each declared error's own
//! status table, and that table names the error type exactly as the author wrote it — bare, since
//! it is the author's own type rather than one this crate generated. A bare name resolves where the
//! macro is *invoked*, so every operation's error type that carries an `error_status` table has to
//! be in scope here, precisely as a client placement already needs the messages it sends.

use crate::tests::{
    ArchiveError, CreateDocumentError, ExplodeError, GetVersionError, SearchError, ThumbnailError,
};

document_service_http_rest_dispatcher!();
