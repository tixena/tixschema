//! The `http_rest` client for `ContentClientService`, expanded out of the transport's macro into a
//! module of its own - the same placement every other transport macro in this harness gets.

// `content_client_service_schema` is named too: the client's generated signature spells the success
// type through it, as the trait wrote it.
use crate::stream_service::{ContentError, ContentRangeError, content_client_service_schema};

content_client_service_http_rest_client!();
