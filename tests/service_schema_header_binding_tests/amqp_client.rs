//! The client, expanded out of the transport's macro into a module this harness names.
//!
//! The `use` is what resolves the types the author declared: the macro spells them exactly as they
//! were written, no crate prefix being true of either.

use crate::tests::{DocumentError, VersionResponse};

document_service_amqp_rpc_client!();
