//! The README's `Services` section: the service it declares, expanded here exactly as the README
//! declares it, and every emission shown beside it read back off the generator.

use super::readme;
use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context as PollContext, Poll, Waker};
use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

/// The context the README declares beside the service, character for character. The rest of that
/// block is elided prose -- `// ... and the other four` -- and is not a declaration to pin.
const DECLARED_CONTEXT: &str = "pub struct BillingContext {
    pub logger_name: String,
}";

/// The trait the README declares, character for character. It is the whole of the section's Rust
/// input: the shape, both message forms, the wire-name override and the one-way flag.
const DECLARED_SERVICE: &str = r#"#[service_schema()]
pub trait BillingService<Ctx> {
    /// No reply, so no return type and no error arm.
    #[service_schema_op(one_way)]
    async fn apply_bundle(&self, ctx: &Ctx, req: OpenBalanceRequest);

    /// Carried on the wire as `billing-generation-request` rather than `can-generate`.
    #[service_schema_op(message = "billing-generation-request")]
    async fn can_generate(
        &self,
        ctx: &Ctx,
        req: OpenBalanceRequest,
    ) -> Result<OpenBalanceResponse, BalanceError>;

    /// One argument after the context: that argument already is the message.
    async fn get_open_balance(
        &self,
        ctx: &Ctx,
        req: OpenBalanceRequest,
    ) -> Result<OpenBalanceResponse, BalanceError>;

    /// None at all, so the macro declares an empty message.
    async fn sweep(&self, ctx: &Ctx) -> Result<OpenBalanceResponse, BalanceError>;

    /// Several arguments after the context: the macro declares the message.
    async fn void_invoice(
        &self,
        ctx: &Ctx,
        customer_id: String,
        invoice_id: String,
    ) -> Result<OpenBalanceResponse, BalanceError>;
}"#;

/// The three types the operations answer through, as the README declares them.
const DECLARED_TYPES: &str = r#"#[model_schema()]
#[derive(Deserialize, Serialize)]
pub struct OpenBalanceRequest {
    pub customer_id: String,
}

#[model_schema()]
#[derive(Deserialize, Serialize)]
pub struct OpenBalanceResponse {
    pub is_post_paid: bool,
    pub open_invoices: u32,
}

#[model_schema()]
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum BalanceError {
    DbError,
    InsufficientBalance { shortfall: u32 },
}"#;

#[model_schema()]
#[derive(Deserialize, Serialize)]
pub struct OpenBalanceRequest {
    pub customer_id: String,
}

#[model_schema()]
#[derive(Deserialize, Serialize)]
pub struct OpenBalanceResponse {
    pub is_post_paid: bool,
    pub open_invoices: u32,
}

#[model_schema()]
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum BalanceError {
    DbError,
    InsufficientBalance { shortfall: u32 },
}

#[service_schema()]
pub trait BillingService<Ctx> {
    /// No reply, so no return type and no error arm.
    #[service_schema_op(one_way)]
    async fn apply_bundle(&self, ctx: &Ctx, req: OpenBalanceRequest);

    /// Carried on the wire as `billing-generation-request` rather than `can-generate`.
    #[service_schema_op(message = "billing-generation-request")]
    async fn can_generate(
        &self,
        ctx: &Ctx,
        req: OpenBalanceRequest,
    ) -> Result<OpenBalanceResponse, BalanceError>;

    /// One argument after the context: that argument already is the message.
    async fn get_open_balance(
        &self,
        ctx: &Ctx,
        req: OpenBalanceRequest,
    ) -> Result<OpenBalanceResponse, BalanceError>;

    /// None at all, so the macro declares an empty message.
    async fn sweep(&self, ctx: &Ctx) -> Result<OpenBalanceResponse, BalanceError>;

    /// Several arguments after the context: the macro declares the message.
    async fn void_invoice(
        &self,
        ctx: &Ctx,
        customer_id: String,
        invoice_id: String,
    ) -> Result<OpenBalanceResponse, BalanceError>;
}

