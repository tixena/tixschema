//! What the service publishes to TypeScript, read off the strings themselves.
//!
//! The rendered TypeScript is asserted as text rather than as tokens, because text is what a bundle
//! writes to a `.ts` file and what a TypeScript compiler then reads.
//!
//! **What text assertions do and do not prove.** Nothing here type-checks the bundle. These tests
//! read structure: that a member is required rather than optional, that a name carries the
//! service, that the transport is named only on the far side of the validation check. They cannot
//! prove the emitted file compiles, and they cannot prove that an implementation missing a method
//! is rejected where it reaches the factory — only a compiler can, and one does: the type-check
//! group in `tests/service_schema_typescript_tests/type_check.rs` hands the bundle and two
//! implementations to a real `tsc` wherever one is reachable.

#[cfg(all(feature = "typescript", feature = "zod"))]
mod client_tests;
#[cfg(feature = "dart")]
mod dart_http_client_tests;
#[cfg(feature = "dart")]
mod dart_result_tests;
#[cfg(feature = "dart")]
mod dart_ws_client_tests;
#[cfg(all(feature = "typescript", feature = "zod"))]
mod http_client_tests;
#[cfg(all(feature = "typescript", feature = "zod"))]
mod http_service_tests;
#[cfg(feature = "kotlin")]
mod kotlin_http_client_tests;
#[cfg(feature = "kotlin")]
mod kotlin_ws_client_tests;
#[cfg(all(feature = "typescript", feature = "zod"))]
mod service_tests;
#[cfg(feature = "swift")]
mod swift_http_client_tests;
#[cfg(feature = "swift")]
mod swift_ws_client_tests;
#[cfg(all(feature = "typescript", feature = "zod"))]
mod ws_client_tests;
#[cfg(all(feature = "typescript", feature = "zod"))]
mod ws_server_tests;
#[cfg(all(feature = "typescript", feature = "zod"))]
mod ws_service_tests;

#[cfg(all(feature = "typescript", feature = "zod"))]
use super::client;
#[cfg(feature = "dart")]
use super::dart_http_client;
#[cfg(feature = "dart")]
use super::dart_result;
#[cfg(feature = "dart")]
use super::dart_ws_client;
use super::emit;
#[cfg(all(feature = "typescript", feature = "zod"))]
use super::http_client;
#[cfg(all(feature = "typescript", feature = "zod"))]
use super::http_service;
#[cfg(feature = "kotlin")]
use super::kotlin_http_client;
#[cfg(feature = "kotlin")]
use super::kotlin_ws_client;
#[cfg(feature = "typescript")]
use super::result;
#[cfg(all(feature = "typescript", feature = "zod"))]
use super::service;
#[cfg(feature = "swift")]
use super::swift_http_client;
#[cfg(feature = "swift")]
use super::swift_ws_client;
#[cfg(all(feature = "typescript", feature = "zod"))]
use super::ws_client;
#[cfg(all(feature = "typescript", feature = "zod"))]
use super::ws_server;
#[cfg(all(feature = "typescript", feature = "zod"))]
use super::ws_service;
use crate::service_schema::parse::{ServiceDef, parse_service};
#[cfg(all(feature = "typescript", feature = "zod"))]
use crate::utils::record_unit_struct;
#[cfg(any(feature = "swift", feature = "kotlin"))]
use crate::utils::record_wire_scalar;
use quote::ToTokens as _;
use syn::ItemTrait;

/// A service with one of every input shape and one of every outcome: a named message, an argument
/// list, no arguments at all, and a one-way operation that answers nothing.
const MIXED_SERVICE: &str = "
    pub trait UsageService<Ctx> {
        async fn get_available_balance(
            &self,
            ctx: &Ctx,
            req: AvailableBalanceRequest,
        ) -> Result<AvailableBalanceResponse, BalanceError>;

        async fn expire_credit(
            &self,
            ctx: &Ctx,
            organization_id: OrganizationId,
            credit_id: CreditId,
        ) -> Result<ExpiredCredit, CreditWriteError>;

        async fn sweep(&self, ctx: &Ctx) -> Result<SweepReport, BalanceError>;

        #[service_schema_op(one_way)]
        async fn apply_bundle(&self, ctx: &Ctx, req: ApplyBundleRequest);
    }
";

/// A service exercising every `http(...)` shape: a bodied `POST` naming its own message with a
/// mapped error, a bodyless `GET` with a path carrying two placeholders on a `Named` message, a
/// `header_in` binding, a `header_out` tuple success and two mapped errors (one of which shares a
/// code with a fixed fault status), a one-way `DELETE` whose one argument is the message and the
/// whole placeholder at once, and an operation naming no `http(...)` group at all.
#[cfg(all(feature = "typescript", feature = "zod"))]
const MIXED_HTTP_SERVICE: &str = "
    pub trait DocumentClientService<Ctx> {
        #[service_schema_op(http(
            method = \"POST\",
            path = \"/documents\",
            error_status(TitleTaken = 409)
        ))]
        async fn create_document(
            &self,
            ctx: &Ctx,
            req: CreateDocumentRequest,
        ) -> Result<CreateDocumentResponse, CreateDocumentError>;

        #[service_schema_op(http(
            method = \"GET\",
            path = \"/documents/{document_id}/versions/{version_id}\",
            ok_status = 200,
            header_in(\"range\" = byte_range),
            header_out(\"etag\"),
            error_status(NotFound = 404, VersionGone = 410),
        ))]
        async fn get_version(
            &self,
            ctx: &Ctx,
            document_id: String,
            version_id: String,
            byte_range: Option<String>,
        ) -> Result<(VersionResponse, String), GetVersionError>;

        #[service_schema_op(one_way, http(method = \"DELETE\", path = \"/documents/{document_id}\"))]
        async fn purge_document(&self, ctx: &Ctx, document_id: String);

        async fn sweep_documents(&self, ctx: &Ctx) -> Result<SweepReport, SweepError>;
    }
";

/// A service with a required (non-`Option`) `header_in` binding, to exercise the presence check
/// `MIXED_HTTP_SERVICE`'s `Option<String>` `byte_range` does not get.
#[cfg(all(feature = "typescript", feature = "zod"))]
const REQUIRED_HEADER_HTTP_SERVICE: &str = "
    pub trait DocumentClientService<Ctx> {
        #[service_schema_op(http(
            method = \"GET\",
            path = \"/documents/{document_id}/versions/{version_id}\",
            ok_status = 200,
            header_in(\"range\" = byte_range),
            error_status(NotFound = 404, VersionGone = 410),
        ))]
        async fn get_version(
            &self,
            ctx: &Ctx,
            document_id: String,
            version_id: String,
            byte_range: String,
        ) -> Result<VersionResponse, GetVersionError>;
    }
";

/// A service whose `header_out` tuple carries a required and an optional element, to exercise the
/// paths `MIXED_HTTP_SERVICE`'s single required `etag` element does not: an absent optional header
/// reading `null`, and a present one failing to decode as its declared type faulting.
#[cfg(all(feature = "typescript", feature = "zod"))]
const OPTIONAL_HEADER_OUT_HTTP_SERVICE: &str = "
    pub trait DocumentClientService<Ctx> {
        #[service_schema_op(http(
            method = \"GET\",
            path = \"/documents/{document_id}\",
            header_out(\"etag\"),
            header_out(\"x-age\"),
        ))]
        async fn get_version(
            &self,
            ctx: &Ctx,
            document_id: String,
        ) -> Result<(VersionResponse, String, Option<u32>), GetVersionError>;
    }
