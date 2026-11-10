//! The `amqp_rpc` client of `DeskService`, in a public module.

use crate::desk::{
    SearchReport, Ticket, TicketCount, TicketError, TicketLookup, desk_service_schema,
};

desk_service_amqp_rpc_client!();
