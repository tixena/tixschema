//! `<Service>Schema::openapi()`: one service per operation shape, each document read for the
//! paths, parameters, bodies, statuses and headers its service declares, and validated against
//! the `OpenAPI` 3.1 schema.

#[cfg(test)]
#[macro_use]
#[path = "openapi_tests/tests.rs"]
mod tests;

#[cfg(all(test, feature = "serde"))]
#[path = "openapi_tests/folder_http_rest_transport.rs"]
mod folder_http_rest_transport;

#[cfg(all(test, feature = "serde"))]
#[path = "openapi_tests/search_http_rest_transport.rs"]
mod search_http_rest_transport;

#[cfg(all(test, feature = "serde"))]
#[path = "openapi_tests/content_http_rest_transport.rs"]
mod content_http_rest_transport;

#[cfg(all(test, feature = "serde"))]
#[path = "openapi_tests/plain_http_rest_transport.rs"]
mod plain_http_rest_transport;

// What a service generates reaches the trait, its module and, for the document, its registry
// through `$crate`, this binary's root.
#[cfg(all(test, feature = "serde"))]
use tests::{
    ContentService, ContentServiceSchema, DocumentService, FolderService, FolderServiceSchema,
    PlainService, RenamedMemberService, SearchService, SearchServiceSchema, UploadService,
    VersionService, content_service_schema, document_service_schema, folder_service_schema,
    plain_service_schema, renamed_member_service_schema, search_service_schema,
    upload_service_schema, version_service_schema,
};
