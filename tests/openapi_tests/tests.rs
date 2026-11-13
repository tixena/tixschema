//! Services that ask for `openapi`, one per operation shape, and the document each answers.

#![cfg(feature = "serde")]

use crate::{
    content_http_rest_transport, folder_http_rest_transport, plain_http_rest_transport,
    search_http_rest_transport,
};
use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context, Poll, Waker};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io::Cursor;
use tixschema::{model_schema, service_schema};

/// Reads back each item a placement writes that no request reaches: every accessor of
/// `IncomingRequest`, and the route table, held to `$routes`.
macro_rules! assert_request_and_routes_read_back {
    ($transport:ident, $routes:expr) => {{
        let request = $transport::IncomingRequest::new(
            "GET".to_owned(),
            "/unused".to_owned(),
            "unused=1".to_owned(),
            vec![("x-trace".to_owned(), "abc".to_owned())],
            b"ignored".to_vec(),
        );
        assert_eq!(request.body(), b"ignored");
        assert_eq!(request.query(), "unused=1");
        assert_eq!(request.header("X-Trace"), Some("abc"));
        assert_eq!(
            request.headers().to_vec(),
            vec![("x-trace".to_owned(), "abc".to_owned())]
        );
        let routes: Vec<(&str, &str, &str, u16, Vec<u16>)> = $transport::ROUTES
            .iter()
            .map(|route| {
                (
                    route.method(),
                    route.path(),
                    route.operation(),
                    route.ok_status(),
                    route.error_statuses().to_vec(),
                )
            })
            .collect();
        let expected: Vec<(&str, &str, &str, u16, Vec<u16>)> = $routes;
        assert_eq!(routes, expected);
    }};
}

/// Builds an `OutgoingResponse` of a placement whose body is bytes, and reads it back.
macro_rules! assert_built_response_reads_back {
    ($transport:ident) => {{
        let built = $transport::OutgoingResponse::new(
            204,
            vec![("x-trace".to_owned(), "abc".to_owned())],
            b"built".to_vec(),
        );
        assert_eq!(built.status(), 204);
        assert_eq!(
            built.headers().to_vec(),
            vec![("x-trace".to_owned(), "abc".to_owned())]
        );
        assert_eq!(built.body(), b"built");
    }};
}

/// The `OpenAPI` 3.1 schema its initiative publishes at
/// `https://spec.openapis.org/oas/3.1/schema/2022-10-07`.
const OPENAPI_SCHEMA: &str = include_str!("oas-3.1-schema-2022-10-07.json");

const COMPONENTS: &str = "#/components/schemas/";

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Document {
    /// The identifier the folder files the document under.
    pub id: String,
    pub tags: Vec<String>,
    pub title: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum DocumentError {
    NotFound,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum CreateDocumentError {
    TitleTaken,
}

/// A message the author declared, whose `document_id` the path binds.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateDocumentRequest {
    pub document_id: String,
    pub title: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct SweepReport {
    pub swept: u32,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum SweepError {
    Busy,
}

/// A JSON body, a path placeholder, a declared message the path reads a member of, an operation
/// with no `http(...)` and a `one_way` one.
#[service_schema(transports = ["http_rest"], openapi(title = "Documents API"))]
pub trait DocumentService<Ctx> {
    #[service_schema_op(http(
        method = "POST",
        path = "/documents",
        error_status(TitleTaken = 409)
    ))]
    async fn create_document(
        &self,
        ctx: &Ctx,
        title: String,
        tags: Vec<String>,
    ) -> Result<Document, CreateDocumentError>;

    #[service_schema_op(http(
        method = "GET",
        path = "/documents/{document_id}",
        error_status(NotFound = 404)
    ))]
    async fn get_document(&self, ctx: &Ctx, document_id: String)
    -> Result<Document, DocumentError>;

    #[service_schema_op(one_way, http(method = "DELETE", path = "/documents/{document_id}"))]
    async fn purge_document(&self, ctx: &Ctx, document_id: String);

    async fn sweep_documents(&self, ctx: &Ctx) -> Result<SweepReport, SweepError>;

    #[service_schema_op(http(
        method = "PUT",
        path = "/documents/{document_id}",
        error_status(NotFound = 404)
    ))]
    async fn update_document(
        &self,
        ctx: &Ctx,
        req: UpdateDocumentRequest,
    ) -> Result<Document, DocumentError>;
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum SearchError {
    FolderMissing,
}

/// A path placeholder and the optional fields a `GET` reads off the query string, with the
/// document answered at a path of its own.
#[service_schema(transports = ["http_rest"], openapi(path = "/v1/openapi.json"))]
pub trait SearchService<Ctx> {
    #[service_schema_op(http(
        method = "GET",
        path = "/folders/{folder_id}/documents",
        error_status(FolderMissing = 404)
    ))]
    async fn search_documents(
        &self,
        ctx: &Ctx,
        folder_id: String,
        text: Option<String>,
        tags: Option<Vec<String>>,
        limit: Option<u32>,
    ) -> Result<Vec<Document>, SearchError>;
}

/// Published under a name of its own, which `components.schemas` keys it by.
#[model_schema(name = "VersionView", enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct VersionResponse {
    pub version_id: u32,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum GetVersionError {
    NotFound,
    VersionGone,
}

/// A message whose `document_id` the path binds, which serde reads under another key.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenamedMemberRequest {
    #[model_schema_prop(minLength = 3)]
    pub document_id: String,
    pub title: String,
}

