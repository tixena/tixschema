//! `LessonService`, which ships the skills folder `tests/mcp_skills` and writes two skills at run
//! time, and the stand-in and the requests the TypeScript, Dart and Kotlin servers are measured
//! against.

use super::mcp_lesson_dispatcher;
use super::mcp_library_rows::{
    HttpAnswer, HttpRow, Match, ORIGIN, Row, assert_answer_valid, request,
};
use super::mcp_library_service::{
    BookError, BookErrorDraft, BookInfo, BookInfoDraft, BookRequest, Sink, book_info_schema,
    book_request_schema,
};
use super::mcp_writer_rows::ready_output;
use super::tests::poll_once;
use crate::mcp_streams::through;
use core::future::{Future, ready};
use serde_json::{Value, json};
use std::panic::resume_unwind;
use tixschema::service_schema;

sinks_to!(mcp_lesson_dispatcher);

/// Teaches the lessons of a school.
#[service_schema(transports = ["mcp"], mcp(skills = "tests/mcp_skills"))]
pub trait LessonService<Ctx> {
    /// Writes the figures of the caller's workspace.
    #[service_schema_op(mcp(skill = "workspace-figures"))]
    async fn figures_skill(&self, ctx: &Ctx, path: String) -> Result<(Vec<u8>, String), BookError>;

    /// Answers how many pages one lesson has.
    async fn get_lesson(&self, ctx: &Ctx, req: BookRequest) -> Result<BookInfo, BookError>;

    /// Writes the release notes of the caller's workspace: "what changed", from its latest changes.
    #[service_schema_op(mcp(skill = "release-notes"))]
    async fn release_notes_skill(&self, ctx: &Ctx, path: String) -> Result<String, BookError>;
}

/// Answers every operation of `LessonService`.
#[derive(Default)]
pub struct Lessons;

impl LessonService<()> for Lessons {
    fn figures_skill(
        &self,
        _ctx: &(),
        path: String,
    ) -> impl Future<Output = Result<(Vec<u8>, String), BookError>> {
        ready(match path.as_str() {
            "SKILL.md" => Ok((b"# Figures\n".to_vec(), "text/markdown".to_owned())),
            "figures.bin" => Ok((vec![0, 255, 128, 1], "application/octet-stream".to_owned())),
            _ => Err(BookErrorDraft::NotFound.validate().unwrap()),
        })
    }

    fn get_lesson(
        &self,
        _ctx: &(),
        req: BookRequest,
    ) -> impl Future<Output = Result<BookInfo, BookError>> {
        ready(if req.book_id == "lesson-1" {
            Ok(BookInfoDraft { pages: 3 }.validate().unwrap())
        } else {
            Err(BookErrorDraft::NotFound.validate().unwrap())
        })
    }

    fn release_notes_skill(
        &self,
        _ctx: &(),
        path: String,
    ) -> impl Future<Output = Result<String, BookError>> {
        ready(match path.as_str() {
            "SKILL.md" => Ok(
                "# Release notes\n\nRead the changes since 2026-11-01, then write one line per change.\n"
                    .to_owned(),
            ),
            "references/changes.md" => Ok("- renamed a document\n".to_owned()),
            "panic" => resume_unwind(Box::new("the handler came apart")),
            _ => Err(BookErrorDraft::NotFound.validate().unwrap()),
        })
    }
}

fn row(name: &'static str, request_id: u64, method: &str, params: &Value) -> Row {
    Row {
        matches: Match::Exact,
        name,
        text: request(&Value::from(request_id), method, params).to_string(),
    }
}

fn read(name: &'static str, request_id: u64, uri: &str) -> Row {
    row(name, request_id, "resources/read", &json!({ "uri": uri }))
}

/// The requests of a run, in order.
pub fn rows() -> Vec<Row> {
    let mut every = listing_rows();
    every.extend(read_rows());
    every
}