";

/// A service exercising every `http(...)` shape the Dart client answers for: a bodied `POST`
/// naming its own message with a mapped error, a bodyless `GET` with a path carrying two
/// placeholders on a `Named` message plus a `header_in` binding and a `header_out` tuple success,
/// a bodyless `GET` whose unbound optional fields (a scalar and a `Vec`) build a query string, a
/// `body = \"bytes\"` `GET` whose one argument is the message and the whole placeholder at once, a
/// one-way `DELETE` in that same single-placeholder shape, and an operation naming no `http(...)`
/// group at all.
#[cfg(feature = "dart")]
const DART_HTTP_SERVICE: &str = "
    pub trait DocumentClientService<Ctx> {
        #[service_schema_op(http(
            method = \"POST\",
            path = \"/documents\",
            error_status(TitleTaken = 409)
        ))]
        async fn create_document(
            &self,
            ctx: &Ctx,
            req: CreateDocumentRequest,
        ) -> Result<CreateDocumentResponse, CreateDocumentError>;

        #[service_schema_op(http(
            method = \"GET\",
            path = \"/documents/{document_id}/versions/{version_id}\",
            ok_status = 200,
            header_in(\"range\" = byte_range),
            header_out(\"etag\"),
            error_status(NotFound = 404, VersionGone = 410),
        ))]
        async fn get_version(
            &self,
            ctx: &Ctx,
            document_id: String,
            version_id: String,
            byte_range: Option<String>,
        ) -> Result<(VersionResponse, String), GetVersionError>;

        #[service_schema_op(http(
            method = \"GET\",
            path = \"/documents/search\",
            error_status(SearchFailed = 500),
        ))]
        async fn search_documents(
            &self,
            ctx: &Ctx,
            q: Option<String>,
            tags: Option<Vec<String>>,
        ) -> Result<SearchResponse, SearchError>;

        #[service_schema_op(http(
            method = \"GET\",
            path = \"/documents/{document_id}/thumbnail\",
            error_status(NotFound = 404),
            body = \"bytes\",
        ))]
        async fn get_thumbnail(
            &self,
            ctx: &Ctx,
            document_id: String,
        ) -> Result<(Vec<u8>, String), ThumbnailError>;

        #[service_schema_op(one_way, http(method = \"DELETE\", path = \"/documents/{document_id}\"))]
        async fn purge_document(&self, ctx: &Ctx, document_id: String);

        async fn sweep_documents(&self, ctx: &Ctx) -> Result<SweepReport, SweepError>;
    }
";

/// A service declaring one `body = \"bytes\"` operation composing `header_out` onto its own tuple:
/// the bytes, their content type, then the declared header.
#[cfg(all(feature = "typescript", feature = "zod"))]
const BYTES_HTTP_SERVICE: &str = "
    pub trait ThumbnailClientService<Ctx> {
        #[service_schema_op(http(
            method = \"GET\",
            path = \"/documents/{document_id}/thumbnail\",
            body = \"bytes\",
            header_out(\"x-document-id\"),
            error_status(NotFound = 404),
        ))]
        async fn get_thumbnail(
            &self,
            ctx: &Ctx,
            document_id: String,
        ) -> Result<(Vec<u8>, String, String), ThumbnailError>;
    }
";

/// A service declaring two `body = \"stream\"` operations: one answering the bare streamed answer,
/// one composing a declared `header_out` onto it. Zod-gated mirror of `DART_STREAM_HTTP_SERVICE`,
/// since a build can carry `zod` without `dart`.
#[cfg(all(feature = "typescript", feature = "zod"))]
const STREAM_HTTP_SERVICE: &str = "
    pub trait ContentClientService<Ctx> {
        #[service_schema_op(http(
            method = \"GET\",
            path = \"/files/{file_id}\",
            body = \"stream\",
            error_status(NotFound = 404),
        ))]
        async fn get_file(
            &self,
            ctx: &Ctx,
            file_id: String,
        ) -> Result<StreamedAnswer, ContentError>;

        #[service_schema_op(http(
            method = \"GET\",
            path = \"/files/{file_id}/tagged\",
            body = \"stream\",
            header_out(\"x-checksum\"),
            error_status(NotFound = 404),
        ))]
        async fn get_tagged_file(
            &self,
            ctx: &Ctx,
            file_id: String,
        ) -> Result<(StreamedAnswer, String), ContentError>;
    }
";

/// A service declaring one `body = \"multipart\"` operation: a path placeholder, two scalar
/// `Generated` fields (one required, one optional) and a `part` binding for the file itself.
#[cfg(all(feature = "typescript", feature = "zod"))]
const MULTIPART_HTTP_SERVICE: &str = "
    pub trait UploadClientService<Ctx> {
        #[service_schema_op(http(
            method = \"POST\",
            path = \"/folders/{folder_id}/documents\",
            body = \"multipart\",
            part(\"file\" = attachment),
            error_status(TooLarge = 413),
        ))]
        async fn upload_document(
            &self,
            ctx: &Ctx,
            folder_id: String,
            title: String,
            description: Option<String>,
            attachment: Box<dyn upload_client_service_schema::BodySource + Send>,
        ) -> Result<UploadResponse, UploadError>;
    }
";

#[cfg(all(feature = "typescript", feature = "zod"))]
const SINGLE_PLACEHOLDER_HTTP_SERVICE: &str = "
    pub trait ConversationClientService<Ctx> {
        #[service_schema_op(http(
            method = \"GET\",
            path = \"/conversations/{conversation_id}/window\",
            error_status(NotFound = 404),
        ))]
        async fn window(
            &self,
            ctx: &Ctx,
            conversation_id: String,
            limit: Option<u32>,
        ) -> Result<WindowPage, WindowError>;

        #[service_schema_op(one_way, http(
            method = \"DELETE\",
            path = \"/conversations/{conversation_id}\",
        ))]
        async fn purge_conversation(&self, ctx: &Ctx, conversation_id: String);
    }
";

/// A service declaring one bodyless `GET` whose macro-generated message carries a placeholder
/// field plus two unbound loose arguments (one numeric, one boolean), and whose operation
/// declares no `error_status` table at all.
#[cfg(all(feature = "typescript", feature = "zod"))]
const QUERY_HTTP_SERVICE: &str = "
    pub trait SearchClientService<Ctx> {
        #[service_schema_op(http(
            method = \"GET\",
            path = \"/search/{category}\",
        ))]
        async fn search(
            &self,
            ctx: &Ctx,
            category: String,
            limit: Option<u32>,
            verbose: Option<bool>,
        ) -> Result<SearchResponse, SearchError>;
    }
";

/// A bodyless `GET` whose macro-generated message's two fields are both bound by the path -
/// nothing left to read off the query - beside a second operation whose `limit` the path leaves
/// unbound.
#[cfg(all(feature = "typescript", feature = "zod"))]
const PATH_BOUND_HTTP_SERVICE: &str = "
    pub trait LabelClientService<Ctx> {
        #[service_schema_op(http(
            method = \"GET\",
            path = \"/orgs/{org}/documents/{document_id}\",
            error_status(NotFound = 404),
        ))]
        async fn get_document(
            &self,
            ctx: &Ctx,
            org: String,
            document_id: String,
        ) -> Result<SearchResponse, SearchError>;

        #[service_schema_op(http(
            method = \"GET\",
            path = \"/orgs/{org}/documents\",
        ))]
        async fn list_documents(
            &self,
            ctx: &Ctx,
            org: String,
            limit: Option<u32>,
        ) -> Result<SearchResponse, SearchError>;
    }