pub struct BillingContext {
    pub logger_name: String,
}

pub struct BillingBackEnd;

impl BillingService<BillingContext> for BillingBackEnd {
    async fn apply_bundle(&self, ctx: &BillingContext, req: OpenBalanceRequest) {
        let _settled = ready(ctx.logger_name.len() + req.customer_id.len()).await;
    }

    async fn can_generate(
        &self,
        ctx: &BillingContext,
        req: OpenBalanceRequest,
    ) -> Result<OpenBalanceResponse, BalanceError> {
        self.get_open_balance(ctx, req).await
    }

    async fn get_open_balance(
        &self,
        ctx: &BillingContext,
        req: OpenBalanceRequest,
    ) -> Result<OpenBalanceResponse, BalanceError> {
        let seen = ready(req.customer_id.len()).await;
        if ctx.logger_name.is_empty() {
            Err(BalanceError::InsufficientBalance { shortfall: 1 })
        } else {
            Ok(OpenBalanceResponse {
                open_invoices: u32::try_from(seen).unwrap_or(0),
                is_post_paid: true,
            })
        }
    }

    async fn sweep(&self, _ctx: &BillingContext) -> Result<OpenBalanceResponse, BalanceError> {
        ready(()).await;
        Err(BalanceError::DbError)
    }

    async fn void_invoice(
        &self,
        _ctx: &BillingContext,
        customer_id: String,
        invoice_id: String,
    ) -> Result<OpenBalanceResponse, BalanceError> {
        let seen = ready(customer_id.len() + invoice_id.len()).await;
        Ok(OpenBalanceResponse {
            open_invoices: u32::try_from(seen).unwrap_or(0),
            is_post_paid: false,
        })
    }
}

/// The generator writes this, and the README shows it verbatim.
fn assert_shown(emission: &str, shown: &str) {
    assert!(
        emission.contains(shown),
        "the generator no longer writes this:\n{shown}\nGot: {emission}"
    );
    assert!(
        readme().contains(shown),
        "the README no longer shows this verbatim:\n{shown}"
    );
}

/// A member matched as a whole line on both sides, so a spelling that is merely a prefix of the
/// emitted one cannot pass for it. What the surrounding `JSDoc` says is the generator's business and
/// not the README's, which is why a block is read line by line rather than whole.
fn assert_shown_members(emission: &str, members: &[&str]) {
    for member in members {
        assert!(
            emission.lines().any(|line| line == *member),
            "the generator no longer writes this member: {member}\nGot: {emission}"
        );
        assert!(
            readme().lines().any(|line| line == *member),
            "the README no longer shows this member verbatim: {member}"
        );
    }
}

/// A declaration the README shows and this file expands, held to being one text rather than two.
///
/// The pin appears twice in this file: once as the constant the README is searched for, and once as
/// the declaration the compiler reads. Anything that moves one and not the other -- a rustfmt pass,
/// an edit to either side -- lands here.
fn assert_declared_and_documented(pinned: &str) {
    assert_eq!(
        source().matches(pinned).count(),
        2,
        "this is pinned but no longer declared here character for character:\n{pinned}"
    );
    assert!(
        readme().contains(pinned),
        "the README no longer declares this verbatim:\n{pinned}"
    );
}

/// This file, read back so a pinned declaration can be held against the one that compiles.
fn source() -> &'static str {
    include_str!("services.rs")
}

#[test]
fn the_readme_declares_the_service_the_section_is_written_around() {
    assert_declared_and_documented(DECLARED_TYPES);
    assert_declared_and_documented(DECLARED_SERVICE);
}

#[test]
fn a_message_declared_from_an_argument_list_is_shown_as_the_generator_writes_it() {
    assert_shown_members(
        &BillingServiceSchema::ts_definition(),
        &[
            "export type VoidInvoiceRequest = {",
            "  customerId: string;",
            "  invoiceId: string;",
            "};",
        ],
    );
}