/// A path that binds a member of a declared message under another key.
#[service_schema(transports = ["http_rest"], openapi)]
pub trait RenamedMemberService<Ctx> {
    #[service_schema_op(http(method = "PUT", path = "/renamed/{document_id}"))]
    async fn renamed(
        &self,
        ctx: &Ctx,
        req: RenamedMemberRequest,
    ) -> Result<SweepReport, SweepError>;
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum RangeError {
    Unsatisfiable,
}

/// `header_in`, `header_out`, `error_header_out` and an `error_status` of two codes.
#[service_schema(transports = ["http_rest"], openapi)]
pub trait VersionService<Ctx> {
    #[service_schema_op(http(
        method = "GET",
        path = "/documents/{document_id}/range",
        header_in("range" = byte_range),
        error_header_out("content-range"),
        error_status(Unsatisfiable = 416),
    ))]
    async fn check_range(
        &self,
        ctx: &Ctx,
        document_id: String,
        byte_range: String,
    ) -> Result<VersionResponse, (RangeError, String)>;

    #[service_schema_op(http(
        method = "GET",
        path = "/documents/{document_id}/versions/{version_id}",
        header_in("range" = byte_range),
        header_out("etag"),
        error_status(NotFound = 404, VersionGone = 410),
    ))]
    async fn get_version(
        &self,
        ctx: &Ctx,
        document_id: String,
        version_id: u32,
        byte_range: Option<String>,
    ) -> Result<(VersionResponse, String), GetVersionError>;
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum ContentError {
    NotFound,
}

/// `body = "bytes"`, and `body = "stream"` with and without a `header_out`.
#[service_schema(transports = ["http_rest"], openapi)]
pub trait ContentService<Ctx> {
    #[service_schema_op(http(
        method = "GET",
        path = "/documents/{document_id}/export",
        body = "stream",
        header_out("etag"),
        error_status(NotFound = 404),
    ))]
    async fn export_document(
        &self,
        ctx: &Ctx,
        document_id: String,
    ) -> Result<(content_service_schema::StreamedAnswer, String), ContentError>;

    #[service_schema_op(http(
        method = "GET",
        path = "/documents/{document_id}/content",
        header_in("range" = byte_range),
        body = "stream",
        error_status(NotFound = 404),
    ))]
    async fn get_content(
        &self,
        ctx: &Ctx,
        document_id: String,
        byte_range: Option<String>,
    ) -> Result<content_service_schema::StreamedAnswer, ContentError>;

    #[service_schema_op(http(
        method = "GET",
        path = "/documents/{document_id}/thumbnail",
        body = "bytes",
        header_out("x-document-id"),
        error_status(NotFound = 404),
    ))]
    async fn get_thumbnail(
        &self,
        ctx: &Ctx,
        document_id: String,
    ) -> Result<(Vec<u8>, String, String), ContentError>;
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct UploadResponse {
    pub document_id: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum UploadError {
    TooLarge,
}

/// `body = "multipart"`: text parts beside a file part.
#[service_schema(transports = ["http_rest"], openapi)]
pub trait UploadService<Ctx> {
    #[service_schema_op(http(
        method = "POST",
        path = "/folders/{folder_id}/documents",
        body = "multipart",
        part("file" = attachment),
        error_status(TooLarge = 413),
    ))]
    async fn upload_document(
        &self,
        ctx: &Ctx,
        folder_id: String,
        title: String,
        description: Option<String>,
        attachment: Box<dyn upload_service_schema::BodySource + Send>,
    ) -> Result<UploadResponse, UploadError>;
}

/// A message that holds itself.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Folder {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<Box<Self>>,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum FolderError {
    NameTaken,
}

/// A recursive type, named by two operations.
#[service_schema(transports = ["http_rest"], openapi)]
pub trait FolderService<Ctx> {
    #[service_schema_op(http(method = "POST", path = "/folders", error_status(NameTaken = 409)))]
    async fn create_folder(&self, ctx: &Ctx, folder: Folder) -> Result<Folder, FolderError>;

    #[service_schema_op(http(
        method = "GET",
        path = "/folders/{name}",
        error_status(NameTaken = 409)
    ))]
    async fn get_folder(&self, ctx: &Ctx, name: String) -> Result<Folder, FolderError>;
}

/// Where a shelf stands: held by the body of both operations of `ShelvingService`.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct ShelfLocation {
    pub aisle: u32,
    pub bay: u32,
}

/// Held by the body of one operation only.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct ShelfNote {
    pub text: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct ShelfMove {
    pub destination: ShelfLocation,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct ShelfPlacement {
    pub location: ShelfLocation,
    pub note: ShelfNote,
}

/// Two bodies that hold one type, and a type only one of them holds.
#[service_schema(transports = ["http_rest"], openapi)]
pub trait ShelvingService<Ctx> {
    #[service_schema_op(http(method = "PUT", path = "/shelves/moves"))]
    async fn move_shelf(&self, ctx: &Ctx, req: ShelfMove) -> Result<SweepReport, SweepError>;

    #[service_schema_op(http(method = "POST", path = "/shelves"))]
    async fn place_shelf(&self, ctx: &Ctx, req: ShelfPlacement) -> Result<SweepReport, SweepError>;
}

/// A service that does not ask for `openapi`.
#[service_schema(transports = ["http_rest"])]
pub trait PlainService<Ctx> {
    #[service_schema_op(http(
        method = "GET",
        path = "/documents/{document_id}",
        error_status(NotFound = 404)
    ))]
    async fn get_document(&self, ctx: &Ctx, document_id: String)
    -> Result<Document, DocumentError>;
}

/// Answers `PlainServiceSchema::openapi()` where the registry has no `openapi()` of its own.
trait NoDocument {
    fn openapi() -> &'static str {
        "none"
    }
}

impl NoDocument for PlainServiceSchema {}

/// One implementation of every service above, whose answers the last two tests read against the
/// documents.
struct Archive;

impl DocumentService<()> for Archive {
    async fn create_document(
        &self,
        _ctx: &(),
        title: String,
        tags: Vec<String>,
    ) -> Result<Document, CreateDocumentError> {
        ready(()).await;
        if title.is_empty() {
            return Err(CreateDocumentErrorDraft::TitleTaken.validate().unwrap());
        }
        Ok(DocumentDraft {
            id: "doc-2".to_owned(),
            tags,
            title,
        }
        .validate()
        .unwrap())
    }

    async fn get_document(
        &self,
        _ctx: &(),
        document_id: String,
    ) -> Result<Document, DocumentError> {
        ready(()).await;
        Ok(DocumentDraft {
            id: document_id,
            tags: vec!["minutes".to_owned()],
            title: "Board meeting".to_owned(),
        }
        .validate()
        .unwrap())
    }

