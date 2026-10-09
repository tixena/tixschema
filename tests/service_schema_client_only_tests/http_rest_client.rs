//! The `http_rest` client, expanded out of the transport's macro into a module this harness names.

use crate::tests::{
    CreateDocumentError, CreateDocumentRequest, CreateDocumentResponse, GetVersionError,
    ThumbnailError, VersionResponse,
};

document_client_service_http_rest_client!();
