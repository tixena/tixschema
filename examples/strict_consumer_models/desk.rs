//! A service another crate reads, asking for `mcp`.

use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

/// What a lookup names a ticket by.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
#[non_exhaustive]
pub struct TicketLookup {
    /// The number printed on the ticket.
    pub number: String,
}

/// A ticket the desk holds.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
#[non_exhaustive]
pub struct Ticket {
    /// The number printed on the ticket.
    pub number: String,
}

/// Why a lookup found no ticket.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
#[non_exhaustive]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum TicketError {
    /// No ticket has the number.
    NotFound,
}

/// What a finished search holds.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
#[non_exhaustive]
pub struct SearchReport {
    /// How many tickets matched.
    pub matches: u32,
}

/// Looks tickets up at a desk.
#[service_schema(non_exhaustive, transports = ["mcp"])]
pub trait DeskService<Ctx> {
    /// Answers a question a search asked.
    #[service_schema_op(mcp(scopes = ["desk:answer"]))]
    async fn answer_search(
        &self,
        ctx: &Ctx,
        task_id: String,
        key: String,
        action: String,
        content: Option<String>,
    ) -> Result<(), TicketError>;

    /// Asks one search to stop.
    #[service_schema_op(mcp(scopes = ["desk:stop"]))]
    async fn cancel_search(&self, ctx: &Ctx, task_id: String) -> Result<(), TicketError>;

    /// Answers the ticket a number names.
    async fn find_ticket(&self, ctx: &Ctx, req: TicketLookup) -> Result<Ticket, TicketError>;

    /// Answers where one search stands.
    #[service_schema_op(mcp(scopes = ["desk:read"]))]
    async fn search_status(
        &self,
        ctx: &Ctx,
        task_id: String,
    ) -> Result<desk_service_schema::McpTaskState<SearchReport>, TicketError>;

    /// Starts a search for the tickets a number names.
    #[service_schema_op(mcp(task(get = "search_status", cancel = "cancel_search", update = "answer_search"), scopes = ["desk:run"]))]
    async fn start_search(
        &self,
        ctx: &Ctx,
        req: TicketLookup,
    ) -> Result<desk_service_schema::McpTask, TicketError>;
}
