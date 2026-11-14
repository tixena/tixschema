//! One service for each transport, holding a renamed and a plain operation, a one-way one, a
//! streamed one, and the prompt and the resource of an MCP service.

use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context, Poll, Waker};
use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum NamesError {
    NotFound,
}

#[service_schema(transports = ["amqp_rpc"])]
pub trait TemplateService<Ctx> {
    async fn create_or_update_combined_template(
        &self,
        ctx: &Ctx,
        name: String,
    ) -> Result<String, NamesError>;

    #[service_schema_op(message = "remove-combined-template")]
    async fn delete_combined_template(&self, ctx: &Ctx, name: String)
    -> Result<String, NamesError>;

    #[service_schema_op(one_way)]
    async fn touch_template(&self, ctx: &Ctx, name: String);
}

#[service_schema(transports = ["http_rest"])]
pub trait ContentService<Ctx> {
    #[service_schema_op(http(
        method = "GET",
        path = "/documents/{document_id}/content",
        body = "stream"
    ))]
    async fn get_content(
        &self,
        ctx: &Ctx,
        document_id: String,
    ) -> Result<content_service_schema::StreamedAnswer, NamesError>;

    #[service_schema_op(http(method = "GET", path = "/documents/{document_id}"))]
    async fn get_document(&self, ctx: &Ctx, document_id: String) -> Result<String, NamesError>;
}

#[service_schema(transports = ["mcp"])]
pub trait CounselService<Ctx> {
    /// Guides the model.
    #[service_schema_op(mcp(prompt))]
    async fn configure_organization(&self, ctx: &Ctx) -> Result<String, NamesError>;

    /// The counsel.
    async fn get_counsel(&self, ctx: &Ctx) -> Result<String, NamesError>;

    /// One invoice.
    #[service_schema_op(mcp(resource = "billing://invoices/{invoice_id}"))]
    async fn get_invoice(&self, ctx: &Ctx, invoice_id: String) -> Result<String, NamesError>;
}

#[service_schema(transports = ["ws_rpc"])]
pub trait SocketService<Ctx> {
    async fn ping_socket(&self, ctx: &Ctx, name: String) -> Result<String, NamesError>;
}

#[service_schema(transports = ["amqp_rpc", "http_rest", "mcp", "ws_rpc"])]
pub trait EveryService<Ctx> {
    /// Everything.
    #[service_schema_op(message = "fetch-all", http(method = "GET", path = "/all"))]
    async fn fetch_everything(&self, ctx: &Ctx) -> Result<String, NamesError>;

    /// The keyword.
    #[service_schema_op(http(method = "GET", path = "/in"))]
    async fn r#in(&self, ctx: &Ctx) -> Result<String, NamesError>;
}

pub struct Unit;

impl TemplateService<()> for Unit {
    async fn create_or_update_combined_template(
        &self,
        _ctx: &(),
        _name: String,
    ) -> Result<String, NamesError> {
        ready(()).await;
        Err(not_found())
    }

    async fn delete_combined_template(
        &self,
        _ctx: &(),
        _name: String,
    ) -> Result<String, NamesError> {
        ready(()).await;
        Err(not_found())
    }

    async fn touch_template(&self, _ctx: &(), _name: String) {
        ready(()).await;
    }
}

impl ContentService<()> for Unit {
    async fn get_content(
        &self,
        _ctx: &(),
        _document_id: String,
    ) -> Result<content_service_schema::StreamedAnswer, NamesError> {
        ready(()).await;
        Err(not_found())
    }

    async fn get_document(&self, _ctx: &(), _document_id: String) -> Result<String, NamesError> {
        ready(()).await;
        Err(not_found())
    }
}

impl CounselService<()> for Unit {
    async fn configure_organization(&self, _ctx: &()) -> Result<String, NamesError> {
        ready(()).await;
        Err(not_found())
    }

    async fn get_counsel(&self, _ctx: &()) -> Result<String, NamesError> {
        ready(()).await;
        Err(not_found())
    }

    async fn get_invoice(&self, _ctx: &(), _invoice_id: String) -> Result<String, NamesError> {
        ready(()).await;
        Err(not_found())
    }
}

impl SocketService<()> for Unit {
    async fn ping_socket(&self, _ctx: &(), _name: String) -> Result<String, NamesError> {
        ready(()).await;
        Err(not_found())
    }
}

impl EveryService<()> for Unit {
    async fn fetch_everything(&self, _ctx: &()) -> Result<String, NamesError> {
        ready(()).await;
        Err(not_found())
    }

    async fn r#in(&self, _ctx: &()) -> Result<String, NamesError> {
        ready(()).await;
        Err(not_found())
    }
}

#[test]
fn a_streamed_operation_is_listed() {
    use content_service_schema::{OPERATIONS, Operation};
    assert_eq!(of_content(&Unit), OPERATIONS);
    assert_eq!(OPERATIONS, ["get-content", "get-document"]);
    assert_eq!(Operation::GetContent.name(), "get-content");
    assert_eq!(Operation::GetDocument.name(), "get-document");
}