";

/// The same declaration `tests/service_schema_emitted_client_tests/tests.rs` runs the emitted
/// clients against — `ConversationId` a wire-scalar newtype, `purge_conversation` declared first,
/// `window` second.
#[cfg(all(feature = "typescript", feature = "zod"))]
const EMITTED_CLIENT_TEST_SERVICE: &str = "
    pub trait ConversationClientService<Ctx> {
        #[service_schema_op(
            one_way,
            http(method = \"DELETE\", path = \"/v1/conversations/{conversation_id}\",)
        )]
        async fn purge_conversation(&self, ctx: &Ctx, conversation_id: ConversationId);

        #[service_schema_op(http(
            method = \"GET\",
            path = \"/v1/conversations/{conversation_id}/window\",
            error_status(NotFound = 404),
        ))]
        async fn window(
            &self,
            ctx: &Ctx,
            conversation_id: String,
            limit: Option<u32>,
        ) -> Result<WindowPage, WindowError>;
    }
";

/// A service declaring one `body = "bytes"` operation composing `header_out` onto its own tuple:
/// the bytes, their content type, then the declared header. Dart-gated mirror of
/// `BYTES_HTTP_SERVICE`, since a build can carry `dart` without `zod`.
#[cfg(feature = "dart")]
const DART_BYTES_HEADER_OUT_SERVICE: &str = "
    pub trait ThumbnailClientService<Ctx> {
        #[service_schema_op(http(
            method = \"GET\",
            path = \"/documents/{document_id}/thumbnail\",
            body = \"bytes\",
            header_out(\"x-document-id\"),
            error_status(NotFound = 404),
        ))]
        async fn get_thumbnail(
            &self,
            ctx: &Ctx,
            document_id: String,
        ) -> Result<(Vec<u8>, String, String), ThumbnailError>;
    }
";

/// A service declaring two `body = "stream"` operations: one answering the bare streamed answer,
/// one composing a declared `header_out` onto it.
#[cfg(feature = "dart")]
const DART_STREAM_HTTP_SERVICE: &str = "
    pub trait ContentClientService<Ctx> {
        #[service_schema_op(http(
            method = \"GET\",
            path = \"/files/{file_id}\",
            body = \"stream\",
            error_status(NotFound = 404),
        ))]
        async fn get_file(
            &self,
            ctx: &Ctx,
            file_id: String,
        ) -> Result<StreamedAnswer, ContentError>;

        #[service_schema_op(http(
            method = \"GET\",
            path = \"/files/{file_id}/tagged\",
            body = \"stream\",
            header_out(\"x-checksum\"),
            error_status(NotFound = 404),
        ))]
        async fn get_tagged_file(
            &self,
            ctx: &Ctx,
            file_id: String,
        ) -> Result<(StreamedAnswer, String), ContentError>;
    }
";

/// A service declaring one `body = "multipart"` operation: a path placeholder, two scalar
/// `Generated` fields (one required, one optional) and a `part` binding for the file itself.
/// Dart-gated mirror of `MULTIPART_HTTP_SERVICE`.
#[cfg(feature = "dart")]
const DART_MULTIPART_HTTP_SERVICE: &str = "
    pub trait UploadClientService<Ctx> {
        #[service_schema_op(http(
            method = \"POST\",
            path = \"/folders/{folder_id}/documents\",
            body = \"multipart\",
            part(\"file\" = attachment),
            error_status(TooLarge = 413),
        ))]
        async fn upload_document(
            &self,
            ctx: &Ctx,
            folder_id: String,
            title: String,
            description: Option<String>,
            attachment: Box<dyn upload_client_service_schema::BodySource + Send>,
        ) -> Result<UploadResponse, UploadError>;
    }
";

#[cfg(feature = "dart")]
const DART_SINGLE_PLACEHOLDER_HTTP_SERVICE: &str = "
    pub trait ConversationClientService<Ctx> {
        #[service_schema_op(http(
            method = \"GET\",
            path = \"/conversations/{conversation_id}/window\",
            error_status(NotFound = 404),
        ))]
        async fn window(
            &self,
            ctx: &Ctx,
            conversation_id: String,
            limit: Option<u32>,
        ) -> Result<WindowPage, WindowError>;

        #[service_schema_op(one_way, http(
            method = \"DELETE\",
            path = \"/conversations/{conversation_id}\",
        ))]
        async fn purge_conversation(&self, ctx: &Ctx, conversation_id: String);
    }
";

/// A reply operation whose success is `()` — nothing rides in the pair's `Ok` member or the
/// clients' unit arm.
#[cfg(feature = "dart")]
const DART_UNIT_SUCCESS_HTTP_SERVICE: &str = "
    pub trait PingClientService<Ctx> {
        #[service_schema_op(http(method = \"POST\", path = \"/v1/ping\"))]
        async fn ping(&self, ctx: &Ctx, req: PingRequest) -> Result<(), PingError>;
    }
";

/// A reply operation whose success is `()`. Mirror of `DART_UNIT_SUCCESS_HTTP_SERVICE`.
#[cfg(all(feature = "typescript", feature = "zod"))]
const TS_UNIT_SUCCESS_SERVICE: &str = "
    pub trait PingClientService<Ctx> {
        #[service_schema_op(http(method = \"POST\", path = \"/v1/ping\"))]
        async fn ping(&self, ctx: &Ctx, req: PingRequest) -> Result<(), PingError>;
    }
";

/// A service binding headers both ways: a required and an optional `header_in`, a `header_out`
/// tuple with a required and an optional element, an `error_header_out` tuple, and a one-way
/// operation carrying a `header_in` of its own.
#[cfg(all(feature = "typescript", feature = "zod"))]
const TS_HEADER_TUPLE_SERVICE: &str = "
    pub trait VersionService<Ctx> {
        #[service_schema_op(http(
            method = \"POST\",
            path = \"/get\",
            header_in(\"x-tenant\" = tenant),
            header_in(\"x-trace\" = trace),
            header_out(\"etag\"),
            header_out(\"x-age\"),
            error_status(NotFound = 404),
            error_header_out(\"x-reason\"),
        ))]
        async fn get(
            &self,
            ctx: &Ctx,
            id: String,
            tenant: String,
            trace: Option<u32>,
        ) -> Result<(Document, String, Option<u32>), (DocError, Option<String>)>;

        #[service_schema_op(
            one_way,
            http(method = \"POST\", path = \"/touch\", header_in(\"x-tenant\" = tenant))
        )]
        async fn touch(&self, ctx: &Ctx, id: String, tenant: String);
    }
";

/// A reply operation whose success is a unit struct, `PingAck` — reads as a `()` success just as
/// `TS_UNIT_SUCCESS_SERVICE` does, once the struct is recorded the way a declared-above
/// `#[model_schema()]` unit struct would be.
#[cfg(all(feature = "typescript", feature = "zod"))]
const TS_UNIT_STRUCT_SUCCESS_SERVICE: &str = "
    pub trait PingClientService<Ctx> {
        #[service_schema_op(http(method = \"POST\", path = \"/v1/ping\"))]
        async fn ping(&self, ctx: &Ctx, req: PingRequest) -> Result<PingAck, PingError>;
    }
";

/// A service exercising every `http(...)` shape the Kotlin client answers for. Kotlin-gated mirror
/// of `DART_HTTP_SERVICE`, since a build can carry `kotlin` without `dart`.
#[cfg(feature = "kotlin")]
const KOTLIN_HTTP_SERVICE: &str = "
    pub trait DocumentClientService<Ctx> {
        #[service_schema_op(http(
            method = \"POST\",
            path = \"/documents\",
            error_status(TitleTaken = 409)
        ))]
        async fn create_document(
            &self,
            ctx: &Ctx,
            req: CreateDocumentRequest,
        ) -> Result<CreateDocumentResponse, CreateDocumentError>;

        #[service_schema_op(http(
            method = \"GET\",
            path = \"/documents/{document_id}/versions/{version_id}\",
            ok_status = 200,
            header_in(\"range\" = byte_range),
            header_out(\"etag\"),
            error_status(NotFound = 404, VersionGone = 410),
        ))]
        async fn get_version(
            &self,
            ctx: &Ctx,
            document_id: String,
            version_id: String,
            byte_range: Option<String>,
        ) -> Result<(VersionResponse, String), GetVersionError>;

        #[service_schema_op(http(
            method = \"GET\",
            path = \"/documents/search\",
            error_status(SearchFailed = 500),
        ))]
        async fn search_documents(
            &self,
            ctx: &Ctx,
            q: Option<String>,
            tags: Option<Vec<String>>,
        ) -> Result<SearchResponse, SearchError>;

        #[service_schema_op(http(
            method = \"GET\",
            path = \"/documents/{document_id}/thumbnail\",
            error_status(NotFound = 404),
            body = \"bytes\",
        ))]
        async fn get_thumbnail(
            &self,
            ctx: &Ctx,
            document_id: String,
        ) -> Result<(Vec<u8>, String), ThumbnailError>;

        #[service_schema_op(one_way, http(method = \"DELETE\", path = \"/documents/{document_id}\"))]
        async fn purge_document(&self, ctx: &Ctx, document_id: String);

        async fn sweep_documents(&self, ctx: &Ctx) -> Result<SweepReport, SweepError>;
    }
