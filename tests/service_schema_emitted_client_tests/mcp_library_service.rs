//! `LibraryService` and `DepotService`, which declare resources, templates, a listing and a
//! completion, and `BareService`, which declares none, all asking for `mcp`, and the stand-in that
//! answers them: the Rust twins the TypeScript, Dart and Kotlin servers are measured against.

use core::future::{Future, ready};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::panic::resume_unwind;
use tixschema::{model_schema, service_schema};

macro_rules! sinks_to {
    ($($module:ident),*) => {$(
        impl super::$module::McpEventWriter for Sink {
            fn write(
                &self,
                _message: String,
            ) -> impl Future<Output = Result<(), super::$module::McpClientGone>> {
                ready(Ok(()))
            }
        }
    )*};
}

/// Where progress is written: nowhere, since these runs compare the answer alone.
pub struct Sink;

sinks_to!(
    mcp_library_dispatcher,
    mcp_depot_dispatcher,
    mcp_bare_dispatcher
);

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum BookError {
    NotFound,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BookRequest {
    #[model_schema_prop(minLength = 1)]
    pub book_id: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BookInfo {
    pub pages: u32,
}

/// Lends books from the shelves of a library.
#[service_schema(transports = ["mcp"])]
pub trait LibraryService<Ctx> {
    /// The cover of one book.
    #[service_schema_op(mcp(resource = "library://covers/{book_id}"))]
    async fn book_cover(&self, ctx: &Ctx, book_id: String) -> Result<(Vec<u8>, String), BookError>;

    /// Answers how many pages one book has.
    #[service_schema_op(mcp(title = "Book Pages"))]
    async fn get_book(&self, ctx: &Ctx, req: BookRequest) -> Result<BookInfo, BookError>;

    /// The guide to the library.
    #[service_schema_op(mcp(resource = "guide://library", mime_type = "text/markdown"))]
    async fn library_guide(&self, ctx: &Ctx) -> Result<String, BookError>;

    /// Lists the books, one page at a time.
    #[service_schema_op(mcp(list_resources))]
    async fn list_books(
        &self,
        ctx: &Ctx,
        cursor: Option<String>,
    ) -> Result<library_service_schema::McpResourcePage, BookError>;

    /// Suggests the books whose identifier starts with what the user typed.
    #[service_schema_op(mcp(completes(resource = "book_cover", parameter = "book_id")))]
    async fn suggest_books(
        &self,
        ctx: &Ctx,
        typed: String,
        chosen: HashMap<String, String>,
    ) -> Result<Vec<String>, BookError>;
}

/// Keeps crates and bins in a depot.
#[service_schema(transports = ["mcp"])]
pub trait DepotService<Ctx> {
    /// One bin of one aisle.
    #[service_schema_op(mcp(resource = "depot://aisles/{aisle}/bins/{bin_id}"))]
    async fn get_bin(&self, ctx: &Ctx, aisle: String, bin_id: String) -> Result<String, BookError>;

    /// One crate.
    #[service_schema_op(mcp(
        resource = "depot://crates/{crate_id}",
        mime_type = "application/json",
        ttl_ms = 1000,
        cache_scope = "public"
    ))]
    async fn get_crate(&self, ctx: &Ctx, crate_id: String) -> Result<String, BookError>;

    /// Lists the crates.
    #[service_schema_op(mcp(list_resources))]
    async fn list_crates(
        &self,
        ctx: &Ctx,
        cursor: Option<String>,
    ) -> Result<depot_service_schema::McpResourcePage, BookError>;
}

/// Mirrors the guide and the covers of `LibraryService`, so an endpoint of both shares their URIs.
#[service_schema(transports = ["mcp"])]
pub trait MirrorService<Ctx> {
    /// The covers again.
    #[service_schema_op(mcp(resource = "library://covers/{book_id}"))]
    async fn mirror_cover(&self, ctx: &Ctx, book_id: String) -> Result<String, BookError>;

    /// The guide again.
    #[service_schema_op(mcp(resource = "guide://library"))]
    async fn mirror_guide(&self, ctx: &Ctx) -> Result<String, BookError>;
}

/// Answers a book and nothing else.
#[service_schema(transports = ["mcp"])]
pub trait BareService<Ctx> {
    /// Answers how many pages one plain book has.
    async fn get_plain(&self, ctx: &Ctx, req: BookRequest) -> Result<BookInfo, BookError>;
}

/// Answers every operation of the three services.
#[derive(Default)]
pub struct Stand;

impl Stand {
    fn book_of(book_id: &str) -> Result<BookInfo, BookError> {
        match book_id {
            "book-1" => Ok(seven_pages()),
            "panic" => came_apart(),
            _ => Err(not_found()),
        }
    }
}

