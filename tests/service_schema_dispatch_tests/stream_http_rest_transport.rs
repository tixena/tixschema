//! The `http_rest` dispatcher for `ContentService`, expanded out of the transport's macro into a
//! module of its own - the same placement every other transport macro in this harness gets.

use crate::stream_service::ContentError;

content_service_http_rest_dispatcher!();