";

/// A service declaring one `body = \"bytes\"` operation composing `header_out` onto its own tuple.
/// Kotlin-gated mirror of `BYTES_HTTP_SERVICE`.
#[cfg(feature = "kotlin")]
const KOTLIN_BYTES_HEADER_OUT_SERVICE: &str = "
    pub trait ThumbnailClientService<Ctx> {
        #[service_schema_op(http(
            method = \"GET\",
            path = \"/documents/{document_id}/thumbnail\",
            body = \"bytes\",
            header_out(\"x-document-id\"),
            error_status(NotFound = 404),
        ))]
        async fn get_thumbnail(
            &self,
            ctx: &Ctx,
            document_id: String,
        ) -> Result<(Vec<u8>, String, String), ThumbnailError>;
    }
";

/// A service declaring two `body = \"stream\"` operations. Kotlin-gated mirror of
/// `DART_STREAM_HTTP_SERVICE`.
#[cfg(feature = "kotlin")]
const KOTLIN_STREAM_HTTP_SERVICE: &str = "
    pub trait ContentClientService<Ctx> {
        #[service_schema_op(http(
            method = \"GET\",
            path = \"/files/{file_id}\",
            body = \"stream\",
            error_status(NotFound = 404),
        ))]
        async fn get_file(
            &self,
            ctx: &Ctx,
            file_id: String,
        ) -> Result<StreamedAnswer, ContentError>;

        #[service_schema_op(http(
            method = \"GET\",
            path = \"/files/{file_id}/tagged\",
            body = \"stream\",
            header_out(\"x-checksum\"),
            error_status(NotFound = 404),
        ))]
        async fn get_tagged_file(
            &self,
            ctx: &Ctx,
            file_id: String,
        ) -> Result<(StreamedAnswer, String), ContentError>;
    }
";

/// A service declaring one `body = \"multipart\"` operation. Kotlin-gated mirror of
/// `MULTIPART_HTTP_SERVICE`.
#[cfg(feature = "kotlin")]
const KOTLIN_MULTIPART_HTTP_SERVICE: &str = "
    pub trait UploadClientService<Ctx> {
        #[service_schema_op(http(
            method = \"POST\",
            path = \"/folders/{folder_id}/documents\",
            body = \"multipart\",
            part(\"file\" = attachment),
            error_status(TooLarge = 413),
        ))]
        async fn upload_document(
            &self,
            ctx: &Ctx,
            folder_id: String,
            title: String,
            description: Option<String>,
            attachment: Box<dyn upload_client_service_schema::BodySource + Send>,
        ) -> Result<UploadResponse, UploadError>;
    }
";

/// The same single-placeholder shape `SINGLE_PLACEHOLDER_HTTP_SERVICE`/
/// `DART_SINGLE_PLACEHOLDER_HTTP_SERVICE` declare, gated on `kotlin` so a build can carry it
/// without `zod` or `dart`.
#[cfg(feature = "kotlin")]
const KOTLIN_SINGLE_PLACEHOLDER_HTTP_SERVICE: &str = "
    pub trait ConversationClientService<Ctx> {
        #[service_schema_op(http(
            method = \"GET\",
            path = \"/conversations/{conversation_id}/window\",
            error_status(NotFound = 404),
        ))]
        async fn window(
            &self,
            ctx: &Ctx,
            conversation_id: String,
            limit: Option<u32>,
        ) -> Result<WindowPage, WindowError>;

        #[service_schema_op(one_way, http(
            method = \"DELETE\",
            path = \"/conversations/{conversation_id}\",
        ))]
        async fn purge_conversation(&self, ctx: &Ctx, conversation_id: String);
    }
";

/// A reply operation whose success is `()`. Kotlin-gated mirror of `DART_UNIT_SUCCESS_HTTP_SERVICE`.
#[cfg(feature = "kotlin")]
const KOTLIN_UNIT_SUCCESS_HTTP_SERVICE: &str = "
    pub trait PingClientService<Ctx> {
        #[service_schema_op(http(method = \"POST\", path = \"/v1/ping\"))]
        async fn ping(&self, ctx: &Ctx, req: PingRequest) -> Result<(), PingError>;
    }
";

/// The design's own running example: a reply operation over a `Named` message answering a
/// declared success or error, and a one-way operation over a branded newtype. Kotlin-gated mirror
/// of `SWIFT_WS_SERVICE`, since it exercises `kotlin_ws_client()` independent of `zod`/`dart`.
#[cfg(feature = "kotlin")]
const KOTLIN_WS_SERVICE: &str = "
    pub trait ConversationClientService<Ctx> {
        #[service_schema_op(http(
            method = \"GET\",
            path = \"/v1/conversations/{conversation_id}/window\",
            error_status(NotFound = 404),
        ))]
        async fn window(
            &self,
            ctx: &Ctx,
            conversation_id: String,
            limit: Option<u32>,
        ) -> Result<WindowPage, WindowError>;

        #[service_schema_op(
            one_way,
            http(method = \"DELETE\", path = \"/v1/conversations/{conversation_id}\",)
        )]
        async fn purge_conversation(&self, ctx: &Ctx, conversation_id: ConversationId);
    }
";

/// A service exercising both operation shapes `ws_rpc` answers for: a reply operation over a
/// `Named` message, and a one-way operation. Named for the design's own running example.
#[cfg(feature = "dart")]
const DART_WS_SERVICE: &str = "
    pub trait Ledger<Ctx> {
        async fn list_transactions(
            &self,
            ctx: &Ctx,
            req: ListTransactionsRequest,
        ) -> Result<TransactionList, ListError>;

        #[service_schema_op(one_way)]
        async fn apply_bundle(&self, ctx: &Ctx, req: ApplyBundleRequest);
    }
";