fn listing_rows() -> Vec<Row> {
    vec![
        row("server/discover", 1, "server/discover", &json!({})),
        row("tools/list", 2, "tools/list", &json!({})),
        row("skills/list", 3, "skills/list", &json!({})),
        row(
            "a skill of the folder by name",
            4,
            "skills/get",
            &json!({ "name": "code-review" }),
        ),
        row(
            "a skill of the folder by URI",
            5,
            "skills/get",
            &json!({ "uri": "skill://pack-parcels/SKILL.md" }),
        ),
        row(
            "a skill an operation writes by name",
            6,
            "skills/get",
            &json!({ "name": "release-notes" }),
        ),
        row(
            "a skill an operation writes by URI",
            7,
            "skills/get",
            &json!({ "uri": "skill://workspace-figures/SKILL.md" }),
        ),
        row(
            "a skill nobody has",
            8,
            "skills/get",
            &json!({ "name": "no-such-skill" }),
        ),
        row("a get that names nothing", 9, "skills/get", &json!({})),
    ]
}

fn read_rows() -> Vec<Row> {
    vec![
        read(
            "the manifest of a folder skill",
            10,
            "skill://code-review/SKILL.md",
        ),
        read(
            "a text file of a folder skill",
            11,
            "skill://code-review/references/checklist.md",
        ),
        read(
            "a binary file of a folder skill",
            12,
            "skill://code-review/assets/seal.bin",
        ),
        read(
            "a file a folder skill does not hold",
            13,
            "skill://code-review/references/missing.md",
        ),
        read(
            "the manifest of a skill with a block description",
            14,
            "skill://pack-parcels/SKILL.md",
        ),
        read(
            "the manifest of a written skill",
            15,
            "skill://release-notes/SKILL.md",
        ),
        read(
            "a supporting file of a written skill",
            16,
            "skill://release-notes/references/changes.md",
        ),
        read(
            "a file a written skill declines",
            17,
            "skill://release-notes/references/missing.md",
        ),
        read(
            "a file a written skill comes apart on",
            18,
            "skill://release-notes/panic",
        ),
        read(
            "the manifest of a skill written as bytes",
            19,
            "skill://workspace-figures/SKILL.md",
        ),
        read(
            "a file of bytes of a written skill",
            20,
            "skill://workspace-figures/figures.bin",
        ),
        read("a skill nobody has", 21, "skill://no-such-skill/SKILL.md"),
        read("a written skill with no path", 22, "skill://release-notes/"),
        row(
            "a tool",
            23,
            "tools/call",
            &json!({ "name": "get-lesson", "arguments": { "book_id": "lesson-1" } }),
        ),
        row(
            "a read that names nothing",
            24,
            "resources/read",
            &json!({}),
        ),
    ]
}

fn http_row(name: &'static str, message: &Value) -> HttpRow {
    HttpRow {
        body: message.to_string(),
        headers: super::mcp_library_rows::headers_for(message),
        name,
    }
}

/// The skills requests in plain HTTP terms.
pub fn http_rows() -> Vec<HttpRow> {
    vec![
        http_row(
            "a skills listing",
            &request(&Value::from(1_u64), "skills/list", &json!({})),
        ),
        http_row(
            "a skills get",
            &request(
                &Value::from(2_u64),
                "skills/get",
                &json!({ "name": "code-review" }),
            ),
        ),
        http_row(
            "a skills get that asks for progress",
            &with_progress(request(
                &Value::from(3_u64),
                "skills/get",
                &json!({ "name": "code-review" }),
            )),
        ),
        http_row(
            "a skill resource read that asks for progress",
            &with_progress(request(
                &Value::from(4_u64),
                "resources/read",
                &json!({ "uri": "skill://code-review/SKILL.md" }),
            )),
        ),
    ]
}

fn with_progress(mut message: Value) -> Value {
    message["params"]["_meta"]["progressToken"] = json!("skill-1");
    message
}

