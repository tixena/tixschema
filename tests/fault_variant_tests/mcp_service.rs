//! `AskingService`: one operation per fault, each asking the user with a form or a link, the
//! fault declared above the service, below it or in another crate, with a draft or without one.

use alloc::string::String;
use core::future::ready;
use fault_variants_apart::{DraftedApart, DraftedApartDraft, PlainApart};
use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

/// What the user is asked.
#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Confirmation {
    pub confirm: bool,
}

/// A fault with a draft, declared above the service, whose arm that asks holds a link.
#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum DraftedAsksAbove {
    NeedsDrive { message: String, url: String },
    NotFound,
}

/// A fault that is its own valid type, declared above the service.
#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlainAsksAbove {
    NeedsConfirmation,
    NotFound,
}

/// Asks the user before it goes on.
#[service_schema(transports = ["mcp"])]
pub trait AskingService<Ctx> {
    /// Asks the user before it goes on.
    #[service_schema_op(mcp(interview(arm = "NeedsConfirmation", answers = confirmation)))]
    async fn form_above(
        &self,
        ctx: &Ctx,
        document_id: String,
        confirmation: Option<Confirmation>,
    ) -> Result<(), PlainAsksAbove>;

    /// Asks the user before it goes on.
    #[service_schema_op(mcp(interview(arm = "Closed", answers = confirmation)))]
    async fn form_apart(
        &self,
        ctx: &Ctx,
        document_id: String,
        confirmation: Option<Confirmation>,
    ) -> Result<(), PlainApart>;

    /// Asks the user before it goes on.
    #[service_schema_op(mcp(interview(arm = "NeedsConfirmation", answers = confirmation)))]
    async fn form_below(
        &self,
        ctx: &Ctx,
        document_id: String,
        confirmation: Option<Confirmation>,
    ) -> Result<(), PlainAsksBelow>;

    /// Asks the user before it goes on.
    #[service_schema_op(mcp(interview(arm = "Closed", answers = confirmation)))]
    async fn form_drafted_apart(
        &self,
        ctx: &Ctx,
        document_id: String,
        confirmation: Option<Confirmation>,
    ) -> Result<(), DraftedApart>;

    /// Asks the user before it goes on.
    #[service_schema_op(mcp(interview(arm = "NeedsDrive", url)))]
    async fn link_above(&self, ctx: &Ctx, file_id: String) -> Result<u32, DraftedAsksAbove>;

    /// Asks the user before it goes on.
    #[service_schema_op(mcp(interview(arm = "NeedsDrive", url)))]
    async fn link_apart(&self, ctx: &Ctx, file_id: String) -> Result<u32, DraftedApart>;

    /// Asks the user before it goes on.
    #[service_schema_op(mcp(interview(arm = "NeedsDrive", url)))]
    async fn link_below(&self, ctx: &Ctx, file_id: String) -> Result<u32, DraftedAsksBelow>;
}

/// A fault with a draft, declared below the service, whose arm that asks holds a link.
#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum DraftedAsksBelow {
    NeedsDrive { message: String, url: String },
    NotFound,
}

/// A fault that is its own valid type, declared below the service.
#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlainAsksBelow {
    NeedsConfirmation,
    NotFound,
}

/// Answers every operation with its fault.
pub struct AskingBackEnd;

impl AskingService<()> for AskingBackEnd {
    async fn form_above(
        &self,
        _ctx: &(),
        _document_id: String,
        _confirmation: Option<Confirmation>,
    ) -> Result<(), PlainAsksAbove> {
        ready(()).await;
        Err(PlainAsksAbove::NotFound)
    }

    async fn form_apart(
        &self,
        _ctx: &(),
        _document_id: String,
        _confirmation: Option<Confirmation>,
    ) -> Result<(), PlainApart> {
        ready(()).await;
        Err(PlainApart::NotFound)
    }

    async fn form_below(
        &self,
        _ctx: &(),
        _document_id: String,
        _confirmation: Option<Confirmation>,
    ) -> Result<(), PlainAsksBelow> {
        ready(()).await;
        Err(PlainAsksBelow::NotFound)
    }

    async fn form_drafted_apart(
        &self,
        _ctx: &(),
        _document_id: String,
        _confirmation: Option<Confirmation>,
    ) -> Result<(), DraftedApart> {
        ready(()).await;
        Err(DraftedApart::NotFound)
    }

    async fn link_above(&self, _ctx: &(), _file_id: String) -> Result<u32, DraftedAsksAbove> {
        ready(()).await;
        Err(DraftedAsksAbove::NotFound)
    }

    async fn link_apart(&self, _ctx: &(), _file_id: String) -> Result<u32, DraftedApart> {
        ready(()).await;
        Err(DraftedApart::NotFound)
    }

    async fn link_below(&self, _ctx: &(), _file_id: String) -> Result<u32, DraftedAsksBelow> {
        ready(()).await;
        Err(DraftedAsksBelow::NotFound)
    }
}