/// A lone `String` message on a bodied method, and an `i64` success: neither is a generated class.
#[cfg(feature = "dart")]
const DART_PRIMITIVE_SERVICE: &str = "
    pub trait Shelves<Ctx> {
        #[service_schema_op(http(method = \"POST\", path = \"/tally\"))]
        async fn tally(&self, ctx: &Ctx, shelf_id: String) -> Result<i64, TallyError>;
    }
";

/// Every `http(...)` shape the Swift client answers for. Swift-gated mirror of `DART_HTTP_SERVICE`.
#[cfg(feature = "swift")]
const SWIFT_HTTP_SERVICE: &str = "
    pub trait DocumentClientService<Ctx> {
        #[service_schema_op(http(
            method = \"POST\",
            path = \"/documents\",
            error_status(TitleTaken = 409)
        ))]
        async fn create_document(
            &self,
            ctx: &Ctx,
            req: CreateDocumentRequest,
        ) -> Result<CreateDocumentResponse, CreateDocumentError>;

        #[service_schema_op(http(
            method = \"GET\",
            path = \"/documents/{document_id}/versions/{version_id}\",
            ok_status = 200,
            header_in(\"range\" = byte_range),
            header_out(\"etag\"),
            error_status(NotFound = 404, VersionGone = 410),
        ))]
        async fn get_version(
            &self,
            ctx: &Ctx,
            document_id: String,
            version_id: String,
            byte_range: Option<String>,
        ) -> Result<(VersionResponse, String), GetVersionError>;

        #[service_schema_op(http(
            method = \"GET\",
            path = \"/documents/search\",
            error_status(SearchFailed = 500),
        ))]
        async fn search_documents(
            &self,
            ctx: &Ctx,
            q: Option<String>,
            tags: Option<Vec<String>>,
        ) -> Result<SearchResponse, SearchError>;

        #[service_schema_op(http(
            method = \"GET\",
            path = \"/documents/{document_id}/thumbnail\",
            error_status(NotFound = 404),
            body = \"bytes\",
        ))]
        async fn get_thumbnail(
            &self,
            ctx: &Ctx,
            document_id: String,
        ) -> Result<(Vec<u8>, String), ThumbnailError>;

        #[service_schema_op(one_way, http(method = \"DELETE\", path = \"/documents/{document_id}\"))]
        async fn purge_document(&self, ctx: &Ctx, document_id: String);

        async fn sweep_documents(&self, ctx: &Ctx) -> Result<SweepReport, SweepError>;
    }
";

/// A service declaring one `body = \"bytes\"` operation composing `header_out` onto its own tuple:
/// the bytes, their content type, then the declared header. Swift-gated mirror of
/// `DART_BYTES_HEADER_OUT_SERVICE`.
#[cfg(feature = "swift")]
const SWIFT_BYTES_HEADER_OUT_SERVICE: &str = "
    pub trait ThumbnailClientService<Ctx> {
        #[service_schema_op(http(
            method = \"GET\",
            path = \"/documents/{document_id}/thumbnail\",
            body = \"bytes\",
            header_out(\"x-document-id\"),
            error_status(NotFound = 404),
        ))]
        async fn get_thumbnail(
            &self,
            ctx: &Ctx,
            document_id: String,
        ) -> Result<(Vec<u8>, String, String), ThumbnailError>;
    }
";

/// A service declaring two `body = \"stream\"` operations: one answering the bare streamed answer,
/// one composing a declared `header_out` onto it. Swift-gated mirror of `DART_STREAM_HTTP_SERVICE`.
#[cfg(feature = "swift")]
const SWIFT_STREAM_HTTP_SERVICE: &str = "
    pub trait ContentClientService<Ctx> {
        #[service_schema_op(http(
            method = \"GET\",
            path = \"/files/{file_id}\",
            body = \"stream\",
            error_status(NotFound = 404),
        ))]
        async fn get_file(
            &self,
            ctx: &Ctx,
            file_id: String,
        ) -> Result<StreamedAnswer, ContentError>;

        #[service_schema_op(http(
            method = \"GET\",
            path = \"/files/{file_id}/tagged\",
            body = \"stream\",
            header_out(\"x-checksum\"),
            error_status(NotFound = 404),
        ))]
        async fn get_tagged_file(
            &self,
            ctx: &Ctx,
            file_id: String,
        ) -> Result<(StreamedAnswer, String), ContentError>;
    }
";

/// A service declaring one `body = \"multipart\"` operation: a path placeholder, two scalar
/// `Generated` fields (one required, one optional) and a `part` binding for the file itself.
/// Swift-gated mirror of `DART_MULTIPART_HTTP_SERVICE`.
#[cfg(feature = "swift")]
const SWIFT_MULTIPART_HTTP_SERVICE: &str = "
    pub trait UploadClientService<Ctx> {
        #[service_schema_op(http(
            method = \"POST\",
            path = \"/folders/{folder_id}/documents\",
            body = \"multipart\",
            part(\"file\" = attachment),
            error_status(TooLarge = 413),
        ))]
        async fn upload_document(
            &self,
            ctx: &Ctx,
            folder_id: String,
            title: String,
            description: Option<String>,
            attachment: Box<dyn upload_client_service_schema::BodySource + Send>,
        ) -> Result<UploadResponse, UploadError>;
    }
";

/// Swift-gated mirror of `DART_SINGLE_PLACEHOLDER_HTTP_SERVICE`.
#[cfg(feature = "swift")]
const SWIFT_SINGLE_PLACEHOLDER_HTTP_SERVICE: &str = "
    pub trait ConversationClientService<Ctx> {
        #[service_schema_op(http(
            method = \"GET\",
            path = \"/conversations/{conversation_id}/window\",
            error_status(NotFound = 404),
        ))]
        async fn window(
            &self,
            ctx: &Ctx,
            conversation_id: String,
            limit: Option<u32>,
        ) -> Result<WindowPage, WindowError>;

        #[service_schema_op(one_way, http(
            method = \"DELETE\",
            path = \"/conversations/{conversation_id}\",
        ))]
        async fn purge_conversation(&self, ctx: &Ctx, conversation_id: String);
    }
";

/// A reply operation whose success is `()` — nothing rides in the client's success arm. Swift-gated
/// mirror of `DART_UNIT_SUCCESS_HTTP_SERVICE`.
#[cfg(feature = "swift")]
const SWIFT_UNIT_SUCCESS_HTTP_SERVICE: &str = "
    pub trait PingClientService<Ctx> {
        #[service_schema_op(http(method = \"POST\", path = \"/v1/ping\"))]
        async fn ping(&self, ctx: &Ctx, req: PingRequest) -> Result<(), PingError>;
    }
";

/// The design's own running example: a reply operation over a `Named` message answering a
/// declared success or error, and a one-way operation over a branded newtype. Swift-gated, since
/// it exercises `swift_ws_client()` independent of `zod`/`dart`.
#[cfg(feature = "swift")]
const SWIFT_WS_SERVICE: &str = "
    pub trait ConversationClientService<Ctx> {
        #[service_schema_op(http(
            method = \"GET\",
            path = \"/v1/conversations/{conversation_id}/window\",
            error_status(NotFound = 404),
        ))]
        async fn window(
            &self,
            ctx: &Ctx,
            conversation_id: String,
            limit: Option<u32>,
        ) -> Result<WindowPage, WindowError>;

        #[service_schema_op(
            one_way,
            http(method = \"DELETE\", path = \"/v1/conversations/{conversation_id}\",)
        )]
        async fn purge_conversation(&self, ctx: &Ctx, conversation_id: ConversationId);
    }