/// What the Rust dispatcher of `LessonService` answers to `text`.
pub fn lesson_answer(text: &str) -> Option<String> {
    poll_once(super::mcp_lesson_dispatcher::answer(
        text,
        &Lessons,
        |call| call.progress(0.0_f64, None, None),
        &Sink,
        &super::mcp_lesson_dispatcher::McpInFlight::default(),
    ))
    .flatten()
}

/// What the Rust dispatcher answers to the request in plain terms.
pub fn rust_http(row: &HttpRow) -> HttpAnswer {
    let terms = super::mcp_lesson_dispatcher::McpHttpRequest::new(
        "POST".to_owned(),
        "/mcp".to_owned(),
        row.headers.clone(),
        row.body.clone().into_bytes(),
    );
    let settings = super::mcp_lesson_dispatcher::McpHttpSettings::new(&[ORIGIN], None);
    let answered = through(|stream| {
        poll_once(super::mcp_lesson_dispatcher::answer_http(
            &terms,
            &settings,
            stream,
            &Lessons,
            async |_request, _call| Ok(mcp_lesson_dispatcher::McpAdmitted::new(())),
        ))
        .unwrap()
    });
    (
        answered.status(),
        answered.headers().to_vec(),
        String::from_utf8(answered.body().to_vec()).unwrap(),
    )
}

/// Holds the answer to `row` to the schema: the last event of a stream, or the whole body.
fn assert_row_valid(row: &HttpRow, body: &str) {
    let answer = match body.strip_prefix("event: message\ndata: ") {
        Some(_) => body
            .trim_end()
            .rsplit("\n\n")
            .next()
            .and_then(|event| event.strip_prefix("event: message\ndata: "))
            .unwrap(),
        None => body,
    };
    assert_answer_valid(&super::mcp_library_rows::method_of(&row.body), answer);
}

/// Holds each plain HTTP answer of a server to the Rust dispatcher's.
pub fn assert_http(which: &str, answers: &[HttpAnswer]) {
    let rows = http_rows();
    assert_eq!(answers.len(), rows.len(), "{which}");
    for (row, (status, headers, body)) in rows.iter().zip(answers) {
        let (rust_status, rust_headers, rust_body) = rust_http(row);
        assert_eq!(*status, rust_status, "{which}: {}: {body}", row.name);
        let lowered = |held: &[(String, String)]| -> Vec<(String, String)> {
            held.iter()
                .map(|(name, value)| (name.to_ascii_lowercase(), value.clone()))
                .collect()
        };
        assert_eq!(
            lowered(headers),
            lowered(&rust_headers),
            "{which}: {}",
            row.name
        );
        assert_eq!(body, &rust_body, "{which}: {}", row.name);
        assert_row_valid(row, body);
    }
}

#[test]
fn the_rust_dispatcher_answers_the_skills_over_http() {
    for row in http_rows() {
        let (status, _, body) = rust_http(&row);
        assert_eq!(status, 200, "{}", row.name);
        assert_row_valid(&row, &body);
    }
}

#[test]
fn the_rust_dispatcher_streams_a_skill_read_that_asks_for_progress() {
    for row in http_rows()
        .iter()
        .filter(|row| row.name.contains("progress"))
    {
        let (_, headers, body) = rust_http(row);
        assert_eq!(
            headers
                .first()
                .map(|(name, value)| (name.as_str(), value.as_str())),
            Some(("Content-Type", "text/event-stream")),
            "{}",
            row.name
        );
        assert!(body.starts_with("event: message\ndata: "), "{}", row.name);
    }
}

#[test]
fn the_rust_dispatcher_answers_every_row_and_every_answer_passes_the_schema() {
    for row in rows() {
        let answered = lesson_answer(&row.text).unwrap();
        assert_answer_valid(&super::mcp_library_rows::method_of(&row.text), &answered);
    }
}

#[test]
fn the_rust_dispatcher_listens_alike_through_every_entry_point() {
    let (alone, served) = rust_listens_alike!(
        mcp_lesson_dispatcher,
        LessonServiceSchema,
        Lessons,
        ready_output
    );
    assert_eq!(alone, served);
    assert!(alone.is_some());
}
