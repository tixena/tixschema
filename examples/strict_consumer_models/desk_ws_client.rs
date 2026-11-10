//! The `ws_rpc` client of `DeskService`, in a public module.

use crate::desk::{
    SearchReport, Ticket, TicketCount, TicketError, TicketLookup, desk_service_schema,
};

desk_service_ws_rpc_client!();
