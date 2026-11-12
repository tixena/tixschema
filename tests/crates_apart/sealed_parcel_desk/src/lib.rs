//! A crate apart from the test binaries that declares a service under `non_exhaustive` and places
//! its `http_rest` dispatcher and its `amqp_rpc` server, so a test binary builds the types they
//! seal from another crate. It holds them where its `service` feature is on, which tixschema's
//! `serde` turns on.

#![cfg(feature = "service")]
#![deny(warnings)]

use core::future::{Future, ready};

use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

/// The shelf a count names.
#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct ParcelQuery {
    /// The shelf's name.
    pub shelf: String,
}

/// How many parcels a shelf holds.
#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct ParcelTally {
    /// The number of parcels.
    pub parcels: u32,
}

/// Why a count found no shelf.
#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum TallyError {
    /// No shelf has the name.
    NoSuchShelf,
}

/// Counts the parcels on a shelf.
#[service_schema(non_exhaustive, transports = ["amqp_rpc", "http_rest"])]
pub trait ParcelDesk<Ctx> {
    /// Answers how many parcels the shelf a query names holds.
    async fn count_parcels(&self, ctx: &Ctx, req: ParcelQuery) -> Result<ParcelTally, TallyError>;
}

/// The `http_rest` dispatcher of `ParcelDesk`.
pub mod parcel_http_dispatcher {
    parcel_desk_http_rest_dispatcher!();
}

/// The `amqp_rpc` server of `ParcelDesk`.
pub mod parcel_amqp_server {
    parcel_desk_amqp_rpc_server!();
}

/// A desk whose one shelf, `north`, holds three parcels.
pub struct Shelves;

impl<Ctx: Sync> ParcelDesk<Ctx> for Shelves {
    fn count_parcels(
        &self,
        _ctx: &Ctx,
        req: ParcelQuery,
    ) -> impl Future<Output = Result<ParcelTally, TallyError>> + Send {
        ready(if req.shelf == "north" {
            Ok(ParcelTallyDraft { parcels: 3 }.validate().unwrap())
        } else {
            Err(TallyErrorDraft::NoSuchShelf.validate().unwrap())
        })
    }
}

/// Answers `request` through the default fault handler, written bare, as the crate that placed the
/// dispatcher writes it.
pub fn answered_by_default(
    request: &parcel_http_dispatcher::IncomingRequest,
) -> impl Future<Output = parcel_http_dispatcher::OutgoingResponse> + Send {
    parcel_http_dispatcher::dispatch(
        &Shelves,
        &(),
        request,
        &parcel_http_dispatcher::DefaultFaultHandler,
    )
}
