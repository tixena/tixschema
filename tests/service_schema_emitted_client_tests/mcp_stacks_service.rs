//! `StacksService`, whose resources, template, listing and completion declare scopes, and the
//! stand-in that answers it: the Rust twin the TypeScript, Dart and Kotlin servers are measured
//! against for the scopes they need.

use super::mcp_auth_rows::{
    AUTHORIZATION_SERVER, AuthRow, METADATA_URL, ORIGIN, RESOURCE, SCOPES_SUPPORTED,
    STACKS_QUERIES, STACKS_STATUSES, stacks_rows,
};
use super::mcp_library_service::{
    BookError, BookInfo, BookInfoDraft, BookRequest, book_info_schema, book_request_schema,
};
use super::mcp_stacks_dispatcher;
use super::tests::poll_once;
use core::future::{Future, ready};
use serde_json::{Value, json};
use std::collections::HashMap;
use tixschema::service_schema;

/// Keeps items on stacks.
#[service_schema(transports = ["mcp"])]
pub trait StacksService<Ctx> {
    /// Counts the pages of one book.
    #[service_schema_op(mcp(scopes = ["documents:write"]))]
    async fn count_items(&self, ctx: &Ctx, req: BookRequest) -> Result<BookInfo, BookError>;

    /// One item.
    #[service_schema_op(mcp(resource = "stacks://items/{item_id}", scopes = ["documents:write"]))]
    async fn get_item(&self, ctx: &Ctx, item_id: String) -> Result<String, BookError>;

    /// Lists the items.
    #[service_schema_op(mcp(list_resources, scopes = ["documents:write"]))]
    async fn list_items(
        &self,
        ctx: &Ctx,
        cursor: Option<String>,
    ) -> Result<stacks_service_schema::McpResourcePage, BookError>;

    /// A notice that needs no scope.
    #[service_schema_op(mcp(resource = "stacks://open"))]
    async fn open_notice(&self, ctx: &Ctx) -> Result<String, BookError>;

    /// The one item the template also matches.
    #[service_schema_op(mcp(resource = "stacks://items/pinned", scopes = ["documents:read"]))]
    async fn pinned_item(&self, ctx: &Ctx) -> Result<String, BookError>;

    /// The index of the stacks.
    #[service_schema_op(mcp(resource = "stacks://index", scopes = ["documents:read"]))]
    async fn stacks_index(&self, ctx: &Ctx) -> Result<String, BookError>;

    /// Suggests the items whose identifier starts with what the user typed.
    #[service_schema_op(mcp(
        completes(resource = "get_item", parameter = "item_id"),
        scopes = ["documents:write"]
    ))]
    async fn suggest_items(
        &self,
        ctx: &Ctx,
        typed: String,
        chosen: HashMap<String, String>,
    ) -> Result<Vec<String>, BookError>;
}

/// Answers every operation of `StacksService`.
#[derive(Default)]
pub struct Shelved;

impl StacksService<()> for Shelved {
    fn count_items(
        &self,
        _ctx: &(),
        _req: BookRequest,
    ) -> impl Future<Output = Result<BookInfo, BookError>> {
        ready(Ok(BookInfoDraft { pages: 3 }))
    }

    fn get_item(
        &self,
        _ctx: &(),
        item_id: String,
    ) -> impl Future<Output = Result<String, BookError>> {
        ready(Ok(item_id))
    }

    fn list_items(
        &self,
        _ctx: &(),
        _cursor: Option<String>,
    ) -> impl Future<Output = Result<stacks_service_schema::McpResourcePage, BookError>> {
        ready(Ok(stacks_service_schema::McpResourcePage {
            resources: vec![stacks_service_schema::McpResource {
                uri: "stacks://items/item-1".to_owned(),
                name: "item-1".to_owned(),
                title: None,
                description: None,
                mime_type: None,
                size: None,
            }],
            next_cursor: None,
        }))
    }

    fn open_notice(&self, _ctx: &()) -> impl Future<Output = Result<String, BookError>> {
        ready(Ok("open".to_owned()))
    }

    fn pinned_item(&self, _ctx: &()) -> impl Future<Output = Result<String, BookError>> {
        ready(Ok("pinned".to_owned()))
    }

    fn stacks_index(&self, _ctx: &()) -> impl Future<Output = Result<String, BookError>> {
        ready(Ok("# The stacks".to_owned()))
    }

