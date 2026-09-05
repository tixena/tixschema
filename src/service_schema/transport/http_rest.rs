//! The `http_rest` transport, registered but not yet emitting anything.
//!
//! `#[service_schema_op(http(...))]` is parsed already: every operation's
//! [`crate::service_schema::parse::OperationDef::http`] carries the whole shape, method through
//! header bindings, so nothing here needs to read the trait again. This module exists so
//! [`super::Transport::emit`]'s match is answered for [`Transport::HttpRest`] without publishing
//! anything a consumer could place — the dispatcher, the route table and the client are separate,
//! later work.

use super::Transport;
use crate::service_schema::parse::ServiceDef;
use proc_macro2::TokenStream;

/// Nothing, for now — see the module doc.
pub fn emit(_service: &ServiceDef, _transport: Transport) -> TokenStream {
    TokenStream::new()
}
