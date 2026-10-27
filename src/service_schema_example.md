```rust
use serde::{Deserialize, Serialize};
use serde_json::json;
use tixschema::{model_schema, service_schema};

use invoicing_schema::{CallError, ServiceFault, ServiceFaultKind};

#[model_schema()]
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceLookup {
    pub invoice_id: String,
}

#[model_schema()]
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceSummary {
    pub invoice_id: String,
    pub total_cents: u32,
}

#[model_schema()]
#[derive(Deserialize, Serialize)]
#[serde(tag = "errorCode", rename_all = "kebab-case")]
pub enum InvoiceError {
    NotFound,
    AlreadyVoided,
}

#[service_schema()]
pub trait Invoicing<Ctx> {
    /// One argument after the context: that argument is the message.
    async fn get_invoice(
        &self,
        ctx: &Ctx,
        req: InvoiceLookup,
    ) -> Result<InvoiceSummary, InvoiceError>;

    /// Several arguments: the macro declares `VoidInvoiceRequest` from them.
    async fn void_invoice(
        &self,
        ctx: &Ctx,
        invoice_id: String,
        reason: String,
    ) -> Result<InvoiceSummary, InvoiceError>;

    /// No reply, so no return type and no error.
    #[service_schema_op(one_way)]
    async fn resend_invoice(&self, ctx: &Ctx, req: InvoiceLookup);
}

/// What a caller does with each outcome of a client call.
fn shown(answered: Result<InvoiceSummary, CallError<InvoiceError>>) -> &'static str {
    match answered {
        Ok(_summary) => "voided",
        Err(CallError::Operation(InvoiceError::NotFound)) => "no such invoice",
        Err(CallError::Operation(InvoiceError::AlreadyVoided)) => "already voided",
        Err(CallError::Fault(_defect)) => "a defect, logged",
    }
}

fn main() {
    // The generated message writes camelCase keys, and has the validator a dispatcher calls.
    let message = VoidInvoiceRequest {
        invoice_id: "INV-7".to_owned(),
        reason: "duplicate".to_owned(),
    };
    assert_eq!(
        serde_json::to_value(&message).unwrap(),
        json!({ "invoiceId": "INV-7", "reason": "duplicate" })
    );
    assert!(invoicing_schema::validated_void_invoice(&message).is_ok());

    // A fault reports a defect no operation declared; no implementation can return one.
    let fault = ServiceFault::unknown_operation("archive-invoice");
    assert_eq!(fault.kind(), ServiceFaultKind::UnknownOperation);
    assert_eq!(
        serde_json::to_value(&fault).unwrap(),
        json!({
            "detail": "the service answers to no operation by that name",
            "kind": "unknown-operation",
            "operation": "archive-invoice"
        })
    );
    assert_eq!(shown(Err(CallError::Fault(fault))), "a defect, logged");
}
```