// Every method below is synchronous work wrapped in an already-ready `Future`: nothing here waits
// on anything.
impl LibraryService<()> for Stand {
    fn book_cover(
        &self,
        _ctx: &(),
        book_id: String,
    ) -> impl Future<Output = Result<(Vec<u8>, String), BookError>> {
        ready(match book_id.as_str() {
            "book-1" => Ok((b"%PDF-".to_vec(), "application/pdf".to_owned())),
            "book-2" => Ok((vec![0, 255, 128, 1, 2, 3, 4], "image/png".to_owned())),
            "panic" => came_apart(),
            _ => Err(BookError::NotFound),
        })
    }

    fn get_book(
        &self,
        _ctx: &(),
        req: BookRequest,
    ) -> impl Future<Output = Result<BookInfo, BookError>> {
        ready(Self::book_of(&req.book_id))
    }

    fn library_guide(&self, _ctx: &()) -> impl Future<Output = Result<String, BookError>> {
        ready(Ok("# The library".to_owned()))
    }

    fn list_books(
        &self,
        _ctx: &(),
        cursor: Option<String>,
    ) -> impl Future<Output = Result<library_service_schema::McpResourcePage, BookError>> {
        let named = |uri: &str, name: &str| library_service_schema::McpResource {
            uri: uri.to_owned(),
            name: name.to_owned(),
            title: None,
            description: None,
            mime_type: Some("application/pdf".to_owned()),
            size: None,
        };
        ready(match cursor.as_deref() {
            None => Ok(library_service_schema::McpResourcePage {
                resources: vec![named("library://covers/book-1", "cover-1.pdf")],
                next_cursor: Some("page-2".to_owned()),
            }),
            Some("page-2") => Ok(library_service_schema::McpResourcePage {
                resources: vec![library_service_schema::McpResource {
                    size: Some(48_213),
                    title: Some("Second cover".to_owned()),
                    ..named("library://covers/book-2", "cover-2.pdf")
                }],
                next_cursor: None,
            }),
            Some("panic") => came_apart(),
            Some(_) => Err(BookError::NotFound),
        })
    }

    fn suggest_books(
        &self,
        _ctx: &(),
        typed: String,
        chosen: HashMap<String, String>,
    ) -> impl Future<Output = Result<Vec<String>, BookError>> {
        ready(match typed.as_str() {
            "many" => Ok((0..150).map(|at| format!("many-{at}")).collect()),
            "panic" => came_apart(),
            "none" => Err(BookError::NotFound),
            "kind" => Ok(vec![chosen.get("kind").cloned().unwrap_or_default()]),
            _ => Ok(["book-1", "book-2"]
                .into_iter()
                .filter(|name| name.starts_with(&typed))
                .map(str::to_owned)
                .collect()),
        })
    }
}

impl DepotService<()> for Stand {
    fn get_bin(
        &self,
        _ctx: &(),
        aisle: String,
        bin_id: String,
    ) -> impl Future<Output = Result<String, BookError>> {
        ready(Ok(format!("{aisle}/{bin_id}")))
    }

    fn get_crate(
        &self,
        _ctx: &(),
        crate_id: String,
    ) -> impl Future<Output = Result<String, BookError>> {
        ready(if crate_id == "crate-1" {
            Ok("{\"total\":120}".to_owned())
        } else {
            Err(BookError::NotFound)
        })
    }

    fn list_crates(
        &self,
        _ctx: &(),
        cursor: Option<String>,
    ) -> impl Future<Output = Result<depot_service_schema::McpResourcePage, BookError>> {
        ready(match cursor.as_deref() {
            None => Ok(depot_service_schema::McpResourcePage {
                resources: vec![depot_service_schema::McpResource {
                    uri: "depot://crates/crate-1".to_owned(),
                    name: "crate-1".to_owned(),
                    title: None,
                    description: None,
                    mime_type: Some("application/json".to_owned()),
                    size: None,
                }],
                next_cursor: None,
            }),
            Some(_) => Err(BookError::NotFound),
        })
    }
}

impl MirrorService<()> for Stand {
    fn mirror_cover(
        &self,
        _ctx: &(),
        book_id: String,
    ) -> impl Future<Output = Result<String, BookError>> {
        ready(Ok(book_id))
    }

    fn mirror_guide(&self, _ctx: &()) -> impl Future<Output = Result<String, BookError>> {
        ready(Ok("# The library again".to_owned()))
    }
}

impl BareService<()> for Stand {
    fn get_plain(
        &self,
        _ctx: &(),
        req: BookRequest,
    ) -> impl Future<Output = Result<BookInfo, BookError>> {
        ready(Self::book_of(&req.book_id))
    }
}

/// A handler that panics with the sentence the other samples' handlers say.
fn came_apart() -> ! {
    resume_unwind(Box::new("the handler came apart"))
}

fn not_found() -> BookError {
    BookError::NotFound
}

fn seven_pages() -> BookInfo {
    BookInfoDraft { pages: 7 }.into()
}