    async fn purge_document(&self, _ctx: &(), _document_id: String) {
        ready(()).await;
    }

    async fn sweep_documents(&self, _ctx: &()) -> Result<SweepReport, SweepError> {
        ready(()).await;
        Err(SweepErrorDraft::Busy.validate().unwrap())
    }

    async fn update_document(
        &self,
        _ctx: &(),
        req: UpdateDocumentRequest,
    ) -> Result<Document, DocumentError> {
        ready(()).await;
        Ok(DocumentDraft {
            id: req.document_id.clone(),
            tags: Vec::new(),
            title: req.title.clone(),
        }
        .validate()
        .unwrap())
    }
}

impl SearchService<()> for Archive {
    async fn search_documents(
        &self,
        _ctx: &(),
        folder_id: String,
        _text: Option<String>,
        _tags: Option<Vec<String>>,
        _limit: Option<u32>,
    ) -> Result<Vec<Document>, SearchError> {
        ready(()).await;
        if folder_id.is_empty() {
            return Err(SearchErrorDraft::FolderMissing.validate().unwrap());
        }
        Ok(Vec::new())
    }
}

impl VersionService<()> for Archive {
    async fn check_range(
        &self,
        _ctx: &(),
        _document_id: String,
        _byte_range: String,
    ) -> Result<VersionResponse, (RangeError, String)> {
        ready(()).await;
        Err((
            RangeErrorDraft::Unsatisfiable.validate().unwrap(),
            "bytes */120".to_owned(),
        ))
    }

    async fn get_version(
        &self,
        _ctx: &(),
        _document_id: String,
        version_id: u32,
        _byte_range: Option<String>,
    ) -> Result<(VersionResponse, String), GetVersionError> {
        ready(()).await;
        if version_id == 0 {
            return Err(GetVersionErrorDraft::VersionGone.validate().unwrap());
        }
        Ok((
            VersionResponseDraft { version_id }.validate().unwrap(),
            "v-7".to_owned(),
        ))
    }
}

impl ContentService<()> for Archive {
    async fn export_document(
        &self,
        _ctx: &(),
        _document_id: String,
    ) -> Result<(content_service_schema::StreamedAnswer, String), ContentError> {
        ready(()).await;
        Err(ContentErrorDraft::NotFound.validate().unwrap())
    }

    async fn get_content(
        &self,
        _ctx: &(),
        _document_id: String,
        _byte_range: Option<String>,
    ) -> Result<content_service_schema::StreamedAnswer, ContentError> {
        ready(()).await;
        Err(ContentErrorDraft::NotFound.validate().unwrap())
    }

    async fn get_thumbnail(
        &self,
        _ctx: &(),
        _document_id: String,
    ) -> Result<(Vec<u8>, String, String), ContentError> {
        ready(()).await;
        Err(ContentErrorDraft::NotFound.validate().unwrap())
    }
}

impl UploadService<()> for Archive {
    async fn upload_document(
        &self,
        _ctx: &(),
        folder_id: String,
        _title: String,
        _description: Option<String>,
        _attachment: Box<dyn upload_service_schema::BodySource + Send>,
    ) -> Result<UploadResponse, UploadError> {
        ready(()).await;
        Ok(UploadResponseDraft {
            document_id: format!("{folder_id}-1"),
        }
        .validate()
        .unwrap())
    }
}

impl RenamedMemberService<()> for Archive {
    async fn renamed(
        &self,
        _ctx: &(),
        req: RenamedMemberRequest,
    ) -> Result<SweepReport, SweepError> {
        ready(()).await;
        if req.title.is_empty() {
            return Err(SweepErrorDraft::Busy.validate().unwrap());
        }
        Ok(SweepReportDraft { swept: 1 }.validate().unwrap())
    }
}

impl ShelvingService<()> for Archive {
    async fn move_shelf(&self, _ctx: &(), req: ShelfMove) -> Result<SweepReport, SweepError> {
        ready(()).await;
        Ok(SweepReportDraft {
            swept: req.destination.aisle,
        }
        .validate()
        .unwrap())
    }

    async fn place_shelf(&self, _ctx: &(), req: ShelfPlacement) -> Result<SweepReport, SweepError> {
        ready(()).await;
        if req.note.text.is_empty() {
            return Err(SweepErrorDraft::Busy.validate().unwrap());
        }
        Ok(SweepReportDraft {
            swept: req.location.bay,
        }
        .validate()
        .unwrap())
    }
}

impl FolderService<()> for Archive {
    async fn create_folder(&self, _ctx: &(), folder: Folder) -> Result<Folder, FolderError> {
        ready(()).await;
        Ok(folder)
    }

    async fn get_folder(&self, _ctx: &(), _name: String) -> Result<Folder, FolderError> {
        ready(()).await;
        Err(FolderErrorDraft::NameTaken.validate().unwrap())
    }
}

impl PlainService<()> for Archive {
    async fn get_document(
        &self,
        _ctx: &(),
        _document_id: String,
    ) -> Result<Document, DocumentError> {
        ready(()).await;
        Err(DocumentErrorDraft::NotFound.validate().unwrap())
    }
}

/// Every error the vendored `OpenAPI` 3.1 schema finds in `document`, each with where it is.
fn openapi_errors(document: &Value) -> Vec<String> {
    let schema: Value = serde_json::from_str(OPENAPI_SCHEMA).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    validator
        .iter_errors(document)
        .map(|error| format!("{error} at {}", error.instance_path()))
        .collect()
}

fn assert_valid(document: &Value) {
    let errors = openapi_errors(document);
    assert!(errors.is_empty(), "{errors:#?}\n{document:#}");
}

