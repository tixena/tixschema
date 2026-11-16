//! The TypeScript every service surface writes, read by the TypeScript compiler's own parser: it
//! holds no `as` expression and no type assertion, so every value of a branded type comes out of
//! its schema and every other value is typed where it is made.

use super::runtime::{node_modules, ran_with_modules, stand_down_modules};
use super::typescript_casts::{driver, found};

/// The package the parser comes from.
const REQUIRED_PACKAGES: &[&str] = &["typescript"];

#[test]
fn the_typescript_of_every_service_surface_writes_no_cast() {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(
            REQUIRED_PACKAGES,
            "the cast search over the service bundles",
        );
        return;
    };
    let texts = [
        ("belfry", super::run_belfry::node_bundle()),
        ("carillon", super::run_carillon::node_bundle()),
        ("carillon_mcp", super::run_carillon_mcp::node_bundle()),
        ("services", services()),
    ];
    let Some(printed) = ran_with_modules(
        "casts",
        "TIXSCHEMA_NODE",
        "node",
        "casts.mjs",
        &driver(&texts),
        &modules,
    ) else {
        return;
    };
    let casts = found(&printed);
    assert!(
        casts.is_empty(),
        "{} casts:\n{}",
        casts.len(),
        casts.join("\n")
    );
}

/// The TypeScript of the services whose operations reach every body kind, header binding and
/// transport surface: a stream, bytes, `header_out`, the `ws_rpc` server, and the `mcp` server and
/// client.
fn services() -> String {
    use super::run_node_bytes::BytesClientServiceSchema;
    use super::run_node_mcp::{ArchiveServiceSchema, BillingServiceSchema, DocumentServiceSchema};
    use super::tests::{
        ContentClientServiceSchema, ConversationClientServiceSchema, EchoClientServiceSchema,
        GateClientServiceSchema, LabelClientServiceSchema, PulseClientServiceSchema,
        SealClientServiceSchema, SearchClientServiceSchema,
    };
    [
        ConversationClientServiceSchema::ts_definition(),
        ConversationClientServiceSchema::ts_service(),
        ConversationClientServiceSchema::ts_client(),
        ConversationClientServiceSchema::ts_http_client(),
        ConversationClientServiceSchema::ts_http_service(),
        ConversationClientServiceSchema::ts_ws_client(),
        ConversationClientServiceSchema::ts_ws_server(),
        ConversationClientServiceSchema::ts_ws_service(),
        BytesClientServiceSchema::ts_service(),
        BytesClientServiceSchema::ts_client(),
        BytesClientServiceSchema::ts_http_client(),
        BytesClientServiceSchema::ts_http_service(),
        ContentClientServiceSchema::ts_service(),
        ContentClientServiceSchema::ts_http_client(),
        ContentClientServiceSchema::ts_http_service(),
        EchoClientServiceSchema::ts_service(),
        EchoClientServiceSchema::ts_http_client(),
        EchoClientServiceSchema::ts_http_service(),
        GateClientServiceSchema::ts_service(),
        GateClientServiceSchema::ts_http_service(),
        LabelClientServiceSchema::ts_service(),
        LabelClientServiceSchema::ts_http_service(),
        PulseClientServiceSchema::ts_service(),
        PulseClientServiceSchema::ts_http_service(),
        SealClientServiceSchema::ts_service(),
        SealClientServiceSchema::ts_http_service(),
        SearchClientServiceSchema::ts_service(),
        SearchClientServiceSchema::ts_http_service(),
        ArchiveServiceSchema::ts_service(),
        ArchiveServiceSchema::ts_mcp_service(),
        ArchiveServiceSchema::ts_mcp_client(),
        BillingServiceSchema::ts_service(),
        BillingServiceSchema::ts_mcp_service(),
        BillingServiceSchema::ts_mcp_client(),
        DocumentServiceSchema::ts_service(),
        DocumentServiceSchema::ts_mcp_service(),
        DocumentServiceSchema::ts_mcp_client(),
    ]
    .join("\n\n")
}
