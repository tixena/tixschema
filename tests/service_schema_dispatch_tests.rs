//! The dispatcher `#[service_schema]` generates, driven end to end: a probe service, a probe reply
//! handle that writes down how each message was settled, and a payload for every path through an
//! arm.

#[cfg(test)]
#[macro_use]
#[path = "service_schema_dispatch_tests/tests.rs"]
mod tests;

#[cfg(all(test, feature = "serde"))]
#[path = "service_schema_dispatch_tests/amqp_transport.rs"]
mod amqp_transport;

/// The same macro again, in a second module of its own: two dispatchers for one service in one
/// crate, which is what the macro emitting bare items rather than a module of its own is for.
#[cfg(all(test, feature = "serde"))]
#[path = "service_schema_dispatch_tests/second_amqp_transport.rs"]
mod second_amqp_transport;

#[cfg(all(
    test,
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[path = "service_schema_dispatch_tests/gate_amqp_transport.rs"]
mod gate_amqp_transport;

/// The `http_rest` dispatcher, in a module of its own — the same placement rules apply to this
/// transport's macro as to `amqp_rpc`'s.
#[cfg(all(test, feature = "serde"))]
#[path = "service_schema_dispatch_tests/http_rest_transport.rs"]
mod http_rest_transport;

/// `ContentService`, its `body = "stream"` operation and the tests driving it - a service of its
/// own rather than an operation added to `DocumentService`, so a streamed operation's own
/// `OutgoingResponse`/`OutgoingBody` shape never reaches a body kind this file already exercises.
#[cfg(test)]
#[macro_use]
#[path = "service_schema_dispatch_tests/stream_service.rs"]
mod stream_service;

/// The `http_rest` dispatcher for `ContentService`, in a module of its own.
#[cfg(all(test, feature = "serde"))]
#[path = "service_schema_dispatch_tests/stream_http_rest_transport.rs"]
mod stream_http_rest_transport;

/// `UploadService`, its `body = "multipart"` operation and the tests driving it - a service of
/// its own rather than an operation added to `DocumentService`, so a multipart request's own
/// extra `parts` argument never reaches a body kind this file already exercises.
#[cfg(test)]
#[macro_use]
#[path = "service_schema_dispatch_tests/multipart_service.rs"]
mod multipart_service;

/// The `http_rest` dispatcher for `UploadService`, in a module of its own.
#[cfg(all(test, feature = "serde"))]
#[path = "service_schema_dispatch_tests/multipart_http_rest_transport.rs"]
mod multipart_http_rest_transport;

/// `SoloPathBoundService`, its fully path-bound `GET` and the tests driving it - the only
/// operation in the service, so its dispatcher must compile with no `parse_query` reachable at
/// all.
#[cfg(test)]
#[macro_use]
#[path = "service_schema_dispatch_tests/solo_path_bound_service.rs"]
mod solo_path_bound_service;

/// The `http_rest` dispatcher for `SoloPathBoundService`, in a module of its own.
#[cfg(all(test, feature = "serde"))]
#[path = "service_schema_dispatch_tests/solo_path_bound_http_rest_transport.rs"]
mod solo_path_bound_http_rest_transport;

/// `PathBoundBesideQueryService`, the same fully path-bound `GET` beside one that reads the
/// query, and the tests driving both - a service of its own rather than an operation added to
/// `SoloPathBoundService`, so the "no query reader at all" service above keeps proving that shape
/// undiluted.
#[cfg(test)]
#[macro_use]
#[path = "service_schema_dispatch_tests/path_bound_beside_query_service.rs"]
mod path_bound_beside_query_service;

/// The `http_rest` dispatcher for `PathBoundBesideQueryService`, in a module of its own.
#[cfg(all(test, feature = "serde"))]
#[path = "service_schema_dispatch_tests/path_bound_beside_query_http_rest_transport.rs"]
mod path_bound_beside_query_http_rest_transport;

/// `ScalarPathBoundService`, whose one operation's one argument already is the message - a
/// scalar the path's one placeholder binds whole - and the tests driving it. A service of its
/// own rather than an operation added to `DocumentService`, so its one route cannot collide with
/// another operation's own path there.
#[cfg(test)]
#[macro_use]
#[path = "service_schema_dispatch_tests/scalar_path_bound_service.rs"]
mod scalar_path_bound_service;

/// The `http_rest` dispatcher for `ScalarPathBoundService`, in a module of its own.
#[cfg(all(test, feature = "serde"))]
#[path = "service_schema_dispatch_tests/scalar_path_bound_http_rest_transport.rs"]
mod scalar_path_bound_http_rest_transport;

// A transport's dispatcher reaches what the service declared through `$crate`, which is this
// binary's root: a service written in a submodule is named here for the expansion to resolve.
#[cfg(all(test, feature = "serde"))]
use multipart_service::{UploadService, upload_service_schema};
#[cfg(all(test, feature = "serde"))]
use path_bound_beside_query_service::{
    PathBoundBesideQueryService, path_bound_beside_query_service_schema,
};
#[cfg(all(test, feature = "serde"))]
use scalar_path_bound_service::{ScalarPathBoundService, scalar_path_bound_service_schema};
#[cfg(all(test, feature = "serde"))]
use solo_path_bound_service::{SoloPathBoundService, solo_path_bound_service_schema};
#[cfg(all(test, feature = "serde"))]
use stream_service::{ContentService, content_service_schema};
#[cfg(all(
    test,
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
use tests::a_message_annotated_with_a_constraint::{GateService, gate_service_schema};
#[cfg(all(test, feature = "serde"))]
use tests::{DocumentService, ProbeService, document_service_schema, probe_service_schema};
