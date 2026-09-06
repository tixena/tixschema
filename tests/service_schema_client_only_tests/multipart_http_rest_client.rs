//! The `http_rest` client for `UploadClientService`, expanded out of the transport's macro into a
//! module of its own - the same placement every other transport macro in this harness gets.

use crate::multipart_service::{UploadError, UploadResponse};
use crate::upload_client_service_schema;

upload_client_service_http_rest_client!();