#[test]
fn the_message_declared_for_an_operation_taking_nothing_is_shown_as_a_document_of_its_own() {
    assert_shown(
        &BillingServiceSchema::ts_definition(),
        "export type SweepRequest = Record<string, never>;",
    );
}

#[test]
fn the_error_an_operation_declares_is_shown_as_the_union_it_publishes() {
    assert_shown_members(
        &BalanceError::ts_definition(),
        &[
            "export type BalanceError = {",
            "  errorCode: \"db-error\";",
            "} | {",
            "  errorCode: \"insufficient-balance\";",
            "  shortfall: number;",
            "};",
        ],
    );
}

#[test]
fn the_result_a_caller_narrows_on_is_shown_with_the_fault_inside_its_failure_arm() {
    assert_shown(
        &BillingServiceSchema::ts_definition(),
        "export type BillingServiceGetOpenBalanceResult =\n  \
         | { ok: true; value: OpenBalanceResponse }\n  \
         | { ok: false; error: BalanceError | { isServiceFault: true; fault: BillingServiceFault } };",
    );
}

// The README shows the client and the dispatcher, which only a build with the Zod surface
// publishes: both parse a message against the schema `#[model_schema()]` writes for it.
#[cfg(feature = "zod")]
#[test]
fn the_outcome_an_implementation_answers_is_shown_with_no_fault_in_it() {
    assert_shown(
        &BillingServiceSchema::ts_service(),
        "export type BillingServiceGetOpenBalanceOutcome =\n  \
         | { ok: true; value: OpenBalanceResponse }\n  \
         | { ok: false; error: BalanceError };",
    );
}

// The README shows the client and the dispatcher, which only a build with the Zod surface
// publishes: both parse a message against the schema `#[model_schema()]` writes for it.
#[cfg(feature = "zod")]
#[test]
fn the_client_type_is_shown_with_every_method_the_service_declares() {
    assert_shown_members(
        &BillingServiceSchema::ts_client(),
        &[
            "export type BillingServiceClient = {",
            "  applyBundle(req: OpenBalanceRequest): Promise<void>;",
            "  canGenerate(req: OpenBalanceRequest): Promise<BillingServiceCanGenerateResult>;",
            "  getOpenBalance(req: OpenBalanceRequest): \
             Promise<BillingServiceGetOpenBalanceResult>;",
            "  sweep(req: SweepRequest): Promise<BillingServiceSweepResult>;",
            "  voidInvoice(req: VoidInvoiceRequest): Promise<BillingServiceVoidInvoiceResult>;",
            "};",
        ],
    );
}

// The README shows the client and the dispatcher, which only a build with the Zod surface
// publishes: both parse a message against the schema `#[model_schema()]` writes for it.
#[cfg(feature = "zod")]
#[test]
fn the_implementable_interface_is_shown_with_every_member_required() {
    assert_shown(
        &BillingServiceSchema::ts_service(),
        "export interface BillingServiceImpl<Ctx> {\n  \
         /** Handles `apply-bundle` on `BillingService`, which expects no reply. */\n  \
         applyBundle(ctx: Ctx, req: OpenBalanceRequest): Promise<void>;\n  \
         /** Handles `billing-generation-request` on `BillingService` and answers it. */\n  \
         canGenerate(ctx: Ctx, req: OpenBalanceRequest): \
         Promise<BillingServiceCanGenerateOutcome>;\n  \
         /** Handles `get-open-balance` on `BillingService` and answers it. */\n  \
         getOpenBalance(ctx: Ctx, req: OpenBalanceRequest): \
         Promise<BillingServiceGetOpenBalanceOutcome>;\n  \
         /** Handles `sweep` on `BillingService` and answers it. */\n  \
         sweep(ctx: Ctx, req: SweepRequest): Promise<BillingServiceSweepOutcome>;\n  \
         /** Handles `void-invoice` on `BillingService` and answers it. */\n  \
         voidInvoice(ctx: Ctx, req: VoidInvoiceRequest): \
         Promise<BillingServiceVoidInvoiceOutcome>;\n}",
    );
}