/// Every `$ref` anywhere in `value`.
fn references(value: &Value, found: &mut Vec<String>) {
    match value {
        Value::Object(members) => {
            for (key, member) in members {
                if key == "$ref" {
                    found.push(member.as_str().unwrap().to_owned());
                } else {
                    references(member, found);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                references(item, found);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

/// Holds `document`'s `components.schemas` to exactly `names`, and every `$ref` in it to a pointer
/// at one of them.
fn assert_components(document: &Value, names: &[&str]) {
    let schemas = document["components"]["schemas"].as_object().unwrap();
    let mut held: Vec<&str> = schemas.keys().map(String::as_str).collect();
    held.sort_unstable();
    let mut expected = names.to_vec();
    expected.sort_unstable();
    assert_eq!(held, expected, "{document:#}");
    let mut found = Vec::new();
    references(document, &mut found);
    assert_ne!(found, Vec::<String>::new(), "{document:#}");
    for pointer in found {
        assert!(
            pointer.starts_with(COMPONENTS),
            "`{pointer}` points outside components.schemas: {document:#}"
        );
        assert!(
            schemas.contains_key(&pointer[COMPONENTS.len()..]),
            "`{pointer}` names nothing: {document:#}"
        );
    }
}

fn reference(name: &str) -> Value {
    json!({ "$ref": format!("{COMPONENTS}{name}") })
}

fn json_schema_of(response: &Value) -> &Value {
    &response["content"]["application/json"]["schema"]
}

fn statuses(responses: &Value) -> Vec<&str> {
    let mut held: Vec<&str> = responses
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    held.sort_unstable();
    held
}

/// What `answering` answers on its first poll, which is every answer `Archive` gives.
fn answered<Answering>(answering: Answering) -> Answering::Output
where
    Answering: Future,
{
    let mut pinned = pin!(answering);
    let reply = match pinned
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(output) => Some(output),
        Poll::Pending => None,
    };
    reply.unwrap()
}

/// Holds `value` to `schema`, a schema of `document` whose references point into its components.
fn assert_described<Answer>(document: &Value, schema: &Value, value: &Answer)
where
    Answer: Serialize,
{
    let rooted = json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "components": document["components"],
        "allOf": [schema],
    });
    let validator = jsonschema::validator_for(&rooted).unwrap();
    let written = serde_json::to_value(value).unwrap();
    let errors: Vec<String> = validator
        .iter_errors(&written)
        .map(|error| format!("{error} at {}", error.instance_path()))
        .collect();
    assert!(errors.is_empty(), "{errors:#?}\n{written:#}\n{schema:#}");
}

#[test]
fn the_vendored_schema_compiles() {
    let schema: Value = serde_json::from_str(OPENAPI_SCHEMA).unwrap();
    jsonschema::validator_for(&schema).unwrap();
}

#[test]
fn a_document_the_schema_refuses_is_refused() {
    let mut document = DocumentServiceSchema::openapi();
    document["paths"]["/documents"]["post"]["responses"] = json!("none");
    assert_ne!(openapi_errors(&document), Vec::<String>::new());
}

#[test]
fn the_document_names_its_version_its_title_and_its_crate_version() {
    let document = DocumentServiceSchema::openapi();
    assert_eq!(document["openapi"], "3.1.0");
    assert_eq!(document["info"]["title"], "Documents API");
    assert_eq!(document["info"]["version"], env!("CARGO_PKG_VERSION"));
    assert_valid(&document);
}

#[test]
fn a_service_that_writes_no_title_is_titled_by_its_name() {
    let document = SearchServiceSchema::openapi();
    assert_eq!(document["info"]["title"], "SearchService");
}

#[test]
fn a_service_without_openapi_has_no_document() {
    assert_eq!(PlainServiceSchema::openapi(), "none");
}

#[test]
fn a_get_reads_its_placeholder_off_the_path_and_answers_its_success_by_reference() {
    let document = DocumentServiceSchema::openapi();
    let get = &document["paths"]["/documents/{document_id}"]["get"];
    assert_eq!(get["operationId"], "get-document");
    assert_eq!(
        get["parameters"],
        json!([{
            "name": "document_id",
            "in": "path",
            "required": true,
            "schema": { "type": "string" },
        }])
    );
    assert!(get.get("requestBody").is_none(), "{get:#}");
    assert_eq!(
        get["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/Document"
    );
    assert_valid(&document);
}

#[test]
fn a_declared_error_and_a_fault_at_one_status_answer_either() {
    let document = DocumentServiceSchema::openapi();
    let responses = &document["paths"]["/documents/{document_id}"]["get"]["responses"];
    let fault = reference("DocumentServiceFaultFields");
    assert_eq!(
        json_schema_of(&responses["404"]),
        &json!({ "anyOf": [reference("DocumentError"), fault] })
    );
    assert_eq!(json_schema_of(&responses["400"]), &fault);
    assert_eq!(json_schema_of(&responses["500"]), &fault);
    assert_eq!(statuses(responses), ["200", "400", "404", "500"]);
}

#[test]
fn a_json_body_is_the_generated_message_by_reference() {
    let document = DocumentServiceSchema::openapi();
    let post = &document["paths"]["/documents"]["post"];
    assert_eq!(post["operationId"], "create-document");
    assert!(post.get("parameters").is_none(), "{post:#}");
    assert_eq!(
        post["requestBody"],
        json!({
            "required": true,
            "content": { "application/json": { "schema": reference("CreateDocumentRequest") } },
        })
    );
    assert_eq!(
        json_schema_of(&post["responses"]["409"]),
        &reference("CreateDocumentError")
    );
    let message = &document["components"]["schemas"]["CreateDocumentRequest"];
    assert_eq!(
        message["properties"]["tags"],
        json!({ "type": "array", "items": { "type": "string" } })
    );
}

#[test]
fn a_placeholder_of_a_declared_message_reads_the_member_it_binds() {
    let document = DocumentServiceSchema::openapi();
    let put = &document["paths"]["/documents/{document_id}"]["put"];
    assert_eq!(
        put["parameters"],
        json!([{
            "name": "document_id",
            "in": "path",
            "required": true,
            "schema": { "type": "string" },
        }])
    );
    assert_eq!(
        put["requestBody"]["content"]["application/json"]["schema"],
        reference("UpdateDocumentRequest")
    );
}

/// The parameter keeps the name the path writes, and describes the member serde reads under
/// another key.
#[test]
fn a_placeholder_reads_the_schema_of_a_member_under_the_key_serde_reads_it_from() {
    let document = RenamedMemberServiceSchema::openapi();
    assert_eq!(
        document["paths"]["/renamed/{document_id}"]["put"]["parameters"],
        json!([{
            "name": "document_id",
            "in": "path",
            "required": true,
            "schema": { "type": "string", "minLength": 3_u32 },
        }])
    );
    let found = answered(
        Archive.renamed(
            &(),
            RenamedMemberRequestDraft {
                document_id: "doc-1".to_owned(),
                title: "t".to_owned(),
            }
            .validate()
            .unwrap(),
        ),
    );
    assert!(found.is_ok(), "the implementation refused");
}

#[test]
fn an_operation_with_no_http_group_is_a_post_to_its_wire_name_with_every_error_at_422() {
    let document = DocumentServiceSchema::openapi();
    let post = &document["paths"]["/sweep-documents"]["post"];
    assert_eq!(post["operationId"], "sweep-documents");
    assert_eq!(
        post["requestBody"]["content"]["application/json"]["schema"],
        reference("SweepDocumentsRequest")
    );
    assert_eq!(
        json_schema_of(&post["responses"]["200"]),
        &reference("SweepReport")
    );
    assert_eq!(
        json_schema_of(&post["responses"]["422"]),
        &reference("SweepError")
    );
}

#[test]
fn a_one_way_operation_answers_its_status_with_no_body() {
    let document = DocumentServiceSchema::openapi();
    let delete = &document["paths"]["/documents/{document_id}"]["delete"];
    let accepted = &delete["responses"]["204"];
    assert!(accepted["description"].is_string(), "{delete:#}");
    assert!(accepted.get("content").is_none(), "{delete:#}");
    assert_eq!(statuses(&delete["responses"]), ["204", "400", "404", "500"]);
}

#[test]
fn every_type_the_document_service_names_is_written_once_and_referenced_by_name() {
    let document = DocumentServiceSchema::openapi();
    assert_components(
        &document,
        &[
            "CreateDocumentError",
            "CreateDocumentRequest",
            "Document",
            "DocumentError",
            "DocumentServiceFaultFields",
            "SweepDocumentsRequest",
            "SweepError",
            "SweepReport",
            "UpdateDocumentRequest",
        ],
    );
    assert_eq!(
        document["components"]["schemas"]["Document"],
        Document::json_schema()
    );
    assert_eq!(
        document["components"]["schemas"]["Document"]["properties"]["id"]["description"],
        "The identifier the folder files the document under."
    );
}

#[test]
fn the_optional_fields_of_a_get_are_query_parameters() {
    let document = SearchServiceSchema::openapi();
    let get = &document["paths"]["/folders/{folder_id}/documents"]["get"];
    assert_eq!(
        get["parameters"],
        json!([
            { "name": "folder_id", "in": "path", "required": true, "schema": { "type": "string" } },
            { "name": "text", "in": "query", "required": false, "schema": { "type": "string" } },
            {
                "name": "tags",
                "in": "query",
                "required": false,
                "style": "form",
                "explode": false,
                "schema": { "type": "array", "items": { "type": "string" } },
            },
            {
                "name": "limit",
                "in": "query",
                "required": false,
                "schema": { "type": "integer" },
            },
        ])
    );
    assert_eq!(
        json_schema_of(&get["responses"]["200"]),
        &json!({ "type": "array", "items": reference("Document") })
    );
    assert_components(
        &document,
        &["Document", "SearchError", "SearchServiceFaultFields"],
    );
    assert_valid(&document);
}

#[test]
fn a_header_in_is_a_header_parameter_and_a_header_out_a_response_header() {
    let document = VersionServiceSchema::openapi();
    let get = &document["paths"]["/documents/{document_id}/versions/{version_id}"]["get"];
    assert_eq!(get["parameters"][1]["name"], "version_id");
    assert_eq!(get["parameters"][1]["schema"]["type"], "integer");
    assert_eq!(
        get["parameters"][2],
        json!({ "name": "range", "in": "header", "required": false, "schema": { "type": "string" } })
    );
    let success = &get["responses"]["200"];
    assert_eq!(json_schema_of(success), &reference("VersionView"));
    assert_eq!(
        success["headers"],
        json!({ "etag": { "required": true, "schema": { "type": "string" } } })
    );
    assert_eq!(
        json_schema_of(&get["responses"]["410"]),
        &reference("GetVersionError")
    );
    assert_eq!(
        json_schema_of(&get["responses"]["404"]),
        &json!({
            "anyOf": [reference("GetVersionError"), reference("VersionServiceFaultFields")],
        })
    );
    assert_valid(&document);
}

#[test]
fn an_error_header_out_is_a_header_of_the_error_response() {
    let document = VersionServiceSchema::openapi();
    let get = &document["paths"]["/documents/{document_id}/range"]["get"];
    assert_eq!(get["parameters"][1]["required"], true);
    let unsatisfiable = &get["responses"]["416"];
    assert_eq!(json_schema_of(unsatisfiable), &reference("RangeError"));
    assert_eq!(
        unsatisfiable["headers"],
        json!({ "content-range": { "required": true, "schema": { "type": "string" } } })
    );
    assert!(get["responses"]["200"].get("headers").is_none(), "{get:#}");
}

/// A type the bodies of two operations hold is one component, and a type one body holds is written
/// in place.
#[test]
fn a_type_two_operations_bodies_hold_is_one_component() {
    let document = ShelvingServiceSchema::openapi();
    assert_valid(&document);
    assert_components(
        &document,
        &[
            "ShelfLocation",
            "ShelfMove",
            "ShelfPlacement",
            "ShelvingServiceFaultFields",
            "SweepError",
            "SweepReport",
        ],
    );
    let schemas = &document["components"]["schemas"];
    assert_eq!(
        schemas["ShelfMove"]["properties"]["destination"],
        reference("ShelfLocation")
    );
    assert_eq!(
        schemas["ShelfPlacement"]["properties"]["location"],
        reference("ShelfLocation")
    );
    assert_eq!(
        schemas["ShelfPlacement"]["properties"]["note"],
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": { "text": { "type": "string" } },
            "required": ["text"],
        })
    );
    let placed = ShelfPlacementDraft {
        location: ShelfLocationDraft { aisle: 4, bay: 2 }.validate().unwrap(),
        note: ShelfNoteDraft {
            text: "fragile".to_owned(),
        }
        .validate()
        .unwrap(),
    };
    assert_described(&document, &reference("ShelfPlacement"), &placed);
    assert_eq!(
        answered(Archive.place_shelf(&(), placed.validate().unwrap()))
            .ok()
            .map(|report| report.swept),
        Some(2)
    );
    let moved = ShelfMoveDraft {
        destination: ShelfLocationDraft { aisle: 7, bay: 1 }.validate().unwrap(),
    };
    assert_described(&document, &reference("ShelfMove"), &moved);
    assert_eq!(
        answered(Archive.move_shelf(&(), moved.validate().unwrap()))
            .ok()
            .map(|report| report.swept),
        Some(7)
    );
}

#[test]
fn a_renamed_type_is_written_under_the_name_it_is_published_under() {
    let document = VersionServiceSchema::openapi();
    assert_components(
        &document,
        &[
            "GetVersionError",
            "RangeError",
            "VersionServiceFaultFields",
            "VersionView",
        ],
    );
}

#[test]
fn bytes_and_a_stream_answer_an_octet_stream() {
    let document = ContentServiceSchema::openapi();
    let thumbnail =
        &document["paths"]["/documents/{document_id}/thumbnail"]["get"]["responses"]["200"];
    assert_eq!(
        thumbnail["content"],
        json!({ "application/octet-stream": {} })
    );
    assert_eq!(
        thumbnail["headers"],
        json!({ "x-document-id": { "required": true, "schema": { "type": "string" } } })
    );
    let content = &document["paths"]["/documents/{document_id}/content"]["get"];
    assert_eq!(
        content["responses"]["200"]["content"],
        json!({ "application/octet-stream": {} })
    );
    assert_eq!(content["parameters"][1]["in"], "header");
    assert_components(&document, &["ContentError", "ContentServiceFaultFields"]);
    assert_valid(&document);
}

#[test]
fn a_multipart_body_holds_one_property_per_part() {
    let document = UploadServiceSchema::openapi();
    let post = &document["paths"]["/folders/{folder_id}/documents"]["post"];
    assert_eq!(post["parameters"][0]["name"], "folder_id");
    assert_eq!(
        post["requestBody"],
        json!({
            "required": true,
            "content": {
                "multipart/form-data": {
                    "schema": {
                        "type": "object",
                        "properties": {
                            "title": { "type": "string" },
                            "description": { "type": "string" },
                            "file": { "type": "string", "contentMediaType": "application/octet-stream" },
                        },
                        "required": ["title", "file"],
                    },
                },
            },
        })
    );
    assert_components(
        &document,
        &["UploadError", "UploadResponse", "UploadServiceFaultFields"],
    );
    assert_valid(&document);
}

#[test]
fn a_recursive_type_named_twice_is_one_definition_reached_by_reference() {
    let document = FolderServiceSchema::openapi();
    assert_components(
        &document,
        &["Folder", "FolderError", "FolderServiceFaultFields"],
    );
    let folder = &document["components"]["schemas"]["Folder"];
    assert_eq!(
        folder["properties"]["parent"],
        json!({ "anyOf": [reference("Folder"), { "type": "null" }] })
    );
    let post = &document["paths"]["/folders"]["post"];
    assert_eq!(
        post["requestBody"]["content"]["application/json"]["schema"],
        reference("Folder")
    );
    assert_eq!(
        json_schema_of(&post["responses"]["200"]),
        &reference("Folder")
    );
    assert_eq!(
        json_schema_of(&document["paths"]["/folders/{name}"]["get"]["responses"]["200"]),
        &reference("Folder")
    );
    assert_valid(&document);
}

#[test]
fn what_an_implementation_answers_with_json_bodies_is_what_the_document_describes() {
    let documents = DocumentServiceSchema::openapi();
    let at = &documents["paths"]["/documents/{document_id}"];
    let found = answered(DocumentService::get_document(
        &Archive,
        &(),
        "doc-1".to_owned(),
    ));
    assert_described(
        &documents,
        json_schema_of(&at["get"]["responses"]["200"]),
        &found.unwrap(),
    );
    let updated = answered(
        Archive.update_document(
            &(),
            UpdateDocumentRequestDraft {
                document_id: "doc-1".to_owned(),
                title: "Minutes".to_owned(),
            }
            .validate()
            .unwrap(),
        ),
    );
    assert_described(
        &documents,
        json_schema_of(&at["put"]["responses"]["200"]),
        &updated.unwrap(),
    );
    let created = answered(Archive.create_document(&(), String::new(), Vec::new()));
    let created_at = &documents["paths"]["/documents"]["post"]["responses"]["409"];
    assert_described(
        &documents,
        json_schema_of(created_at),
        &created.unwrap_err(),
    );
    let swept = answered(Archive.sweep_documents(&()));
    let swept_at = &documents["paths"]["/sweep-documents"]["post"]["responses"]["422"];
    assert_described(&documents, json_schema_of(swept_at), &swept.unwrap_err());
    answered(Archive.purge_document(&(), "doc-1".to_owned()));

    let search = SearchServiceSchema::openapi();
    let listed = answered(Archive.search_documents(&(), "inbox".to_owned(), None, None, None));
    let listed_at = &search["paths"]["/folders/{folder_id}/documents"]["get"]["responses"]["200"];
    assert_described(&search, json_schema_of(listed_at), &listed.unwrap());
}

#[test]
fn what_an_implementation_answers_beside_headers_and_parts_is_what_the_document_describes() {
    let versions = VersionServiceSchema::openapi();
    let (version, _) = answered(Archive.get_version(&(), "doc-1".to_owned(), 7, None)).unwrap();
    let version_at = &versions["paths"]["/documents/{document_id}/versions/{version_id}"]["get"];
    assert_described(
        &versions,
        json_schema_of(&version_at["responses"]["200"]),
        &version,
    );
    let (refused, _) =
        answered(Archive.check_range(&(), "doc-1".to_owned(), "bytes=500-".to_owned()))
            .unwrap_err();
    let refused_at =
        &versions["paths"]["/documents/{document_id}/range"]["get"]["responses"]["416"];
    assert_described(&versions, json_schema_of(refused_at), &refused);

    let content = ContentServiceSchema::openapi();
    let missing = answered(Archive.get_thumbnail(&(), "doc-1".to_owned())).unwrap_err();
    let missing_at =
        &content["paths"]["/documents/{document_id}/thumbnail"]["get"]["responses"]["404"];
    assert_described(&content, json_schema_of(missing_at), &missing);
    let streamed = answered(Archive.get_content(&(), "doc-1".to_owned(), None));
    let streamed_at =
        &content["paths"]["/documents/{document_id}/content"]["get"]["responses"]["404"];
    assert_described(
        &content,
        json_schema_of(streamed_at),
        &streamed.err().unwrap(),
    );

    let uploads = UploadServiceSchema::openapi();
    let uploaded = answered(Archive.upload_document(
        &(),
        "inbox".to_owned(),
        "Minutes".to_owned(),
        None,
        Box::new(Cursor::new(b"minutes".to_vec())),
    ));
    let uploaded_at =
        &uploads["paths"]["/folders/{folder_id}/documents"]["post"]["responses"]["200"];
    assert_described(&uploads, json_schema_of(uploaded_at), &uploaded.unwrap());

    let folders = FolderServiceSchema::openapi();
    let nested = FolderDraft {
        name: "Board".to_owned(),
        parent: Some(Box::new(
            FolderDraft {
                name: "Archive".to_owned(),
                parent: None,
            }
            .validate()
            .unwrap(),
        )),
    };
    let created_folder = answered(Archive.create_folder(&(), nested.validate().unwrap())).unwrap();
    let folder_at = &folders["paths"]["/folders"]["post"]["responses"]["200"];
    assert_described(&folders, json_schema_of(folder_at), &created_folder);
    let taken = answered(Archive.get_folder(&(), "Board".to_owned())).unwrap_err();
    let taken_at = &folders["paths"]["/folders/{name}"]["get"]["responses"]["409"];
    assert_described(&folders, json_schema_of(taken_at), &taken);

    let documents = DocumentServiceSchema::openapi();
    let plain = answered(PlainService::get_document(
        &Archive,
        &(),
        "doc-1".to_owned(),
    ));
    assert_described(
        &documents,
        json_schema_of(&documents["paths"]["/documents/{document_id}"]["get"]["responses"]["404"]),
        &plain.unwrap_err(),
    );
}

/// Each operation of `document` that lists a `206`, as its method and path.
fn partial_answers(document: &Value) -> Vec<String> {
    let mut found = Vec::new();
    for (path, methods) in document["paths"].as_object().unwrap() {
        for (method, operation) in methods.as_object().unwrap() {
            if operation["responses"].get("206").is_some() {
                found.push(format!("{method} {path}"));
            }
        }
    }
    found.sort_unstable();
    found
}

#[test]
fn a_stream_answers_206_with_its_content_and_a_required_content_range() {
    let document = ContentServiceSchema::openapi();
    let responses = &document["paths"]["/documents/{document_id}/content"]["get"]["responses"];
    assert_eq!(statuses(responses), ["200", "206", "400", "404", "500"]);
    let partial = &responses["206"];
    assert_eq!(
        partial["description"],
        "A partial body: the range the implementation answered."
    );
    assert_eq!(partial["content"], responses["200"]["content"]);
    assert_eq!(
        partial["content"],
        json!({ "application/octet-stream": {} })
    );
    assert_eq!(
        partial["headers"],
        json!({ "content-range": { "required": true, "schema": { "type": "string" } } })
    );
    assert_valid(&document);
}

#[test]
fn a_streams_206_carries_its_header_out_beside_the_content_range() {
    let document = ContentServiceSchema::openapi();
    let responses = &document["paths"]["/documents/{document_id}/export"]["get"]["responses"];
    let text = json!({ "required": true, "schema": { "type": "string" } });
    assert_eq!(responses["200"]["headers"], json!({ "etag": text }));
    assert_eq!(
        responses["206"]["headers"],
        json!({ "content-range": text, "etag": text })
    );
}

#[test]
fn only_a_stream_answers_206() {
    assert_eq!(
        partial_answers(&ContentServiceSchema::openapi()),
        [
            "get /documents/{document_id}/content",
            "get /documents/{document_id}/export",
        ]
    );
    for document in [
        DocumentServiceSchema::openapi(),
        SearchServiceSchema::openapi(),
        VersionServiceSchema::openapi(),
        UploadServiceSchema::openapi(),
        FolderServiceSchema::openapi(),
    ] {
        assert_eq!(
            partial_answers(&document),
            Vec::<String>::new(),
            "{document:#}"
        );
    }
}

fn folder_dispatched(method: &str, path: &str) -> folder_http_rest_transport::OutgoingResponse {
    let request = folder_http_rest_transport::IncomingRequest::new(
        method.to_owned(),
        path.to_owned(),
        String::new(),
        Vec::new(),
        Vec::new(),
    );
    answered(folder_http_rest_transport::dispatch(
        &Archive,
        &(),
        &request,
        &folder_http_rest_transport::DefaultFaultHandler,
    ))
}

fn search_dispatched(path: &str) -> search_http_rest_transport::OutgoingResponse {
    let request = search_http_rest_transport::IncomingRequest::new(
        "GET".to_owned(),
        path.to_owned(),
        String::new(),
        Vec::new(),
        Vec::new(),
    );
    answered(search_http_rest_transport::dispatch(
        &Archive,
        &(),
        &request,
        &search_http_rest_transport::DefaultFaultHandler,
    ))
}

/// Holds `body` to the fault a request no route answers to earns, naming `operation`.
fn assert_unknown_route(status: u16, body: &[u8], operation: &str) {
    assert_eq!(status, 404);
    let fault: Value = serde_json::from_slice(body).unwrap();
    assert_eq!(fault["kind"], "unknown-operation", "{fault:#}");
    assert_eq!(fault["operation"], operation, "{fault:#}");
}

fn json_content_type() -> Vec<(String, String)> {
    vec![("content-type".to_owned(), "application/json".to_owned())]
}

#[test]
fn the_rust_dispatcher_answers_get_at_the_default_path_with_the_document() {
    let response = folder_dispatched("GET", "/openapi.json");
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers().to_vec(), json_content_type());
    let document: Value = serde_json::from_slice(response.body()).unwrap();
    assert_eq!(document["openapi"], "3.1.0");
    assert_eq!(document, FolderServiceSchema::openapi());
    let taken = folder_dispatched("GET", "/folders/Board");
    assert_eq!(taken.status(), 409);
}

