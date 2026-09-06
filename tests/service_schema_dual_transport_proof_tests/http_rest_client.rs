//! The `http_rest` client, expanded out of the transport's macro into a module this harness names.
//!
//! Unlike `amqp_rpc`'s client, which decodes generically and so never spells a success or an error
//! type by name, this one reads a response body straight into the operation's own declared types,
//! both spelled bare — the author's own types, resolved where this macro is invoked rather than
//! where the service was declared.

use crate::tests::{
    ArchiveError, GetVersionError, GetVersionRequest, SweepError, SweepReport, ThumbnailError,
    VersionResponse,
};

document_service_http_rest_client!();