// The README shows the client and the dispatcher, which only a build with the Zod surface
// publishes: both parse a message against the schema `#[model_schema()]` writes for it.
#[cfg(feature = "zod")]
#[test]
fn the_dispatcher_factory_is_shown_with_the_signature_it_is_emitted_under() {
    assert_shown(
        &BillingServiceSchema::ts_service(),
        "export function createBillingServiceDispatcher<Ctx>(\n  \
         impl: BillingServiceImpl<Ctx>,\n\
         ): (\n  \
         ctx: Ctx,\n  \
         operation: string,\n  \
         payload: unknown,\n  \
         headers?: ReadonlyArray<readonly [string, string]>,\n  \
         parts?: ReadonlyArray<readonly [string, unknown]>,\n\
         ) => Promise<BillingServiceDispatched | undefined> {",
    );
}

#[cfg(feature = "zod")]
#[test]
fn the_schema_a_declared_message_publishes_is_shown_as_the_generator_writes_it() {
    let published = BillingServiceSchema::ts_definition();
    assert_shown(
        &published,
        "const VoidInvoiceRequest$RawSchema = z.strictObject({\n  \
         customerId: z.string(),\n  \
         invoiceId: z.string(),\n});",
    );
    assert_shown(
        &published,
        "export const VoidInvoiceRequest$Schema: ZodType<VoidInvoiceRequest> = \
         VoidInvoiceRequest$RawSchema.meta({\n  examples: []\n});",
    );
}

#[cfg(feature = "zod")]
#[test]
fn the_schema_an_empty_message_publishes_is_shown_too() {
    let published = BillingServiceSchema::ts_definition();
    assert_shown(
        &published,
        "const SweepRequest$RawSchema = z.strictObject({\n});",
    );
    assert_shown(
        &published,
        "export const SweepRequest$Schema: ZodType<SweepRequest> = SweepRequest$RawSchema.meta({\n  examples: []\n});",
    );
}

#[cfg(feature = "zod")]
#[test]
fn the_shape_a_refused_one_way_message_is_thrown_as_is_shown() {
    assert_shown(
        &BillingServiceSchema::ts_client(),
        "export type BillingServiceRefusal = Error & { fault: BillingServiceFault };",
    );
}

#[cfg(feature = "zod")]
#[test]
fn the_throw_a_one_way_method_documents_is_shown_as_emitted() {
    assert_shown(
        &BillingServiceSchema::ts_client(),
        "   * @throws {BillingServiceRefusal} when the message fails its own schema. The \
         operation\n   \
         * answers `Promise<void>`, so there is no failure arm to put the fault in; the \
         transport\n   \
         * is still never reached.",
    );
}

#[cfg(feature = "zod")]
#[test]
fn the_one_way_refusal_the_client_throws_is_shown_as_emitted() {
    assert_shown(
        &BillingServiceSchema::ts_client(),
        "    async applyBundle(req) {\n      \
         const validated = billingServiceChecked(OpenBalanceRequest$Schema, req);\n      \
         if (!validated.success) {\n        \
         throw billingServiceRefused(billingServiceOutboundFault(\"apply-bundle\", \
         validated.error.issues));\n      \
         }\n      \
         await transport.notify(\"apply-bundle\", validated.sent, []);\n    },",
    );
}

