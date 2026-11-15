//! `StatusService`: one operation per fault, each fault declared above the service, below it or
//! in another crate, with a draft or without one.

use alloc::string::String;
use core::future::ready;
use fault_variants_apart::{DraftedApart, DraftedApartDraft, PlainApart};
use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

/// A fault with a draft, declared above the service.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum DraftedAbove {
    Closed { reason: String },
    NotFound,
}

/// A fault that is its own valid type, declared above the service.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlainAbove {
    Closed,
    NotFound,
}

/// One operation per fault, each answering `404` for `NotFound` and `410` for `Closed`.
#[service_schema(transports = ["http_rest"])]
pub trait StatusService<Ctx> {
    #[service_schema_op(http(
        method = "GET",
        path = "/drafted-above/{which}",
        error_status(NotFound = 404, Closed = 410),
    ))]
    async fn drafted_above(&self, ctx: &Ctx, which: String) -> Result<u32, DraftedAbove>;

    #[service_schema_op(http(
        method = "GET",
        path = "/drafted-apart/{which}",
        error_status(NotFound = 404, Closed = 410, NeedsDrive = 412),
    ))]
    async fn drafted_apart(&self, ctx: &Ctx, which: String) -> Result<u32, DraftedApart>;

    #[service_schema_op(http(
        method = "GET",
        path = "/drafted-below/{which}",
        error_status(NotFound = 404, Closed = 410),
    ))]
    async fn drafted_below(&self, ctx: &Ctx, which: String) -> Result<u32, DraftedBelow>;

    #[service_schema_op(http(
        method = "GET",
        path = "/plain-above/{which}",
        error_status(NotFound = 404, Closed = 410),
    ))]
    async fn plain_above(&self, ctx: &Ctx, which: String) -> Result<u32, PlainAbove>;

    #[service_schema_op(http(
        method = "GET",
        path = "/plain-apart/{which}",
        error_status(NotFound = 404, Closed = 410),
    ))]
    async fn plain_apart(&self, ctx: &Ctx, which: String) -> Result<u32, PlainApart>;

    #[service_schema_op(http(
        method = "GET",
        path = "/plain-below/{which}",
        error_status(NotFound = 404, Closed = 410),
    ))]
    async fn plain_below(&self, ctx: &Ctx, which: String) -> Result<u32, PlainBelow>;
}

/// A fault with a draft, declared below the service.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum DraftedBelow {
    Closed { reason: String },
    NotFound,
}

/// A fault that is its own valid type, declared below the service.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlainBelow {
    Closed,
    NotFound,
}

/// Answers `NotFound` for `not-found` and `Closed` for any other text.
pub struct StatusBackEnd;

impl StatusService<()> for StatusBackEnd {
    async fn drafted_above(&self, _ctx: &(), which: String) -> Result<u32, DraftedAbove> {
        ready(()).await;
        Err(if which == "not-found" {
            DraftedAbove::NotFound
        } else {
            DraftedAboveDraft::Closed {
                reason: "shut".to_owned(),
            }
            .into()
        })
    }

    async fn drafted_apart(&self, _ctx: &(), which: String) -> Result<u32, DraftedApart> {
        ready(()).await;
        Err(if which == "not-found" {
            DraftedApart::NotFound
        } else {
            DraftedApartDraft::Closed {
                reason: "shut".to_owned(),
            }
            .into()
        })
    }

    async fn drafted_below(&self, _ctx: &(), which: String) -> Result<u32, DraftedBelow> {
        ready(()).await;
        Err(if which == "not-found" {
            DraftedBelow::NotFound
        } else {
            DraftedBelowDraft::Closed {
                reason: "shut".to_owned(),
            }
            .into()
        })
    }

    async fn plain_above(&self, _ctx: &(), which: String) -> Result<u32, PlainAbove> {
        ready(()).await;
        Err(if which == "not-found" {
            PlainAbove::NotFound
        } else {
            PlainAbove::Closed
        })
    }

    async fn plain_apart(&self, _ctx: &(), which: String) -> Result<u32, PlainApart> {
        ready(()).await;
        Err(if which == "not-found" {
            PlainApart::NotFound
        } else {
            PlainApart::Closed
        })
    }

    async fn plain_below(&self, _ctx: &(), which: String) -> Result<u32, PlainBelow> {
        ready(()).await;
        Err(if which == "not-found" {
            PlainBelow::NotFound
        } else {
            PlainBelow::Closed
        })
    }
}
