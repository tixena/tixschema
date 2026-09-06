//! The `amqp_rpc` client, expanded out of the transport's macro into a module this harness names.
//!
//! The `use` is what resolves the types the author declared: the macro spells them exactly as they
//! were written, no crate prefix.

use crate::tests::{
    ArchiveError, GetVersionError, GetVersionRequest, SweepError, SweepReport, ThumbnailError,
    VersionResponse,
};

document_service_amqp_rpc_client!();