";

/// A reply operation whose success is `()` — nothing rides in the decoded `Result`'s success arm.
#[cfg(feature = "swift")]
const SWIFT_UNIT_SUCCESS_SERVICE: &str = "
    pub trait PingClientService<Ctx> {
        #[service_schema_op(http(method = \"POST\", path = \"/v1/ping\"))]
        async fn ping(&self, ctx: &Ctx, req: PingRequest) -> Result<(), PingError>;
    }
";

/// A service declaring a `Vec<Option<String>>` `header_in` binding, to exercise a nested optional
/// element inside an otherwise-required header. Swift-gated mirror of the Dart suite's own inline
/// fixture.
#[cfg(feature = "swift")]
const SWIFT_HEADER_VEC_OF_OPTIONS_SERVICE: &str = "
    pub trait TagClientService<Ctx> {
        #[service_schema_op(http(
            method = \"GET\",
            path = \"/tags\",
            header_in(\"x-tags\" = tags),
        ))]
        async fn list_tags(
            &self,
            ctx: &Ctx,
            tags: Vec<Option<String>>,
        ) -> Result<ListTagsResponse, ListTagsError>;
    }
";

/// `ConversationId` is a wire-scalar newtype throughout the fixtures this parses, and the
/// registry recording that is thread-local and reset per test — see [`record_wire_scalar`].
#[cfg(feature = "swift")]
fn swift_ws_client_of(source: &str) -> String {
    record_wire_scalar("ConversationId");
    swift_ws_client::emit(&parsed(source)).join("\n\n")
}

#[cfg(all(feature = "typescript", feature = "zod"))]
fn client_of(source: &str) -> String {
    client::emit(&parsed(source)).join("\n\n")
}

#[cfg(all(feature = "typescript", feature = "zod"))]
fn http_client_of(source: &str) -> String {
    http_client::emit(&parsed(source)).join("\n\n")
}

#[cfg(all(feature = "typescript", feature = "zod"))]
fn http_service_of(source: &str) -> String {
    http_service::emit(&parsed(source)).join("\n\n")
}

#[cfg(all(feature = "typescript", feature = "zod"))]
fn ws_client_of(source: &str) -> String {
    ws_client::emit(&parsed(source)).join("\n\n")
}

#[cfg(all(feature = "typescript", feature = "zod"))]
fn ws_service_of(source: &str) -> String {
    ws_service::emit(&parsed(source)).join("\n\n")
}

#[cfg(all(feature = "typescript", feature = "zod"))]
fn ws_server_of(source: &str) -> String {
    ws_server::emit(&parsed(source)).join("\n\n")
}

#[cfg(feature = "dart")]
fn dart_http_client_of(source: &str) -> String {
    dart_http_client::emit(&parsed(source)).join("\n\n")
}

#[cfg(feature = "dart")]
fn dart_ws_client_of(source: &str) -> String {
    dart_ws_client::emit(&parsed(source)).join("\n\n")
}

#[cfg(feature = "dart")]
fn dart_result_of(source: &str) -> Vec<String> {
    dart_result::emit(&parsed(source))
}

#[cfg(feature = "swift")]
fn swift_http_client_of(source: &str) -> String {
    swift_http_client::emit(&parsed(source)).join("\n\n")
}

#[cfg(feature = "kotlin")]
fn kotlin_http_client_of(source: &str) -> String {
    kotlin_http_client::emit(&parsed(source)).join("\n\n")
}

/// `ConversationId` is a wire-scalar newtype throughout the fixtures this parses, and the
/// registry recording that is thread-local and reset per test — see [`record_wire_scalar`].
#[cfg(feature = "kotlin")]
fn kotlin_ws_client_of(source: &str) -> String {
    record_wire_scalar("ConversationId");
    kotlin_ws_client::emit(&parsed(source)).join("\n\n")
}

fn parsed(source: &str) -> ServiceDef {
    parse_service(&syn::parse_str::<ItemTrait>(source).unwrap()).unwrap()
}

fn registration(source: &str) -> String {
    emit(&parsed(source), false).to_token_stream().to_string()
}

#[cfg(all(feature = "typescript", feature = "zod"))]
fn service_of(source: &str) -> String {
    service::emit(&parsed(source)).join("\n\n")
}

#[cfg(feature = "typescript")]
#[test]
fn a_one_way_operation_gets_no_result_type() {
    let published = result::emit(&parsed(MIXED_SERVICE));
    assert_eq!(published.len(), 3, "got: {published:?}");
    assert!(
        !published
            .iter()
            .any(|ts| ts.contains("UsageServiceApplyBundleResult")),
        "an operation that declared no reply has no arms to join. Got: {published:?}"
    );
}

#[cfg(feature = "typescript")]
#[test]
fn every_declared_message_is_registered_with_the_service() {
    let rendered = registration(MIXED_SERVICE);
    for declared in ["ExpireCreditRequest", "SweepRequest"] {
        assert!(
            rendered.contains(&format!("{declared} :: ts_definition")),
            "a message the macro declared reaches the bundle through the service's own line. \
             Got: {rendered}"
        );
    }
    assert!(
        !rendered.contains("AvailableBalanceRequest :: ts_definition"),
        "the message the author declared is registered by the author, not here. Got: {rendered}"
    );
}

#[test]
fn the_bundle_line_hangs_off_a_struct_named_for_the_service() {
    let rendered = registration(MIXED_SERVICE);
    assert!(
        rendered.contains("pub struct UsageServiceSchema"),
        "got: {rendered}"
    );
    assert!(
        rendered.contains("impl UsageServiceSchema"),
        "got: {rendered}"
    );
}

#[cfg(feature = "typescript")]
#[test]
fn the_fault_s_fields_are_asked_for_rather_than_written_here() {
    let rendered = registration(MIXED_SERVICE);
    for asked in [
        "usage_service_schema :: UsageServiceFaultFields :: ts_definition",
        "usage_service_schema :: UsageServiceFaultKind :: ts_definition",
    ] {
        assert!(
            rendered.contains(asked),
            "the fault's TypeScript comes from the same declaration the Rust dispatcher builds \
             faults from, never from a literal beside it. Got: {rendered}"
        );
    }
    assert!(
        !rendered.contains("export type ServiceFault ="),
        "a hand-maintained literal beside a generated type is how the two drift. Got: {rendered}"
    );
    // The seal is written here and the fields are not, so the sealed alias names the asked-for
    // type and spells no member of its own. A field written here is a field that can drift.
    for member in ["detail:", "field:", "kind:", "operation:"] {
        assert!(
            !rendered.contains(&format!("export type UsageServiceFault = {{\\n  {member}")),
            "the seal adds a brand and nothing else; the members stay the Rust declaration's. \
             Got: {rendered}"
        );
    }
}