#[test]
fn the_rust_dispatcher_answers_the_document_at_the_path_written_and_nowhere_else() {
    let response = search_dispatched("/v1/openapi.json");
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers().to_vec(), json_content_type());
    let document: Value = serde_json::from_slice(response.body()).unwrap();
    assert_eq!(document, SearchServiceSchema::openapi());
    let default = search_dispatched("/openapi.json");
    assert_unknown_route(default.status(), default.body(), "GET /openapi.json");
}

#[test]
fn another_method_at_the_document_path_answers_the_unknown_route_fault() {
    let response = folder_dispatched("POST", "/openapi.json");
    assert_unknown_route(response.status(), response.body(), "POST /openapi.json");
}

#[test]
fn a_service_without_openapi_answers_the_unknown_route_fault_at_the_default_path() {
    let request = plain_http_rest_transport::IncomingRequest::new(
        "GET".to_owned(),
        "/openapi.json".to_owned(),
        String::new(),
        Vec::new(),
        Vec::new(),
    );
    let response = answered(plain_http_rest_transport::dispatch(
        &Archive,
        &(),
        &request,
        &plain_http_rest_transport::DefaultFaultHandler,
    ));
    assert_unknown_route(response.status(), response.body(), "GET /openapi.json");
}

#[test]
fn a_dispatcher_whose_responses_stream_answers_the_document_as_bytes() {
    let request = content_http_rest_transport::IncomingRequest::new(
        "GET".to_owned(),
        "/openapi.json".to_owned(),
        String::new(),
        Vec::new(),
        Vec::new(),
    );
    let response = answered(content_http_rest_transport::dispatch(
        &Archive,
        &(),
        &request,
        &content_http_rest_transport::DefaultFaultHandler,
    ));
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers().to_vec(), json_content_type());
    let document: Value = serde_json::from_slice(&drained(response.into_body())).unwrap();
    assert_eq!(document, ContentServiceSchema::openapi());
}

