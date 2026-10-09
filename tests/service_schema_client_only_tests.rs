//! A crate that places a client and no dispatcher, which is the half a caller of a service wants.

#![cfg(feature = "serde")]

#[cfg(test)]
#[macro_use]
#[path = "service_schema_client_only_tests/tests.rs"]
mod tests;

#[cfg(all(test, feature = "serde"))]
#[path = "service_schema_client_only_tests/amqp_client.rs"]
mod amqp_client;

#[cfg(all(test, feature = "serde"))]
#[path = "service_schema_client_only_tests/http_rest_client.rs"]
mod http_rest_client;

/// `ContentClientService`, its `body = "stream"` operation and the tests calling it - a service of
/// its own rather than an operation added to `DocumentClientService`, so a streamed operation's own
/// `IncomingResponse`/`IncomingBody` shape never reaches a body kind this file already exercises.
#[cfg(test)]
#[macro_use]
#[path = "service_schema_client_only_tests/stream_service.rs"]
mod stream_service;

#[cfg(all(test, feature = "serde"))]
#[path = "service_schema_client_only_tests/stream_http_rest_client.rs"]
mod stream_http_rest_client;

/// `UploadClientService`, its `body = "multipart"` operation and the tests calling it - a service
/// of its own rather than an operation added to `DocumentClientService`, so a multipart request's
/// own `OutgoingRequest::into_parts` never reaches a body kind this file already exercises.
#[cfg(test)]
#[macro_use]
#[path = "service_schema_client_only_tests/multipart_service.rs"]
mod multipart_service;

#[cfg(all(test, feature = "serde"))]
#[path = "service_schema_client_only_tests/multipart_http_rest_client.rs"]
mod multipart_http_rest_client;

// The client reaches what the service declared through `$crate`, which is this binary's root: the
// service's own module, which every message it sends is built through.
#[cfg(all(test, feature = "serde"))]
use multipart_service::{UploadClientService, upload_client_service_schema};
#[cfg(all(test, feature = "serde"))]
use stream_service::{ContentClientService, content_client_service_schema};
#[cfg(all(test, feature = "serde"))]
use tests::{
    CallService, DocumentClientService, call_service_schema, document_client_service_schema,
};
