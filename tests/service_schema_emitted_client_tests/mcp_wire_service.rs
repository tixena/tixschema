//! `ShelfWireService`, whose operations answer content blocks and prompt messages, asked for on
//! every transport.

use core::future::{Future, ready};
use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum ShelfWireError {
    NotFound,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ShelfWireRequest {
    pub shelf: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ShelfCount {
    pub books: u32,
}

/// Describes the shelves of a library.
#[service_schema(transports = ["mcp", "ws_rpc", "http_rest", "amqp_rpc"])]
pub trait ShelfWireService<Ctx> {
    /// Answers where the count of one shelf stands.
    async fn count_status(
        &self,
        ctx: &Ctx,
        task_id: String,
    ) -> Result<shelf_wire_service_schema::McpTaskState<ShelfCount>, ShelfWireError>;

    /// Answers the blocks that describe one shelf.
    #[service_schema_op(mcp(content))]
    async fn describe_shelf(
        &self,
        ctx: &Ctx,
        req: ShelfWireRequest,
    ) -> Result<Vec<shelf_wire_service_schema::McpContent>, ShelfWireError>;

    /// Lists the shelves, one page at a time.
    #[service_schema_op(mcp(list_resources))]
    async fn list_shelves(
        &self,
        ctx: &Ctx,
        cursor: Option<String>,
    ) -> Result<shelf_wire_service_schema::McpResourcePage, ShelfWireError>;

    /// Guides the model through one shelf.
    #[service_schema_op(mcp(prompt))]
    async fn review_shelf(
        &self,
        ctx: &Ctx,
        shelf_name: String,
    ) -> Result<Vec<shelf_wire_service_schema::McpPromptMessage>, ShelfWireError>;

    /// Starts the count of one shelf.
    #[service_schema_op(mcp(task(get = "count_status")))]
    async fn start_count(
        &self,
        ctx: &Ctx,
        req: ShelfWireRequest,
    ) -> Result<shelf_wire_service_schema::McpTask, ShelfWireError>;
}

/// Answers every operation, or a fault for a shelf that is not the history shelf.
pub struct Stand;

impl ShelfWireService<()> for Stand {
    fn count_status(
        &self,
        _ctx: &(),
        task_id: String,
    ) -> impl Future<Output = Result<shelf_wire_service_schema::McpTaskState<ShelfCount>, ShelfWireError>>
    {
        ready(if task_id == "count-1" {
            Ok(shelf_wire_service_schema::McpTaskState::Completed {
                task: task(),
                result: ShelfCount { books: 11 },
            })
        } else {
            Err(ShelfWireError::NotFound)
        })
    }

    fn describe_shelf(
        &self,
        _ctx: &(),
        req: ShelfWireRequest,
    ) -> impl Future<Output = Result<Vec<shelf_wire_service_schema::McpContent>, ShelfWireError>>
    {
        ready(if req.shelf == "history" {
            Ok(blocks())
        } else {
            Err(ShelfWireError::NotFound)
        })
    }

    fn list_shelves(
        &self,
        _ctx: &(),
        cursor: Option<String>,
    ) -> impl Future<Output = Result<shelf_wire_service_schema::McpResourcePage, ShelfWireError>>
    {
        ready(match cursor {
            None => Ok(page()),
            Some(_) => Err(ShelfWireError::NotFound),
        })
    }

    fn review_shelf(
        &self,
        _ctx: &(),
        shelf_name: String,
    ) -> impl Future<Output = Result<Vec<shelf_wire_service_schema::McpPromptMessage>, ShelfWireError>>
    {
        ready(if shelf_name == "history" {
            Ok(messages())
        } else {
            Err(ShelfWireError::NotFound)
        })
    }

    fn start_count(
        &self,
        _ctx: &(),
        req: ShelfWireRequest,
    ) -> impl Future<Output = Result<shelf_wire_service_schema::McpTask, ShelfWireError>> {
        ready(if req.shelf == "history" {
            Ok(task())
        } else {
            Err(ShelfWireError::NotFound)
        })
    }
}

/// The task the count of the history shelf starts.
pub fn task() -> shelf_wire_service_schema::McpTask {
    shelf_wire_service_schema::McpTask {
        task_id: "count-1".to_owned(),
        created_at: "2026-07-28T09:00:00Z".to_owned(),
        last_updated_at: "2026-07-28T09:00:00Z".to_owned(),
        ttl_ms: None,
        poll_interval_ms: None,
    }
}

/// One block of each kind, which the history shelf is described with.
pub fn blocks() -> Vec<shelf_wire_service_schema::McpContent> {
    use shelf_wire_service_schema::{McpContent, McpResourceContents};
    vec![
        McpContent::Text {
            text: "Eleven books.".to_owned(),
        },
        McpContent::Image {
            data: vec![0x89, b'P', b'N', b'G'],
            mime_type: "image/png".to_owned(),
        },
        McpContent::Audio {
            data: vec![0xff, 0xfb, 0x90],
            mime_type: "audio/mpeg".to_owned(),
        },
        McpContent::ResourceLink {
            uri: "shelf://history".to_owned(),
            name: "history".to_owned(),
            title: Some("History".to_owned()),
            description: None,
            mime_type: Some("text/markdown".to_owned()),
        },
        McpContent::Resource {
            uri: "shelf://history/index".to_owned(),
            mime_type: Some("text/markdown".to_owned()),
            contents: McpResourceContents::Text("# History".to_owned()),
        },
        McpContent::Resource {
            uri: "shelf://history/cover".to_owned(),
            mime_type: None,
            contents: McpResourceContents::Blob(vec![9, 8, 7]),
        },
    ]
}

/// The one page of shelves the listing answers.
pub fn page() -> shelf_wire_service_schema::McpResourcePage {
    shelf_wire_service_schema::McpResourcePage {
        resources: vec![shelf_wire_service_schema::McpResource {
            uri: "shelf://history".to_owned(),
            name: "history".to_owned(),
            title: None,
            description: None,
            mime_type: Some("text/markdown".to_owned()),
            size: Some(11),
        }],
        next_cursor: Some("page-2".to_owned()),
    }
}

/// The two messages the history shelf is reviewed with.
pub fn messages() -> Vec<shelf_wire_service_schema::McpPromptMessage> {
    use shelf_wire_service_schema::{McpContent, McpPromptMessage, McpRole};
    vec![
        McpPromptMessage {
            role: McpRole::User,
            content: McpContent::Text {
                text: "Review the history shelf.".to_owned(),
            },
        },
        McpPromptMessage {
            role: McpRole::Assistant,
            content: McpContent::Text {
                text: "Which books?".to_owned(),
            },
        },
    ]
}