/// Every byte `body` holds, pulled from its source where it streams.
fn drained(body: content_http_rest_transport::OutgoingBody) -> Vec<u8> {
    match body {
        content_http_rest_transport::OutgoingBody::Bytes(bytes) => bytes,
        content_http_rest_transport::OutgoingBody::Stream(mut source) => {
            let mut pulled = Vec::new();
            let mut chunk = [0_u8; 64];
            loop {
                let read = source.pull(&mut chunk).unwrap();
                if read == 0 {
                    return pulled;
                }
                pulled.extend_from_slice(&chunk[..read]);
            }
        }
    }
}

#[test]
fn each_placement_reads_back_its_requests_and_its_route_table() {
    assert_request_and_routes_read_back!(
        folder_http_rest_transport,
        vec![
            ("POST", "/folders", "create-folder", 200, vec![409]),
            ("GET", "/folders/{name}", "get-folder", 200, vec![409]),
        ]
    );
    assert_request_and_routes_read_back!(
        search_http_rest_transport,
        vec![(
            "GET",
            "/folders/{folder_id}/documents",
            "search-documents",
            200,
            vec![404],
        )]
    );
    assert_request_and_routes_read_back!(
        plain_http_rest_transport,
        vec![(
            "GET",
            "/documents/{document_id}",
            "get-document",
            200,
            vec![404],
        )]
    );
    assert_request_and_routes_read_back!(
        content_http_rest_transport,
        vec![
            (
                "GET",
                "/documents/{document_id}/export",
                "export-document",
                200,
                vec![404],
            ),
            (
                "GET",
                "/documents/{document_id}/content",
                "get-content",
                200,
                vec![404],
            ),
            (
                "GET",
                "/documents/{document_id}/thumbnail",
                "get-thumbnail",
                200,
                vec![404],
            ),
        ]
    );
}

