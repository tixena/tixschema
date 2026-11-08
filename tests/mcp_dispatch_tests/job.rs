//! `JobService`: an operation that starts a task, the operations that report, cancel and answer
//! it, and the stand-in that keeps the tasks.

use alloc::sync::Arc;
use core::future::{Future, ready};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tixschema::{model_schema, service_schema};

pub const STARTED: &str = "2026-11-01T20:00:00Z";
pub const FINISHED: &str = "2026-11-01T20:04:10Z";

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum JobError {
    NotFound,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExportRequest {
    pub folder: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExportReport {
    pub files: u32,
}

/// Runs the long exports of a workspace.
#[service_schema(transports = ["mcp"])]
pub trait JobService<Ctx> {
    /// Answers a question an export asked.
    #[service_schema_op(mcp(scopes = ["jobs:answer"]))]
    async fn answer_export(
        &self,
        ctx: &Ctx,
        task_id: String,
        key: String,
        action: String,
        content: Option<String>,
    ) -> Result<(), JobError>;

    /// Asks one export to stop.
    #[service_schema_op(mcp(scopes = ["jobs:stop"]))]
    async fn cancel_export(&self, ctx: &Ctx, task_id: String) -> Result<(), JobError>;

    /// Tells how many exports ran.
    async fn count_exports(&self, ctx: &Ctx) -> Result<ExportReport, JobError>;

    /// Answers where one export stands.
    #[service_schema_op(mcp(scopes = ["jobs:read"]))]
    async fn export_status(
        &self,
        ctx: &Ctx,
        task_id: String,
    ) -> Result<job_service_schema::McpTaskState<ExportReport>, JobError>;

    /// Starts the export of one folder.
    #[service_schema_op(mcp(task(get = "export_status", cancel = "cancel_export", update = "answer_export"), scopes = ["jobs:run"]))]
    async fn start_export(
        &self,
        ctx: &Ctx,
        req: ExportRequest,
    ) -> Result<job_service_schema::McpTask, JobError>;
}

/// Keeps the tasks of the stand-in: `7f3a` works on its first poll and completes on its second,
/// `needs` waits for an answer, `broken` failed and `gone` is unknown.
#[derive(Clone, Default)]
pub struct Jobs {
    pub polls: Arc<Mutex<u32>>,
    pub reached: Arc<Mutex<Vec<String>>>,
}

impl Jobs {
    fn reach(&self, call: String) {
        self.reached.lock().unwrap().push(call);
    }

    fn task(id: &str) -> job_service_schema::McpTask {
        job_service_schema::McpTask {
            task_id: id.to_owned(),
            created_at: STARTED.to_owned(),
            last_updated_at: STARTED.to_owned(),
            ttl_ms: Some(3_600_000),
            poll_interval_ms: Some(2000),
        }
    }
}

// Every method below is synchronous work wrapped in an already-ready `Future`: nothing here waits
// on anything.
impl JobService<()> for Jobs {
    fn answer_export(
        &self,
        _ctx: &(),
        task_id: String,
        key: String,
        action: String,
        content: Option<String>,
    ) -> impl Future<Output = Result<(), JobError>> {
        self.reach(format!(
            "answer-export {task_id} {key} {action} {content:?}"
        ));
        ready(if task_id == "gone" {
            Err(JobError::NotFound)
        } else {
            Ok(())
        })
    }

    fn cancel_export(
        &self,
        _ctx: &(),
        task_id: String,
    ) -> impl Future<Output = Result<(), JobError>> {
        self.reach(format!("cancel-export {task_id}"));
        ready(if task_id == "gone" {
            Err(JobError::NotFound)
        } else {
            Ok(())
        })
    }

    fn count_exports(&self, _ctx: &()) -> impl Future<Output = Result<ExportReport, JobError>> {
        self.reach("count-exports".to_owned());
        ready(Ok(ExportReport { files: 1 }))
    }

    fn export_status(
        &self,
        _ctx: &(),
        task_id: String,
    ) -> impl Future<Output = Result<job_service_schema::McpTaskState<ExportReport>, JobError>>
    {
        use job_service_schema::McpTaskState;
        self.reach(format!("export-status {task_id}"));
        let mut polls = self.polls.lock().unwrap();
        *polls += 1;
        let polled = *polls;
        drop(polls);
        ready(match task_id.as_str() {
            "7f3a" if polled < 2 => Ok(McpTaskState::Working {
                task: Self::task(&task_id),
            }),
            "7f3a" => {
                let mut task = Self::task(&task_id);
                FINISHED.clone_into(&mut task.last_updated_at);
                Ok(McpTaskState::Completed {
                    task,
                    result: ExportReport { files: 3 },
                })
            }
            "needs" => Ok(McpTaskState::InputRequired {
                task: Self::task(&task_id),
                input_requests: [(
                    "overwrite".to_owned(),
                    serde_json::json!({
                        "method": "elicitation/create",
                        "params": {
                            "mode": "form",
                            "message": "Overwrite the folder?",
                            "requestedSchema": { "type": "object", "properties": {} },
                        },
                    }),
                )]
                .into(),
            }),
            "broken" => Ok(McpTaskState::Failed {
                task: Self::task(&task_id),
                error: job_service_schema::McpTaskError {
                    code: -32603,
                    message: "the folder vanished".to_owned(),
                    data: None,
                },
            }),
            "stopped" => Ok(McpTaskState::Cancelled {
                task: Self::task(&task_id),
            }),
            _ => Err(JobError::NotFound),
        })
    }

    fn start_export(
        &self,
        _ctx: &(),
        req: ExportRequest,
    ) -> impl Future<Output = Result<job_service_schema::McpTask, JobError>> {
        self.reach(format!("start-export {}", req.folder));
        ready(if req.folder == "missing" {
            Err(JobError::NotFound)
        } else {
            Ok(Self::task("7f3a"))
        })
    }
}
