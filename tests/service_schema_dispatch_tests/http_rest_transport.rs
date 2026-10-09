//! The `http_rest` dispatcher, expanded out of the transport's macro into a module this harness
//! names.

use crate::tests::{
    ArchiveError, CreateDocumentError, ExplodeError, GetVersionError, SearchError, ThumbnailError,
    VaultError,
};

document_service_http_rest_dispatcher!();