/// The two declarations the seal is: a symbol the bundle exports nowhere, and the fault a caller
/// names, declared as the asked-for fields plus one property keyed on that symbol.
///
/// This is what TypeScript is given in place of the private fields Rust has. The Rust fault refuses
/// the literal an implementation would write with `E0451`, and a plain structural object type
/// refuses nothing at all.
#[cfg(feature = "typescript")]
#[test]
fn the_published_fault_is_the_asked_for_fields_under_a_brand_the_bundle_exports_nowhere() {
    let rendered = registration(MIXED_SERVICE);
    assert!(
        rendered.contains("declare const usageServiceFaultSeal: unique symbol;"),
        "a brand keyed on an exported name is a brand anyone can write. Got: {rendered}"
    );
    assert!(
        !rendered.contains("export declare const usageServiceFaultSeal"),
        "an exported symbol is one an implementation can name, and a property it can write. \
         Got: {rendered}"
    );
    assert!(
        rendered.contains(
            "export type UsageServiceFault = UsageServiceFaultFields & {\\n  readonly \
             [usageServiceFaultSeal]: true;\\n};"
        ),
        "the fault a caller names is the fields the Rust declaration published, plus the brand. \
         Got: {rendered}"
    );
    // The README states both halves where it documents the fault — what the seal stops, and the
    // assertion it does not — so the two cannot drift.
    let readme = include_str!("../../../README.md");
    assert!(
        readme.contains("**And the fault type itself refuses to be written.**")
            && readme.contains("declare const usageServiceFaultSeal: unique symbol;")
            && readme.contains("`built as UsageServiceFault` compiles"),
        "the README no longer says what the seal is, or that a type assertion still gets past it"
    );
}

#[cfg(feature = "typescript")]
#[test]
fn the_result_joins_the_two_declared_arms_and_adds_nothing_to_either() {
    let published = result::emit(&parsed(MIXED_SERVICE));
    let found = published
        .iter()
        .find(|ts| ts.contains("export type UsageServiceGetAvailableBalanceResult ="));
    assert!(found.is_some(), "got: {published:?}");
    let balance = found.unwrap();
    assert!(
        balance.contains("| { ok: true; value: AvailableBalanceResponse }"),
        "got: {balance}"
    );
    assert!(
        balance.contains(
            "| { ok: false; error: BalanceError | { isServiceFault: true; fault: \
             UsageServiceFault } };"
        ),
        "got: {balance}"
    );
}

/// The caller-side result type says `value: undefined` for a unit success.
#[cfg(all(feature = "typescript", feature = "zod"))]
#[test]
fn a_unit_success_result_type_says_the_value_is_undefined() {
    let published = result::emit(&parsed(TS_UNIT_SUCCESS_SERVICE));
    let found = published
        .iter()
        .find(|ts| ts.contains("export type PingClientServicePingResult ="));
    assert!(found.is_some(), "got: {published:?}");
    assert!(
        found.unwrap().contains("| { ok: true; value: undefined }"),
        "got: {found:?}"
    );
}

/// The registry's own limit: a name nothing recorded reads as the field-position type, not a unit
/// success — the wire still agrees either way, since both sides write and read `{}` regardless.
#[cfg(all(feature = "typescript", feature = "zod"))]
#[test]
fn an_unrecorded_success_type_is_not_treated_as_a_unit_success() {
    let published = result::emit(&parsed(TS_UNIT_STRUCT_SUCCESS_SERVICE));
    let found = published
        .iter()
        .find(|ts| ts.contains("export type PingClientServicePingResult ="));
    assert!(found.is_some(), "got: {published:?}");
    assert!(
        found.unwrap().contains("| { ok: true; value: PingAck }"),
        "got: {found:?}"
    );
}

/// A unit struct's success position reads exactly like `()`'s own, once the struct is recorded —
/// this fixture is a parsed string rather than a real macro expansion, so the registry needs
/// poking by hand, mirroring `record_wire_scalar`'s own test precedent.
#[cfg(all(feature = "typescript", feature = "zod"))]
#[test]
fn a_unit_struct_success_result_type_says_the_value_is_undefined() {
    record_unit_struct("PingAck");
    let published = result::emit(&parsed(TS_UNIT_STRUCT_SUCCESS_SERVICE));
    let found = published
        .iter()
        .find(|ts| ts.contains("export type PingClientServicePingResult ="));
    assert!(found.is_some(), "got: {published:?}");
    assert!(
        found.unwrap().contains("| { ok: true; value: undefined }"),
        "got: {found:?}"
    );
}

/// `StreamedAnswer` carries no `#[model_schema()]` of its own, so a bare `value: StreamedAnswer`
/// would publish a TypeScript reference nothing declares. The result type stands in the fixed
/// streamed record instead.
#[cfg(all(feature = "typescript", feature = "zod"))]
#[test]
fn the_result_answers_the_streamed_record_rather_than_the_undescribable_rust_type() {
    let published = result::emit(&parsed(STREAM_HTTP_SERVICE));
    let found = published
        .iter()
        .find(|ts| ts.contains("export type ContentClientServiceGetFileResult ="));
    assert!(found.is_some(), "got: {published:?}");
    let result = found.unwrap();
    assert!(
        result.contains(
            "| { ok: true; value: { contentRange: string | undefined; body: \
             ReadableStream<Uint8Array> } }"
        ),
        "got: {result}"
    );
    assert!(
        !result.contains("StreamedAnswer"),
        "the Rust-only seam type never leaks into the published TypeScript. Got: {result}"
    );
}

/// A declared `header_out` wraps the streamed record in a tuple, exactly as the JSON and bytes
/// paths compose theirs.
#[cfg(all(feature = "typescript", feature = "zod"))]
#[test]
fn the_result_composes_header_out_onto_the_streamed_record_in_a_tuple() {
    let published = result::emit(&parsed(STREAM_HTTP_SERVICE));
    let found = published
        .iter()
        .find(|ts| ts.contains("export type ContentClientServiceGetTaggedFileResult ="));
    assert!(found.is_some(), "got: {published:?}");
    let result = found.unwrap();
    assert!(
        result.contains(
            "| { ok: true; value: [{ contentRange: string | undefined; body: \
             ReadableStream<Uint8Array> }, string] }"
        ),
        "got: {result}"
    );
}

#[cfg(feature = "typescript")]
#[test]
fn the_result_takes_its_name_from_the_service_and_the_operation() {
    let published = result::emit(&parsed(MIXED_SERVICE));
    for named in [
        "UsageServiceGetAvailableBalanceResult",
        "UsageServiceExpireCreditResult",
        "UsageServiceSweepResult",
    ] {
        assert!(
            published
                .iter()
                .any(|ts| ts.contains(&format!("export type {named} ="))),
            "a bundle carrying ten services is one flat file, so every name carries the service. \
             Got: {published:?}"
        );
    }
    assert!(
        !published
            .iter()
            .any(|ts| ts.contains("export type SweepResult =")),
        "an unprefixed result collides with any other service declaring the same operation. \
         Got: {published:?}"
    );
}

#[cfg(feature = "typescript")]
#[test]
fn two_operations_naming_unrelated_errors_keep_them_apart() {
    let published = result::emit(&parsed(MIXED_SERVICE));
    let found = published
        .iter()
        .find(|ts| ts.contains("export type UsageServiceExpireCreditResult ="));
    assert!(found.is_some(), "got: {published:?}");
    let expire = found.unwrap();
    assert!(
        expire.contains("error: CreditWriteError |"),
        "an operation's failure arm carries the error that operation declared, not the service's. \
         Got: {expire}"
    );
}

/// The pair that says a client and a dispatcher are published exactly where their check can be.
/// This is the half that runs in a build with the Zod surface; the one below it is the same
/// registration read in a build without it, and neither could pass alone.
#[cfg(all(feature = "typescript", feature = "zod"))]
#[test]
fn a_build_that_publishes_a_schema_publishes_the_client_and_the_dispatcher_that_parse_it() {
    let rendered = registration(MIXED_SERVICE);
    for published in [
        "pub fn ts_client",
        "pub fn ts_http_client",
        "pub fn ts_service",
        "pub fn ts_ws_client",
        "pub fn ts_ws_service",
        "pub fn ts_ws_server",
        "pub fn ts_definition",
    ] {
        assert!(
            rendered.contains(published),
            "a build with a schema to parse against publishes all seven artifacts. \
             Got: {rendered}"
        );
    }
}

