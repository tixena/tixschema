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

/// Looks tickets up at a desk.
#[service_schema(non_exhaustive, transports = ["mcp"])]
pub trait DeskService<Ctx> {
    /// Answers the ticket a number names.
    async fn find_ticket(&self, ctx: &Ctx, req: TicketLookup) -> Result<Ticket, TicketError>;
}