#[test]
fn a_websocket_service_and_a_service_of_every_transport_have_the_items() {
    assert_eq!(of_socket(&Unit), socket_service_schema::OPERATIONS);
    assert_eq!(of_every(&Unit), every_service_schema::OPERATIONS);
    assert_eq!(socket_service_schema::OPERATIONS, ["ping-socket"]);
    assert_eq!(
        socket_service_schema::Operation::PingSocket.name(),
        "ping-socket"
    );
    assert_eq!(every_service_schema::OPERATIONS, ["fetch-all", "in"]);
    assert_eq!(
        every_service_schema::Operation::FetchEverything.name(),
        "fetch-all"
    );
    assert_eq!(every_service_schema::Operation::In.name(), "in");
}

#[test]
fn an_amqp_service_lists_a_renamed_and_a_plain_operation_in_declaration_order() {
    use template_service_schema::{OPERATIONS, Operation};
    assert_eq!(of_template(&Unit), OPERATIONS);
    assert_eq!(
        OPERATIONS,
        [
            "create-or-update-combined-template",
            "remove-combined-template",
            "touch-template"
        ]
    );
    assert_eq!(
        Operation::CreateOrUpdateCombinedTemplate.name(),
        "create-or-update-combined-template"
    );
    assert_eq!(
        Operation::DeleteCombinedTemplate.name(),
        "remove-combined-template"
    );
    assert_eq!(Operation::TouchTemplate.name(), "touch-template");
    takes(OPERATIONS);
}

#[test]
fn an_mcp_prompt_and_resource_are_listed() {
    use counsel_service_schema::{OPERATIONS, Operation};
    assert_eq!(of_counsel(&Unit), OPERATIONS);
    assert_eq!(
        OPERATIONS,
        ["configure-organization", "get-counsel", "get-invoice"]
    );
    assert_eq!(
        Operation::ConfigureOrganization.name(),
        "configure-organization"
    );
    assert_eq!(Operation::GetInvoice.name(), "get-invoice");
    assert_eq!(Operation::GetCounsel.name(), "get-counsel");
}

fn answered<Answering>(answering: Answering) -> Answering::Output
where
    Answering: Future,
{
    let reply = match pin!(answering).poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(output) => Some(output),
        Poll::Pending => None,
    };
    reply.unwrap()
}

#[test]
fn each_variant_names_its_entry() {
    use template_service_schema::{OPERATIONS, Operation};
    let variants = [
        Operation::CreateOrUpdateCombinedTemplate,
        Operation::DeleteCombinedTemplate,
        Operation::TouchTemplate,
    ];
    let named: Vec<&str> = variants.iter().map(|variant| variant.name()).collect();
    assert_eq!(named, OPERATIONS);
}

fn not_found() -> NamesError {
    NamesError::NotFound
}

fn of_content<S>(_: &S) -> &'static [&'static str]
where
    S: ContentService<()>,
{
    content_service_schema::OPERATIONS
}

fn of_counsel<S>(_: &S) -> &'static [&'static str]
where
    S: CounselService<()>,
{
    counsel_service_schema::OPERATIONS
}

fn of_every<S>(_: &S) -> &'static [&'static str]
where
    S: EveryService<()>,
{
    every_service_schema::OPERATIONS
}

fn of_socket<S>(_: &S) -> &'static [&'static str]
where
    S: SocketService<()>,
{
    socket_service_schema::OPERATIONS
}

fn of_template<S>(_: &S) -> &'static [&'static str]
where
    S: TemplateService<()>,
{
    template_service_schema::OPERATIONS
}

fn takes(_: &'static [&'static str]) {}

#[test]
fn the_back_end_of_each_service_answers_each_operation() {
    let unit = Unit;
    let name = || "name".to_owned();
    answered(TemplateService::create_or_update_combined_template(
        &unit,
        &(),
        name(),
    ))
    .unwrap_err();
    answered(TemplateService::delete_combined_template(
        &unit,
        &(),
        name(),
    ))
    .unwrap_err();
    answered(TemplateService::touch_template(&unit, &(), name()));
    answered(ContentService::get_content(&unit, &(), name()))
        .err()
        .unwrap();
    answered(ContentService::get_document(&unit, &(), name())).unwrap_err();
    answered(CounselService::configure_organization(&unit, &())).unwrap_err();
    answered(CounselService::get_invoice(&unit, &(), name())).unwrap_err();
    answered(CounselService::get_counsel(&unit, &())).unwrap_err();
    answered(SocketService::ping_socket(&unit, &(), name())).unwrap_err();
    answered(EveryService::fetch_everything(&unit, &())).unwrap_err();
    answered(EveryService::r#in(&unit, &())).unwrap_err();
}

#[test]
fn the_fault_kind_a_service_writes_for_itself_has_its_wire_name_on_the_one_enum() {
    use template_service_schema::ServiceFaultKind;
    for kind in [
        ServiceFaultKind::FailedValidation,
        ServiceFaultKind::UnknownOperation,
    ] {
        assert_eq!(
            serde_json::to_value(kind).unwrap(),
            serde_json::Value::String(kind.wire_name().to_owned())
        );
    }
    assert_eq!(
        ServiceFaultKind::FailedValidation.wire_name(),
        "failed-validation"
    );
}
