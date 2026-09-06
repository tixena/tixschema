//! The `http_rest` client for `ContentClientService`, expanded out of the transport's macro into a
//! module of its own - the same placement every other transport macro in this harness gets.

// `content_client_service_schema` is named here, not just `ContentError`: the client's own
// generated method signature spells the operation's declared success type bare - here
// `content_client_service_schema::StreamedAnswer`, exactly as the trait wrote it - and a
// module-qualified success type needs the module itself importable, not only the error it names
// beside it.
use crate::stream_service::{ContentError, content_client_service_schema};

content_client_service_http_rest_client!();
