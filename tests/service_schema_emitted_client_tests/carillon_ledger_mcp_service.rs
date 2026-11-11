//! `CarillonLedgerMcpService` on `mcp`, whose one tool takes a ledger that holds a `u64` and
//! answers it: the TypeScript server writes a `u64` beyond what a JavaScript `number` holds as the
//! Rust dispatcher writes it, and the Rust dispatcher answers alike through each of its entry
//! points. A build with `swift` or `kotlin` refuses a `u64`, so this module is compiled only
//! without them. Nothing here waits on a clock.

use super::carillon_ledger_mcp_dispatcher;
use super::carillon_service::CarillonError;
use super::mcp_writer_rows::ready_output;
use super::runtime::{node_modules, ran_with_modules, stand_down_modules};
use super::tests::poll_once;
use crate::mcp_streams::through;
use core::future::{Future, ready};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::{model_schema, service_schema};

/// The server handed the request, its answer printed as the text it wrote.
const NODE_DRIVER: &str = "
const impl = {
  async record(ctx, req) {
    return { ok: true, value: req };
  },
};
const dispatch = createCarillonLedgerMcpServiceMcpDispatcher(impl);
console.log(await dispatch(REQUEST, () => ({}), async () => {}, new CarillonLedgerMcpServiceMcpInFlight()));
";

/// The rungs a carillon rang.
#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CarillonLedger {
    pub rungs: u64,
}

/// Keeps the ledger of a carillon over `mcp`.
#[service_schema(transports = ["mcp"])]
pub trait CarillonLedgerMcpService<Ctx> {
    /// Records the ledger the arguments name, and answers it.
    async fn record(&self, ctx: &Ctx, req: CarillonLedger)
    -> Result<CarillonLedger, CarillonError>;
}

/// Answers the ledger it is handed.
pub struct McpLedger;

impl CarillonLedgerMcpService<()> for McpLedger {
    fn record(
        &self,
        _ctx: &(),
        req: CarillonLedger,
    ) -> impl Future<Output = Result<CarillonLedger, CarillonError>> {
        ready(Ok(req))
    }
}

/// Where progress is written: nowhere, since this run compares the answer alone.
struct Sink;

impl carillon_ledger_mcp_dispatcher::McpEventWriter for Sink {
    fn write(
        &self,
        _message: String,
    ) -> impl Future<Output = Result<(), carillon_ledger_mcp_dispatcher::McpClientGone>> {
        ready(Ok(()))
    }
}

/// The `tools/call` request that carries the ledger of `u64::MAX` rungs to `record`.
fn call() -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"record","arguments":{{"rungs":{}}},"_meta":{{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{{}}}}}}}}"#,
        u64::MAX
    )
}

#[test]
fn the_typescript_server_writes_a_u64_beyond_a_number_as_the_rust_dispatcher_writes_it() {
    use super::CarillonLedgerMcpServiceSchema;

    const REQUIRED_PACKAGES: &[&str] = &["zod"];
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(
            REQUIRED_PACKAGES,
            "the emitted TypeScript carillon ledger mcp service",
        );
        return;
    };
    let source = [
        "import { z, type ZodType, type $brand } from \"zod\";".to_owned(),
        CarillonLedger::ts_definition(),
        CarillonLedger::zod_schema(),
        CarillonError::ts_definition(),
        CarillonError::zod_schema(),
        CarillonLedgerMcpServiceSchema::ts_definition(),
        CarillonLedgerMcpServiceSchema::ts_service(),
        CarillonLedgerMcpServiceSchema::ts_mcp_service(),
        format!("const REQUEST = String.raw`{}`;", call()),
        NODE_DRIVER.to_owned(),
    ]
    .join("\n\n");
    let Some(wrote) = ran_with_modules(
        "carillon-ledger-mcp",
        "TIXSCHEMA_NODE",
        "node",
        "carillon_ledger_mcp.mts",
        &source,
        &modules,
    ) else {
        return;
    };
    let rust = poll_once(carillon_ledger_mcp_dispatcher::answer(
        &call(),
        &McpLedger,
        |_call| (),
        &Sink,
        &carillon_ledger_mcp_dispatcher::McpInFlight::default(),
    ))
    .flatten()
    .unwrap();
    assert!(rust.contains(&format!(r#""structuredContent":{{"rungs":{}}}"#, u64::MAX)));
    assert_eq!(wrote.trim(), rust);
}

#[test]
fn the_rust_dispatcher_answers_alone_beside_an_endpoint_and_over_http_alike() {
    let (alone, served, status) = rust_serves_alike!(
        carillon_ledger_mcp_dispatcher,
        super::CarillonLedgerMcpServiceSchema,
        McpLedger,
        ready_output
    );
    assert_eq!(alone, served);
    assert!(alone.is_some());
    assert_eq!(status, 200);
}

#[test]
fn the_rust_dispatcher_listens_alike_through_every_entry_point() {
    let (alone, served) = rust_listens_alike!(
        carillon_ledger_mcp_dispatcher,
        super::CarillonLedgerMcpServiceSchema,
        McpLedger,
        ready_output
    );
    assert_eq!(alone, served);
    assert!(alone.is_some());
}

#[test]
fn an_admit_that_denies_answers_its_status_over_http() {
    for (denied, status) in [
        (
            carillon_ledger_mcp_dispatcher::McpDenied::Forbidden,
            403_u16,
        ),
        (
            carillon_ledger_mcp_dispatcher::McpDenied::InsufficientScope {
                scope: "ledger:write".to_owned(),
            },
            403,
        ),
        (
            carillon_ledger_mcp_dispatcher::McpDenied::NoValidCredentials { scope: None },
            401,
        ),
    ] {
        let asked = carillon_ledger_mcp_dispatcher::McpHttpRequest::new(
            "POST".to_owned(),
            "/mcp".to_owned(),
            vec![
                ("Content-Type".to_owned(), "application/json".to_owned()),
                ("MCP-Protocol-Version".to_owned(), "2026-07-28".to_owned()),
                ("Mcp-Method".to_owned(), "tools/call".to_owned()),
                ("Mcp-Name".to_owned(), "record".to_owned()),
            ],
            call().into_bytes(),
        );
        let settings = carillon_ledger_mcp_dispatcher::McpHttpSettings::new(&[], None);
        let answered = through(|stream| {
            ready_output(carillon_ledger_mcp_dispatcher::answer_http(
                &asked,
                &settings,
                stream,
                &McpLedger,
                async |request: &carillon_ledger_mcp_dispatcher::McpHttpRequest,
                       _call: carillon_ledger_mcp_dispatcher::McpCall|
                       -> Result<
                    carillon_ledger_mcp_dispatcher::McpAdmitted<()>,
                    carillon_ledger_mcp_dispatcher::McpDenied,
                > {
                    assert_eq!(request.mcp_name().as_deref(), Some("record"));
                    Err(denied.clone())
                },
            ))
        });
        assert_eq!(answered.status(), status);
    }
}
