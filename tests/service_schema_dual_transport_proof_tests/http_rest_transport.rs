//! The `http_rest` dispatcher, expanded out of the transport's macro into a module this harness
//! names.

use crate::shadowing::{Box, Clone, Default, Err, None, Ok, Send, Sized, Some, Sync};
use crate::tests::{ArchiveErrorDraft, GetVersionErrorDraft, RangeErrorDraft, ThumbnailErrorDraft};

document_service_http_rest_dispatcher!();

shadowing_names_built!();
