//! The emitted TypeScript `mcp` server run by node over a service whose resources, template,
//! listing and completion declare scopes: its scopes function and its HTTP server under OAuth
//! answer what the Rust dispatcher answers.

use super::StacksServiceSchema;
use super::mcp_library_service::{BookError, BookInfo, BookRequest};
use super::mcp_stacks_service::{assert_matches_rust, input};
use super::runtime::{node_modules, ran_with_modules, stand_down_modules};
use crate::mcp_streams::NODE_RESPOND;
use serde_json::Value;

const RUNTIME_VAR: &str = "TIXSCHEMA_NODE";

const REQUIRED_PACKAGES: &[&str] = &["zod"];

const DRIVER: &str = r##"
async function main() {
  const input = JSON.parse(INPUT);
  let reached = 0;
  const found = (value) => { reached += 1; return { ok: true, value }; };
  const impl = {
    async stacksIndex() { return found("# The stacks"); },
    async pinnedItem() { return found("pinned"); },
    async getItem(ctx, itemId) { return found(itemId); },
    async openNotice() { return found("open"); },
    async listItems() { return found({ resources: [{ uri: "stacks://items/item-1", name: "item-1" }] }); },
    async suggestItems(ctx, req) { return found(["item-1", "item-2"].filter((name) => name.startsWith(req.typed))); },
    async countItems() { return found({ pages: 3 }); },
  };
  const tokens = new Map([["read-token", "documents:read"], ["write-token", "documents:read documents:write"]]);
  const admit = async (request) => {
    const needed = stacksServiceMcpScopes(stacksServiceMcpHeader(request, "mcp-method") ?? "", stacksServiceMcpName(request));
    const token = stacksServiceMcpBearerToken(request);
    if (token === undefined) return { denied: needed.length === 0 ? { kind: "noValidCredentials" } : { kind: "noValidCredentials", scope: needed.join(" ") } };
    const held = tokens.get(token);
    if (held === undefined) return { denied: { kind: "noValidCredentials" } };
    if (needed.some((scope) => !held.split(" ").includes(scope))) return { denied: { kind: "insufficientScope", scope: needed.join(" ") } };
    return { ctx: {} };
  };
  const settings = { allowedOrigins: [input.origin], oauth: input.oauth };
  const dispatch = createStacksServiceMcpHttpDispatcher(impl, settings, admit);
  const answers = [];
  for (const row of input.rows) {
    const before = reached;
    const response = await respond(dispatch, { method: row.method, path: row.path, headers: row.headers, body: new TextEncoder().encode(row.body) });
    answers.push({ status: response.status, headers: response.headers, body: new TextDecoder().decode(response.body), reached: reached - before });
  }
  const scopes = input.queries.map(([method, name]) => stacksServiceMcpScopes(method, name ?? undefined));
  console.log(JSON.stringify({ answers, scopes }));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"##;

fn bundle() -> String {
    [
        "import { z, type ZodType, type $brand } from \"zod\";".to_owned(),
        BookRequest::ts_definition(),
        BookRequest::zod_schema(),
        BookInfo::ts_definition(),
        BookInfo::zod_schema(),
        BookError::ts_definition(),
        BookError::zod_schema(),
        StacksServiceSchema::ts_definition(),
        StacksServiceSchema::ts_service(),
        StacksServiceSchema::ts_mcp_content(),
        StacksServiceSchema::ts_mcp_service(),
    ]
    .join("\n\n")
}

fn driven() -> Option<Value> {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(
            REQUIRED_PACKAGES,
            "the emitted TypeScript MCP server over scoped resources",
        );
        return None;
    };
    let literal = Value::String(input().to_string());
    let source = format!(
        "{}\n\n{}\nconst INPUT = {literal};\n{DRIVER}",
        bundle(),
        NODE_RESPOND
    );
    let wrote = ran_with_modules(
        "mcp-scopes",
        RUNTIME_VAR,
        "node",
        "mcp-scopes.mts",
        &source,
        &modules,
    )?;
    Some(serde_json::from_str(wrote.trim()).unwrap())
}

#[test]
fn the_typescript_server_answers_the_scopes_of_a_resource_as_the_rust_dispatcher_does() {
    let Some(report) = driven() else {
        return;
    };
    assert_matches_rust("typescript", &report);
}
