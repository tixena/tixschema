//! The client of `JobService`, expanded out of the transport's macro.

use crate::job::{ExportReport, ExportRequest, JobError};
use crate::job_service_schema;

job_service_mcp_client!();
