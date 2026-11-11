//! Five MCP services of one operation each, whose server and client in every language write a
//! different set of the module-private helpers: a prompt answering messages, a prompt answering
//! text, a content pair, a list of content blocks and a plain tool.

use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context, Poll, Waker};
use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

/// The error every operation below declares.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum NoteMissing {
    NotFound,
}

/// How many notes a shelf holds.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NoteCount {
    pub notes: u32,
}

/// Reviews one note with the model.
#[service_schema(transports = ["mcp"])]
pub trait NoteReviewService<Ctx> {
    /// Answers the messages that review one note.
    #[service_schema_op(mcp(prompt))]
    async fn review_note(
        &self,
        ctx: &Ctx,
        title: String,
    ) -> Result<Vec<note_review_service_schema::McpPromptMessage>, NoteMissing>;
}

/// Greets the reader of a note.
#[service_schema(transports = ["mcp"])]
pub trait NoteGreetingService<Ctx> {
    /// Greets one reader.
    #[service_schema_op(mcp(prompt))]
    async fn greet(&self, ctx: &Ctx, name: String) -> Result<String, NoteMissing>;
}

/// Answers a note's first page as an image.
#[service_schema(transports = ["mcp"])]
pub trait NotePreviewService<Ctx> {
    /// Answers the first page of one note.
    #[service_schema_op(mcp(content))]
    async fn preview(&self, ctx: &Ctx, title: String) -> Result<(Vec<u8>, String), NoteMissing>;
}

/// Describes a note in blocks.
#[service_schema(transports = ["mcp"])]
pub trait NoteBlocksService<Ctx> {
    /// Answers the blocks that describe one note.
    #[service_schema_op(mcp(content))]
    async fn describe(
        &self,
        ctx: &Ctx,
        title: String,
    ) -> Result<Vec<note_blocks_service_schema::McpContent>, NoteMissing>;
}

/// Counts the notes of a shelf.
#[service_schema(transports = ["mcp"])]
pub trait NoteCountService<Ctx> {
    /// Counts the notes of one shelf.
    async fn count(&self, ctx: &Ctx, label: String) -> Result<NoteCount, NoteMissing>;
}

/// Implements each service above, so that none of the traits is dead code.
pub struct NoteShelf;

impl NoteReviewService<()> for NoteShelf {
    async fn review_note(
        &self,
        _ctx: &(),
        _title: String,
    ) -> Result<Vec<note_review_service_schema::McpPromptMessage>, NoteMissing> {
        ready(()).await;
        Err(NoteMissing::NotFound)
    }
}

impl NoteGreetingService<()> for NoteShelf {
    async fn greet(&self, _ctx: &(), name: String) -> Result<String, NoteMissing> {
        ready(()).await;
        Ok(name)
    }
}

impl NotePreviewService<()> for NoteShelf {
    async fn preview(&self, _ctx: &(), title: String) -> Result<(Vec<u8>, String), NoteMissing> {
        ready(()).await;
        Ok((title.into_bytes(), "image/png".to_owned()))
    }
}

impl NoteBlocksService<()> for NoteShelf {
    async fn describe(
        &self,
        _ctx: &(),
        _title: String,
    ) -> Result<Vec<note_blocks_service_schema::McpContent>, NoteMissing> {
        ready(()).await;
        Ok(Vec::new())
    }
}

impl NoteCountService<()> for NoteShelf {
    async fn count(&self, _ctx: &(), label: String) -> Result<NoteCount, NoteMissing> {
        ready(()).await;
        Ok(NoteCount {
            notes: u32::try_from(label.len()).unwrap(),
        })
    }
}

/// The answer of `answering`, which never suspends, so one poll answers it.
fn poll_once<Answered>(answering: Answered) -> Option<Answered::Output>
where
    Answered: Future,
{
    let mut pinned = pin!(answering);
    match pinned
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(answer) => Some(answer),
        Poll::Pending => None,
    }
}

#[test]
fn the_five_services_are_implementable_and_callable() {
    let title = || "first".to_owned();
    assert_eq!(
        poll_once(NoteReviewService::review_note(&NoteShelf, &(), title())).unwrap(),
        Err(NoteMissing::NotFound)
    );
    assert_eq!(
        poll_once(NoteGreetingService::greet(&NoteShelf, &(), title())).unwrap(),
        Ok(title())
    );
    assert_eq!(
        poll_once(NotePreviewService::preview(&NoteShelf, &(), title()))
            .unwrap()
            .unwrap()
            .1,
        "image/png"
    );
    assert_eq!(
        poll_once(NoteBlocksService::describe(&NoteShelf, &(), title())).unwrap(),
        Ok(Vec::new())
    );
    assert_eq!(
        poll_once(NoteCountService::count(&NoteShelf, &(), title())).unwrap(),
        Ok(NoteCount { notes: 5 })
    );
}