/// A build with `typescript` on and `zod` off publishes the service's types and neither seam
/// artifact.
///
/// Both of them parse a message against the `<Message>$Schema` const `#[model_schema()]` writes,
/// and this build writes none. Emitting them without the parse is what this replaced: a client that
/// forwarded whatever it was handed, and a dispatcher that narrowed an unread payload with `as` and
/// gave it to an implementation entitled to assume it was valid. Both compiled, both read like the
/// checked ones, and the Rust half of the same service went on validating — so the two halves
/// disagreed about what they accept and nothing said so.
#[cfg(all(feature = "typescript", not(feature = "zod")))]
#[test]
fn a_build_that_publishes_no_schema_publishes_no_client_and_no_dispatcher() {
    let rendered = registration(MIXED_SERVICE);
    for withheld in [
        "pub fn ts_client",
        "pub fn ts_http_client",
        "pub fn ts_service",
        "pub fn ts_ws_client",
        "pub fn ts_ws_service",
        "pub fn ts_ws_server",
    ] {
        assert!(
            !rendered.contains(withheld),
            "an artifact that cannot hold the guarantee its callers are written against is not \
             published at all. Got: {rendered}"
        );
    }
    assert!(
        rendered.contains("pub fn ts_definition"),
        "the types describe what the Rust half puts on the wire and are true either way. \
         Got: {rendered}"
    );
    // The README states the consequence where it documents the requirement, so the two cannot
    // drift. Newlines collapse to spaces first: the sentence may wrap anywhere in the source.
    let readme = include_str!("../../../README.md").replace('\n', " ");
    assert!(
        readme.contains("**A service that publishes TypeScript needs the `zod` feature too.**")
            && readme.contains(
                "no `<Service>Schema::ts_client()`, no `<Service>Schema::ts_http_client()`, and"
            )
            && readme.contains("no `<Service>Schema::ts_service()`"),
        "the README no longer says what a build without the Zod surface publishes"
    );
}

/// The missing methods are the one thing a reader of this build's registry goes looking for, so
/// the reason they are missing is written on the registry itself rather than left to an
/// `E0599` naming the method and nothing else.
#[cfg(all(feature = "typescript", not(feature = "zod")))]
#[test]
fn a_build_that_publishes_no_client_says_on_the_registry_why_not() {
    let rendered = registration(MIXED_SERVICE);
    for said in [
        "This build publishes no `UsageServiceSchema::ts_client()`, no \
         `UsageServiceSchema::ts_http_client()`, no `UsageServiceSchema::ts_http_service()`, no \
         `UsageServiceSchema::ts_service()`, no `UsageServiceSchema::ts_ws_client()`, no \
         `UsageServiceSchema::ts_ws_service()`, and no `UsageServiceSchema::ts_ws_server()`.",
        "The first six parse a message against the schema",
        "leaves the seven seam artifacts out",
        "only a build with tixschema's `zod` feature writes one",
        "Add `features = [\\\"zod\\\"]` to the tixschema dependency to get them.",
    ] {
        assert!(
            rendered.contains(said),
            "the registry's own rustdoc names the feature and what to add. Got: {rendered}"
        );
    }
}

/// `ts_http_service()` is withheld for the same reason as `ts_service()`: its dispatcher parses a
/// message against a schema this build does not write.
#[cfg(not(feature = "zod"))]
#[test]
fn a_build_that_publishes_no_schema_publishes_no_http_service_either() {
    let rendered = registration(MIXED_SERVICE);
    assert!(
        !rendered.contains("pub fn ts_http_service"),
        "got: {rendered}"
    );
}

/// The counterpart of the refusal above, in the build where the accessor exists.
#[cfg(all(feature = "typescript", feature = "zod"))]
#[test]
fn a_build_that_publishes_a_schema_publishes_ts_http_service() {
    let rendered = registration(MIXED_SERVICE);
    assert!(
        rendered.contains("pub fn ts_http_service"),
        "got: {rendered}"
    );
}

/// What the Zod-less build still publishes, and therefore why it is not refused outright: the
/// message types and the result envelopes describe what the *Rust* dispatcher and client put on the
/// wire, and that half validates in this build exactly as it does in any other.
#[cfg(all(feature = "typescript", not(feature = "zod")))]
#[test]
fn a_build_that_publishes_no_client_still_publishes_every_type_the_wire_carries() {
    let rendered = registration(MIXED_SERVICE);
    for asked in [
        "ExpireCreditRequest :: ts_definition",
        "usage_service_schema :: UsageServiceFaultFields :: ts_definition",
        "declare const usageServiceFaultSeal: unique symbol;",
        "export type UsageServiceGetAvailableBalanceResult =",
    ] {
        assert!(
            rendered.contains(asked),
            "the types are what a hand-written caller of this service reads, and nothing about \
             them depends on the Zod surface. Got: {rendered}"
        );
    }
}

#[cfg(all(feature = "typescript", feature = "zod"))]
#[test]
fn a_declared_message_brings_its_schema_along_with_its_type() {
    let rendered = registration(MIXED_SERVICE);
    assert!(
        rendered.contains("ExpireCreditRequest :: zod_schema"),
        "the schema has no registration line of its own either. Got: {rendered}"
    );
}

#[cfg(feature = "dart")]
#[test]
fn dart_definition_publishes_messages_then_kind_then_fields_then_results() {
    let rendered = registration(MIXED_SERVICE);
    let messages = [
        rendered
            .find("expire_credit_request_dart :: dart_definition")
            .unwrap(),
        rendered
            .find("sweep_request_dart :: dart_definition")
            .unwrap(),
    ];
    let kind = rendered
        .find("usage_service_schema :: usage_service_fault_kind_dart :: dart_definition")
        .unwrap();
    let fields = rendered
        .find("usage_service_schema :: usage_service_fault_fields_dart :: dart_definition")
        .unwrap();
    let results = rendered
        .find("sealed class UsageServiceGetAvailableBalanceResult")
        .unwrap();
    assert!(
        messages.iter().all(|&message| message < kind),
        "got: {rendered}"
    );
    assert!(kind < fields, "got: {rendered}");
    assert!(fields < results, "got: {rendered}");
}

#[cfg(feature = "dart")]
#[test]
fn dart_definition_asks_for_the_generated_messages_a_declared_message_is_registered_by_hand() {
    let rendered = registration(MIXED_SERVICE);
    for declared in ["expire_credit_request_dart", "sweep_request_dart"] {
        assert!(
            rendered.contains(&format!("{declared} :: dart_definition")),
            "a message the macro declared reaches the bundle through the service's own line. \
             Got: {rendered}"
        );
    }
    assert!(
        !rendered.contains("available_balance_request_dart"),
        "the message the author declared is registered by the author, not here. Got: {rendered}"
    );
}

#[cfg(feature = "kotlin")]
#[test]
fn a_build_with_kotlin_publishes_kotlin_http_client() {
    let rendered = registration(MIXED_SERVICE);
    assert!(
        rendered.contains("pub fn kotlin_http_client"),
        "got: {rendered}"
    );
}

#[cfg(not(feature = "kotlin"))]
#[test]
fn a_build_without_kotlin_publishes_no_kotlin_http_client() {
    let rendered = registration(MIXED_SERVICE);
    assert!(
        !rendered.contains("kotlin_http_client"),
        "an artifact behind a feature that is off is absent rather than emitted empty. \
         Got: {rendered}"
    );
}
