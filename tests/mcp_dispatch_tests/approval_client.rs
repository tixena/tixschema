//! The client of `ApprovalService`, expanded out of the transport's macro.

use crate::approval::{
    DeleteConfirmation, DeleteError, ImportError, ImportedDocument, PreviewError, ReportRequest,
    ShippingForm,
};
use crate::approval_service_schema;

approval_service_mcp_client!();
