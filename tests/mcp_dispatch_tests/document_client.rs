//! The client, expanded out of the transport's macro into a module this harness names.
//!
//! The `use` is what resolves the types the author declared: the macro spells them exactly as
//! they were written.

use std::collections::HashMap;

use crate::tests::{DocumentId, DraftBatch, GetVersionRequest, VersionError, VersionResponse};

document_service_mcp_client!();
