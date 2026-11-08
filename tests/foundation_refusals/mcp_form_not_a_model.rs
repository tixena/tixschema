//! A service whose operation asks with a form of a type that is no model type: it has no
//! `mcp_form_flat()`.

use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

#[derive(Debug, Deserialize, Serialize)]
pub struct ShippingForm {
    pub express: bool,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "errorCode")]
pub enum ShipError {
    NeedsShipping,
}

#[service_schema(transports = ["mcp"])]
pub trait ShippingService<Ctx> {
    /// Ships one document.
    #[service_schema_op(mcp(interview(arm = "NeedsShipping", answers = shipping)))]
    async fn ship_document(
        &self,
        ctx: &Ctx,
        document_id: String,
        shipping: Option<ShippingForm>,
    ) -> Result<(), ShipError>;
}

fn main() {}