#[cfg(feature = "zod")]
#[test]
fn the_outbound_check_the_client_runs_before_the_transport_is_shown_as_emitted() {
    assert_shown(
        &BillingServiceSchema::ts_client(),
        "    async voidInvoice(req) {\n      \
         const validated = billingServiceChecked(VoidInvoiceRequest$Schema, req);\n      \
         if (!validated.success) {\n        \
         return {\n          \
         ok: false,\n          \
         error: {\n            \
         isServiceFault: true,\n            \
         fault: billingServiceOutboundFault(\"void-invoice\", validated.error.issues),\n          \
         },\n        \
         };\n      \
         }\n      \
         const { answered } = await \
         transport.request<BillingServiceVoidInvoiceResult>(\"void-invoice\", validated.sent, \
         []);\n      \
         if (answered.ok) {\n        \
         const parsed = OpenBalanceResponse$Schema.safeParse(answered.value);\n        \
         if (!parsed.success) {\n          \
         return {\n            \
         ok: false,\n            \
         error: {\n              \
         isServiceFault: true,\n              \
         fault: billingServiceReplyFault(\"void-invoice\", parsed.error.issues),\n            \
         },\n          \
         };\n        \
         }\n        \
         return { ok: true, value: parsed.data };\n      \
         }\n      \
         const error = answered.error;\n      \
         if (typeof error === \"object\" && error !== null && \"isServiceFault\" in error) {\n        \
         return { ok: false, error };\n      \
         }\n      \
         const declared = BalanceError$Schema.safeParse(error);\n      \
         if (!declared.success) {\n        \
         return {\n          \
         ok: false,\n          \
         error: {\n            \
         isServiceFault: true,\n            \
         fault: billingServiceReplyFault(\"void-invoice\", declared.error.issues),\n          \
         },\n        \
         };\n      \
         }\n      \
         return { ok: false, error: declared.data };\n    },",
    );
}

/// The service never suspends, so one poll answers it; `None` says an assumption about the bodies
/// above stopped holding rather than that the runtime is missing.
fn poll_once<Answered>(answering: Answered) -> Option<Answered::Output>
where
    Answered: Future,
{
    let mut pinned = pin!(answering);
    let mut polling = PollContext::from_waker(Waker::noop());
    match pinned.as_mut().poll(&mut polling) {
        Poll::Ready(answer) => Some(answer),
        Poll::Pending => None,
    }
}

/// The customer the sample calls are made for.
fn asking() -> OpenBalanceRequest {
    OpenBalanceRequest {
        customer_id: "acme".to_owned(),
    }
}

/// The invoices an operation answered with, and `u32::MAX` where it answered its declared error.
fn invoices(answered: Result<OpenBalanceResponse, BalanceError>) -> u32 {
    answered.map_or(u32::MAX, |balance| balance.open_invoices)
}

#[test]
fn the_service_the_readme_declares_is_implementable_at_the_context_it_shows() {
    assert_declared_and_documented(DECLARED_CONTEXT);
    let ctx = BillingContext {
        logger_name: "readme".to_owned(),
    };
    assert_eq!(
        invoices(poll_once(BillingBackEnd.get_open_balance(&ctx, asking())).unwrap()),
        4,
        "the README's sample is a service that compiles and answers, not a sketch"
    );
    assert_eq!(
        invoices(poll_once(BillingBackEnd.can_generate(&ctx, asking())).unwrap()),
        4,
        "an overridden wire name changes nothing about the Rust that calls it"
    );
    assert_eq!(
        invoices(
            poll_once(BillingBackEnd.void_invoice(&ctx, "acme".to_owned(), "cr-1".to_owned()))
                .unwrap()
        ),
        8,
        "a multi-argument operation still takes its arguments separately"
    );
    assert_eq!(
        invoices(poll_once(BillingBackEnd.sweep(&ctx)).unwrap()),
        u32::MAX,
        "the error arm is the operation's own, and it is the only failure it can name"
    );
    assert_eq!(
        poll_once(BillingBackEnd.apply_bundle(&ctx, asking())),
        Some(()),
        "a one-way operation answers with nothing at all"
    );
}