#[test]
fn each_placement_builds_a_response_that_reads_back() {
    assert_built_response_reads_back!(folder_http_rest_transport);
    assert_built_response_reads_back!(search_http_rest_transport);
    assert_built_response_reads_back!(plain_http_rest_transport);
    let streamed = content_http_rest_transport::OutgoingResponse::new(
        206,
        Vec::new(),
        content_http_rest_transport::OutgoingBody::Stream(Box::new(Cursor::new(b"built".to_vec()))),
    );
    assert_eq!(streamed.status(), 206);
    assert_eq!(drained(streamed.into_body()), b"built");
}

#[cfg(all(feature = "typescript", feature = "zod"))]
#[test]
fn the_typescript_server_holds_the_document_text_as_a_string_literal_and_answers_it_as_it_is() {
    let written = DocumentServiceSchema::ts_http_service();
    assert!(!written.contains("__TIXSCHEMA_OPENAPI__"), "{written}");
    let constant = format!(
        "const documentServiceOpenApiDocument = {};",
        Value::String(DocumentServiceSchema::openapi().to_string())
    );
    assert!(written.contains(&constant), "{written}");
    assert!(
        written.contains("body: new TextEncoder().encode(documentServiceOpenApiDocument) }"),
        "{written}"
    );
    assert!(
        !written.contains("JSON.stringify(documentServiceOpenApiDocument"),
        "{written}"
    );
    assert!(
        !PlainServiceSchema::ts_http_service().contains("OpenApiDocument"),
        "a service without openapi holds no document"
    );
}