    fn suggest_items(
        &self,
        _ctx: &(),
        typed: String,
        _chosen: HashMap<String, String>,
    ) -> impl Future<Output = Result<Vec<String>, BookError>> {
        ready(Ok(["item-1", "item-2"]
            .into_iter()
            .filter(|name| name.starts_with(&typed))
            .map(str::to_owned)
            .collect()))
    }
}

/// What the Rust dispatcher of `StacksService` answers to `row`.
fn rust_answer(row: &AuthRow) -> (u16, Vec<(String, String)>, String) {
    rust_auth_answer!(
        mcp_stacks_dispatcher,
        StacksServiceSchema,
        Shelved,
        row,
        ready_now
    )
}

fn ready_now<Answering>(answering: Answering) -> Answering::Output
where
    Answering: Future,
{
    poll_once(answering).unwrap()
}

/// What the drivers are given: the settings, the rows and the scope queries.
pub fn input() -> Value {
    json!({
        "origin": ORIGIN,
        "oauth": {
            "authorizationServers": [AUTHORIZATION_SERVER],
            "resource": RESOURCE,
            "resourceMetadataUrl": METADATA_URL,
            "scopesSupported": [SCOPES_SUPPORTED],
        },
        "rows": stacks_rows().iter().map(|row| json!({
            "body": row.body,
            "headers": row.headers.iter().map(|(name, value)| json!([name, value])).collect::<Vec<_>>(),
            "method": row.method,
            "path": row.path,
        })).collect::<Vec<_>>(),
        "queries": STACKS_QUERIES.iter().map(|(method, name)| json!([method, name])).collect::<Vec<_>>(),
    })
}

/// Holds a report of the answers and the scopes of one server to what the Rust dispatcher says.
pub fn assert_matches_rust(which: &str, report: &Value) {
    let rows = stacks_rows();
    let answers = report["answers"].as_array().unwrap();
    assert_eq!(answers.len(), rows.len(), "{which}");
    for (row, held) in rows.iter().zip(answers) {
        let (status, headers, body) = rust_answer(row);
        assert_eq!(
            held["status"].as_u64().unwrap(),
            u64::from(status),
            "{which}: {}: {held}",
            row.name
        );
        let held_headers: Vec<(String, String)> = held["headers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|pair| {
                (
                    pair[0].as_str().unwrap().to_owned(),
                    pair[1].as_str().unwrap().to_owned(),
                )
            })
            .collect();
        assert_eq!(held_headers, headers, "{which}: {}", row.name);
        assert_eq!(
            held["body"].as_str().unwrap(),
            body,
            "{which}: {}",
            row.name
        );
        if status != 200 {
            assert_eq!(
                held["reached"], 0_i32,
                "{which}: {}: no handler is called",
                row.name
            );
        }
    }
    let statuses: Vec<u64> = answers
        .iter()
        .map(|held| held["status"].as_u64().unwrap())
        .collect();
    assert_eq!(
        statuses,
        STACKS_STATUSES.map(u64::from),
        "{which}: the status of each row"
    );
    let scopes = report["scopes"].as_array().unwrap();
    assert_eq!(scopes.len(), STACKS_QUERIES.len(), "{which}");
    for ((method, name), held) in STACKS_QUERIES.iter().zip(scopes) {
        let rust = StacksServiceSchema::mcp_scopes(method, *name);
        assert_eq!(*held, json!(rust), "{which}: {method} {name:?}");
    }
    assert_eq!(
        StacksServiceSchema::mcp_scopes("resources/read", Some("stacks://items/pinned")),
        ["documents:read"]
    );
    assert_eq!(
        StacksServiceSchema::mcp_scopes("resources/read", Some("stacks://items/other")),
        ["documents:write"]
    );
}

#[test]
fn the_rust_dispatcher_answers_alone_beside_a_second_service_and_over_http_alike() {
    let (alone, served, status) = rust_serves_alike!(
        mcp_stacks_dispatcher,
        StacksServiceSchema,
        Shelved,
        ready_now
    );
    assert_eq!(alone, served);
    assert!(alone.is_some());
    assert_eq!(status, 200);
}

#[test]
fn the_rust_dispatcher_listens_alike_through_every_entry_point() {
    let (alone, served) = rust_listens_alike!(
        mcp_stacks_dispatcher,
        StacksServiceSchema,
        Shelved,
        ready_now
    );
    assert_eq!(alone, served);
    assert!(alone.is_some());
}
