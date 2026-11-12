//! The `http_rest` dispatcher for `UploadService`, in a module of its own.

use crate::multipart_service::UploadErrorDraft;
use crate::upload_service_schema;

upload_service_http_rest_dispatcher!();
