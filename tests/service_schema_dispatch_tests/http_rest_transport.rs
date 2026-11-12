//! The `http_rest` dispatcher, expanded out of the transport's macro into a module this harness
//! names.

use crate::tests::{
    ArchiveErrorDraft, CreateDocumentErrorDraft, ExplodeErrorDraft, GetVersionErrorDraft,
    SearchErrorDraft, ThumbnailErrorDraft, VaultErrorDraft,
};

document_service_http_rest_dispatcher!();
