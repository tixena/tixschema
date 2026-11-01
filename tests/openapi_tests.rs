//! `<Service>Schema::openapi()`: one service per operation shape, each document read for the
//! paths, parameters, bodies, statuses and headers its service declares, and validated against
//! the `OpenAPI` 3.1 schema.

#[cfg(test)]
#[path = "openapi_tests/tests.rs"]
mod tests;

// What a service generates reaches the trait and its module through `$crate`, this binary's root.
#[cfg(all(test, feature = "serde"))]
use tests::{
    ContentService, DocumentService, FolderService, PlainService, SearchService, UploadService,
    VersionService, content_service_schema, document_service_schema, folder_service_schema,
    plain_service_schema, search_service_schema, upload_service_schema, version_service_schema,
};
