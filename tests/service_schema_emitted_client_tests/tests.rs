//! The contract the runtime groups beside this one exercise: one operation whose single argument
//! is the author's own struct, bound to a path with one placeholder, carrying one field the path
//! does not bind.

/// Codec rows declared once so `run_swift.rs` and `run_kotlin.rs` can each round-trip them
/// through their own emitted definition text and this file's own `serde_json` writes.
#[cfg(any(feature = "swift", feature = "kotlin"))]
pub mod swift_codec_fixture {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use tixschema::model_schema;

    /// Row 1: a renamed field, an omitted optional, a present optional, and a `nullable` field
    /// that must still write its key as `null`.
    #[model_schema()]
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CodecOptionsRow {
        #[model_schema_prop(nullable)]
        pub nullable_field: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub omitted_optional: Option<i32>,
        pub plain_field: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub present_optional: Option<i32>,
    }

    /// Row 2a: externally tagged (serde's default once a variant carries data).
    #[model_schema()]
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum CodecExternalTagged {
        Bar(i64),
        Baz,
        Foo { a: String },
    }

    /// Row 2b: internally tagged (`tag = "..."`, no `content`).
    #[model_schema()]
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(tag = "type")]
    pub enum CodecInternalTagged {
        Foo { a: String },
        Reset,
    }

    /// Row 2c: adjacently tagged (`tag = "...", content = "..."`).
    #[model_schema()]
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(tag = "type", content = "value")]
    pub enum CodecAdjacentTagged {
        Cleared,
        Flag(bool),
    }

    /// Row 3: untagged — the decode tries each member in declaration order.
    #[model_schema()]
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(untagged)]
    pub enum CodecUntagged {
        Count { count: i64 },
        Text { text: String },
    }

    /// Row 4: a tuple field, which Swift has no native `Codable` for and emits as a wrapper
    /// struct over an unkeyed container.
    #[model_schema()]
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CodecTuplePoint {
        pub label: String,
        pub pair: (String, u32),
    }

    /// Row 5: a generic struct, bound through conditional conformance.
    #[model_schema(default_types(T = String))]
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CodecEnvelope<T> {
        pub note: String,
        pub value: T,
    }

    /// A plain enum used as a non-string map key below.
    #[model_schema()]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub enum CodecPrimary {
        Primary,
        Secondary,
    }

    /// Row 6: a numeric and an enum map key, each needing the keyed-wrapper codec Swift's native
    /// `Dictionary` conformance does not give a non-`String`/`Int` key. A `bool`-keyed map is
    /// deliberately not exercised here: its emitted decode does not compile.
    #[model_schema()]
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CodecMapKeys {
        pub counters: HashMap<u32, String>,
        pub tiers: HashMap<CodecPrimary, String>,
    }
}

use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context as PollContext, Poll, Waker};
use serde::{Deserialize, Serialize};
use std::io::{self, Read};
use tixschema::{model_schema, service_schema};

/// The full body the one streamed operation in this file answers — it declares no `header_in`
/// binding of its own, so the answer is always the full body.
const STREAMED_CONTENT: &[u8] = b"the quick brown fox jumps over the lazy dog";

/// The most [`ChunkedSlice::read`] ever answers in one call, so draining [`STREAMED_CONTENT`]
/// takes several `pull()` calls rather than one buffered copy.
const CHUNKED_READ_CAP: usize = 5;

/// `conversation_id` is what the placeholder names; `limit` has nowhere to go but the query.
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WindowRequest {
    pub conversation_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WindowPage {
    pub items: Vec<String>,
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum WindowError {
    NotFound,
}

/// A message that already is a wire scalar: the whole message is the segment.
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct ConversationId(pub String);

#[service_schema(transports = ["http_rest"])]
pub trait ConversationClientService<Ctx> {
    #[service_schema_op(
        one_way,
        http(method = "DELETE", path = "/v1/conversations/{conversation_id}",)
    )]
    async fn purge_conversation(&self, ctx: &Ctx, conversation_id: ConversationId);

    #[service_schema_op(http(
        method = "GET",
        path = "/v1/conversations/{conversation_id}/window",
        error_status(NotFound = 404),
    ))]
    async fn window(&self, ctx: &Ctx, req: WindowRequest) -> Result<WindowPage, WindowError>;
}

/// A backend answering the contract, so the trait is implementable rather than merely declared.
struct ConversationBackEnd;

impl ConversationClientService<()> for ConversationBackEnd {
    async fn purge_conversation(&self, _ctx: &(), _conversation_id: ConversationId) {
        ready(()).await;
    }

    async fn window(&self, _ctx: &(), req: WindowRequest) -> Result<WindowPage, WindowError> {
        ready(()).await;
        Ok(WindowPage {
            items: vec![req.conversation_id],
        })
    }
}

// -------------------------------------------------------------------------------------------
// Reader forms: one service per serde form a declared error enum can carry, each with a mapped
// operation and an unmapped one — except the payload-variant form, see `ArchiveClientService`.
// -------------------------------------------------------------------------------------------

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GateStatus {
    pub open: bool,
}

/// Internally tagged: the tag's own `rename_all` inverts `NotFound`, an explicit `rename`
/// inverts `Locked`.
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum GateError {
    #[serde(rename = "sealed")]
    Locked,
    NotFound,
}

#[service_schema(transports = ["http_rest"])]
pub trait GateClientService<Ctx> {
    #[service_schema_op(http(
        method = "GET",
        path = "/gates/{gate_id}",
        error_status(NotFound = 404, Locked = 423),
    ))]
    async fn check_gate(&self, ctx: &Ctx, gate_id: String) -> Result<GateStatus, GateError>;

    /// No `http(...)` group at all — an empty `error_status` table refuses to compile against
    /// an inhabited error type, so the default binding is the only way to reach 422.
    async fn check_gate_unmapped(
        &self,
        ctx: &Ctx,
        gate_id: String,
    ) -> Result<GateStatus, GateError>;
}

struct GateBackEnd;

impl GateClientService<()> for GateBackEnd {
    async fn check_gate(&self, _ctx: &(), gate_id: String) -> Result<GateStatus, GateError> {
        ready(()).await;
        match gate_id.as_str() {
            "missing" => Err(GateError::NotFound),
            "locked" => Err(GateError::Locked),
            _ => Ok(GateStatus { open: true }),
        }
    }

    async fn check_gate_unmapped(
        &self,
        _ctx: &(),
        _gate_id: String,
    ) -> Result<GateStatus, GateError> {
        ready(()).await;
        Err(GateError::NotFound)
    }
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VaultStatus {
    pub open: bool,
}

/// Adjacently tagged: the tag sits beside a `data` key that a unit variant never writes.
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "kebab-case")]
pub enum VaultError {
    #[serde(rename = "bolted")]
    Locked,
    NotFound,
}

#[service_schema(transports = ["http_rest"])]
pub trait VaultClientService<Ctx> {
    #[service_schema_op(http(
        method = "GET",
        path = "/vaults/{vault_id}",
        error_status(NotFound = 404, Locked = 423),
    ))]
    async fn check_vault(&self, ctx: &Ctx, vault_id: String) -> Result<VaultStatus, VaultError>;

    /// No `http(...)` group at all — see `check_gate_unmapped`'s own note.
    async fn check_vault_unmapped(
        &self,
        ctx: &Ctx,
        vault_id: String,
    ) -> Result<VaultStatus, VaultError>;
}

struct VaultBackEnd;

impl VaultClientService<()> for VaultBackEnd {
    async fn check_vault(&self, _ctx: &(), vault_id: String) -> Result<VaultStatus, VaultError> {
        ready(()).await;
        match vault_id.as_str() {
            "missing" => Err(VaultError::NotFound),
            "locked" => Err(VaultError::Locked),
            _ => Ok(VaultStatus { open: true }),
        }
    }

    async fn check_vault_unmapped(
        &self,
        _ctx: &(),
        _vault_id: String,
    ) -> Result<VaultStatus, VaultError> {
        ready(()).await;
        Err(VaultError::NotFound)
    }
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArchiveStatus {
    pub archived: bool,
}

/// Externally tagged with a payload variant: `NotFound` writes as a bare string, `Locked`
/// writes as the object's sole (renamed) key holding its content. A payload variant cannot be
/// named in an `error_status` table, so this enum is read through `{Enum}$Variant` directly.
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ArchiveError {
    #[serde(rename = "vault-locked")]
    Locked {
        retry_after: u32,
    },
    NotFound,
}

#[service_schema(transports = ["http_rest"])]
pub trait ArchiveClientService<Ctx> {
    /// No `http(...)` group at all — `ArchiveError` carries a payload variant, so it cannot
    /// appear in any `error_status` table at all, mapped or empty.
    async fn check_archive(
        &self,
        ctx: &Ctx,
        archive_id: String,
    ) -> Result<ArchiveStatus, ArchiveError>;
}

struct ArchiveBackEnd;

impl ArchiveClientService<()> for ArchiveBackEnd {
    async fn check_archive(
        &self,
        _ctx: &(),
        archive_id: String,
    ) -> Result<ArchiveStatus, ArchiveError> {
        ready(()).await;
        match archive_id.as_str() {
            "missing" => Err(ArchiveError::NotFound),
            "locked" => Err(ArchiveError::Locked { retry_after: 30 }),
            _ => Ok(ArchiveStatus { archived: true }),
        }
    }
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SealStatus {
    pub sealed: bool,
}

/// Unit-only, no tag: serde writes the value itself as the variant's (possibly renamed) wire
/// name, a bare JSON string.
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SealError {
    #[serde(rename = "sealed-shut")]
    Locked,
    NotFound,
}

#[service_schema(transports = ["http_rest"])]
pub trait SealClientService<Ctx> {
    #[service_schema_op(http(
        method = "GET",
        path = "/seals/{seal_id}",
        error_status(NotFound = 404, Locked = 423),
    ))]
    async fn check_seal(&self, ctx: &Ctx, seal_id: String) -> Result<SealStatus, SealError>;

    /// No `http(...)` group at all — see `check_gate_unmapped`'s own note.
    async fn check_seal_unmapped(
        &self,
        ctx: &Ctx,
        seal_id: String,
    ) -> Result<SealStatus, SealError>;
}

struct SealBackEnd;

impl SealClientService<()> for SealBackEnd {
    async fn check_seal(&self, _ctx: &(), seal_id: String) -> Result<SealStatus, SealError> {
        ready(()).await;
        match seal_id.as_str() {
            "missing" => Err(SealError::NotFound),
            "locked" => Err(SealError::Locked),
            _ => Ok(SealStatus { sealed: true }),
        }
    }

    async fn check_seal_unmapped(
        &self,
        _ctx: &(),
        _seal_id: String,
    ) -> Result<SealStatus, SealError> {
        ready(()).await;
        Err(SealError::NotFound)
    }
}

// -------------------------------------------------------------------------------------------
// A macro-generated message read off the query on a bodyless `GET`, and off the whole JSON
// body on a `POST` at the same path — the coerced fields echoed back for a driver to read.
// -------------------------------------------------------------------------------------------

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SearchEcho {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verbose: Option<bool>,
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum SearchError {
    DbError,
}

#[service_schema(transports = ["http_rest"])]
pub trait SearchClientService<Ctx> {
    #[service_schema_op(http(method = "GET", path = "/search", error_status(DbError = 500)))]
    async fn search(
        &self,
        ctx: &Ctx,
        limit: Option<u32>,
        verbose: Option<bool>,
    ) -> Result<SearchEcho, SearchError>;

    #[service_schema_op(http(method = "POST", path = "/search", error_status(DbError = 500)))]
    async fn search_body(
        &self,
        ctx: &Ctx,
        limit: Option<u32>,
        verbose: Option<bool>,
    ) -> Result<SearchEcho, SearchError>;
}

struct SearchBackEnd;

impl SearchClientService<()> for SearchBackEnd {
    async fn search(
        &self,
        _ctx: &(),
        limit: Option<u32>,
        verbose: Option<bool>,
    ) -> Result<SearchEcho, SearchError> {
        ready(()).await;
        Ok(SearchEcho { limit, verbose })
    }

    async fn search_body(
        &self,
        _ctx: &(),
        limit: Option<u32>,
        verbose: Option<bool>,
    ) -> Result<SearchEcho, SearchError> {
        ready(()).await;
        Ok(SearchEcho { limit, verbose })
    }
}

// -------------------------------------------------------------------------------------------
// A bodyless `GET` whose macro-generated message's two fields are both bound by the path,
// leaving nothing to read off the query string.
// -------------------------------------------------------------------------------------------

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LabelStatus {
    pub label: String,
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum LabelError {
    NotFound,
}

#[service_schema(transports = ["http_rest"])]
pub trait LabelClientService<Ctx> {
    #[service_schema_op(http(
        method = "GET",
        path = "/orgs/{org_id}/labels/{label_id}",
        error_status(NotFound = 404),
    ))]
    async fn get_label(
        &self,
        ctx: &Ctx,
        org_id: String,
        label_id: String,
    ) -> Result<LabelStatus, LabelError>;
}

struct LabelBackEnd;

impl LabelClientService<()> for LabelBackEnd {
    async fn get_label(
        &self,
        _ctx: &(),
        org_id: String,
        label_id: String,
    ) -> Result<LabelStatus, LabelError> {
        ready(()).await;
        if label_id == "missing" {
            return Err(LabelError::NotFound);
        }
        Ok(LabelStatus {
            label: format!("{org_id}/{label_id}"),
        })
    }
}

// -------------------------------------------------------------------------------------------
// The three body kinds: `bytes`, `stream` and `multipart`, dispatched through the Rust macro
// and through the emitted TypeScript so the same request can be compared byte-for-byte.
// -------------------------------------------------------------------------------------------

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum ThumbnailError {
    NotFound,
}

#[service_schema(transports = ["http_rest"])]
pub trait ThumbnailClientService<Ctx> {
    #[service_schema_op(http(
        method = "GET",
        path = "/thumbnails/{document_id}",
        body = "bytes",
        header_out("x-document-id"),
        error_status(NotFound = 404),
    ))]
    async fn get_thumbnail(
        &self,
        ctx: &Ctx,
        document_id: String,
    ) -> Result<(Vec<u8>, String, String), ThumbnailError>;
}

pub struct ThumbnailBackEnd;

impl ThumbnailClientService<()> for ThumbnailBackEnd {
    async fn get_thumbnail(
        &self,
        _ctx: &(),
        document_id: String,
    ) -> Result<(Vec<u8>, String, String), ThumbnailError> {
        ready(()).await;
        if document_id == "missing" {
            return Err(ThumbnailError::NotFound);
        }
        Ok((
            vec![0x89, 0x50, 0x4e, 0x47],
            "image/png".to_owned(),
            format!("doc-{document_id}"),
        ))
    }
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum ContentError {
    NotFound,
}

#[service_schema(transports = ["http_rest"])]
pub trait ContentClientService<Ctx> {
    #[service_schema_op(http(
        method = "GET",
        path = "/files/{file_id}",
        body = "stream",
        error_status(NotFound = 404),
    ))]
    async fn get_file(
        &self,
        ctx: &Ctx,
        file_id: String,
    ) -> Result<content_client_service_schema::StreamedAnswer, ContentError>;
}

/// A chunked [`Read`] source: every call answers at most [`CHUNKED_READ_CAP`] bytes.
/// [`BodySource`](content_client_service_schema::BodySource) is blanket implemented for every
/// `Read`, so this satisfies the seam for free.
struct ChunkedSlice {
    remaining: Vec<u8>,
}

impl ChunkedSlice {
    fn new(content: &[u8]) -> Self {
        Self {
            remaining: content.to_vec(),
        }
    }
}

impl Read for ChunkedSlice {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let take = buf.len().min(self.remaining.len()).min(CHUNKED_READ_CAP);
        let rest = self.remaining.split_off(take);
        buf[..take].copy_from_slice(&self.remaining);
        self.remaining = rest;
        Ok(take)
    }
}

pub struct ContentBackEnd;

impl ContentClientService<()> for ContentBackEnd {
    async fn get_file(
        &self,
        _ctx: &(),
        file_id: String,
    ) -> Result<content_client_service_schema::StreamedAnswer, ContentError> {
        ready(()).await;
        if file_id == "missing" {
            return Err(ContentError::NotFound);
        }
        Ok(content_client_service_schema::StreamedAnswer::Full(
            Box::new(ChunkedSlice::new(STREAMED_CONTENT)),
        ))
    }
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct UploadDocumentResponse {
    pub document_id: String,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum UploadDocumentError {
    TooLarge,
}

#[service_schema(transports = ["http_rest"])]
pub trait UploadDocumentClientService<Ctx> {
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
        attachment: Box<dyn upload_document_client_service_schema::BodySource + Send>,
    ) -> Result<UploadDocumentResponse, UploadDocumentError>;
}

pub struct UploadDocumentBackEnd;

impl UploadDocumentClientService<()> for UploadDocumentBackEnd {
    async fn upload_document(
        &self,
        _ctx: &(),
        folder_id: String,
        title: String,
        description: Option<String>,
        mut attachment: Box<dyn upload_document_client_service_schema::BodySource + Send>,
    ) -> Result<UploadDocumentResponse, UploadDocumentError> {
        ready(()).await;
        let mut drained = Vec::new();
        let mut buf = [0_u8; 64];
        loop {
            let read = attachment.pull(&mut buf).unwrap();
            if read == 0 {
                break;
            }
            drained.extend_from_slice(&buf[..read]);
        }
        if title == "toolarge" {
            return Err(UploadDocumentError::TooLarge);
        }
        // The Node driver's own implementation never reads the part's content either, so it
        // plays no part in the success value the two dispatchers are compared on.
        assert!(
            !drained.is_empty(),
            "the file part must have drained something"
        );
        Ok(UploadDocumentResponse {
            document_id: format!("doc-{folder_id}-{title}-{}", description.is_some()),
        })
    }
}

/// A required `header_in` binding, so the TypeScript implementation reaches for the argument
/// `create{Service}Dispatcher` now decodes and hands it — the Rust twin the Node driver beside
/// this one is measured against.
///
/// Carries a path placeholder purely so the message is the wire scalar it binds (mirroring
/// `purge_document`'s own shape): a bodyless `GET`/`DELETE` operation with no placeholder at all
/// is a separate, pre-existing gap between the two message-assembly rules, filed on its own and
/// left for that task rather than this one.
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EchoRangeResponse {
    pub received: String,
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum EchoRangeError {
    NotFound,
}

#[service_schema(transports = ["http_rest"])]
pub trait EchoClientService<Ctx> {
    #[service_schema_op(http(
        method = "GET",
        path = "/echo/{document_id}",
        header_in("range" = byte_range),
        error_status(NotFound = 404),
    ))]
    async fn echo_range(
        &self,
        ctx: &Ctx,
        document_id: String,
        byte_range: String,
    ) -> Result<EchoRangeResponse, EchoRangeError>;
}

pub struct EchoBackEnd;

impl EchoClientService<()> for EchoBackEnd {
    async fn echo_range(
        &self,
        _ctx: &(),
        document_id: String,
        byte_range: String,
    ) -> Result<EchoRangeResponse, EchoRangeError> {
        ready(()).await;
        if document_id == "missing" {
            return Err(EchoRangeError::NotFound);
        }
        Ok(EchoRangeResponse {
            received: byte_range,
        })
    }
}

/// A bodyless `GET` carrying no field beside the context at all — the shape the TypeScript
/// dispatcher used to assemble as `null`, which the generated `z.strictObject({})` refuses. The
/// Rust twin the group beside this one is measured against.
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PulseResponse {
    pub alive: bool,
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum PulseError {
    Unavailable,
}

#[service_schema(transports = ["http_rest"])]
pub trait PulseClientService<Ctx> {
    #[service_schema_op(http(method = "GET", path = "/pulse"))]
    async fn pulse(&self, ctx: &Ctx) -> Result<PulseResponse, PulseError>;
}

pub struct PulseBackEnd;

impl PulseClientService<()> for PulseBackEnd {
    async fn pulse(&self, _ctx: &()) -> Result<PulseResponse, PulseError> {
        ready(()).await;
        Ok(PulseResponse { alive: true })
    }
}

/// A reply operation whose success is `()`.
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WatchRequest {
    pub topic: String,
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum WatchError {
    Unavailable,
}

#[service_schema(transports = [])]
pub trait WatchClientService<Ctx> {
    async fn watch(&self, ctx: &Ctx, req: WatchRequest) -> Result<(), WatchError>;
}

pub struct WatchBackEnd;

impl WatchClientService<()> for WatchBackEnd {
    async fn watch(&self, _ctx: &(), _req: WatchRequest) -> Result<(), WatchError> {
        ready(()).await;
        Ok(())
    }
}

/// Every declared type is constructible — the groups beside this one read only emitted text.
#[test]
fn every_declared_type_is_constructible() {
    let asked = WindowRequest {
        conversation_id: "652f1a3b4c5d6e7f8a9b0c1d".to_owned(),
        limit: Some(10),
    };
    assert_eq!(asked.limit, Some(10));
    assert_eq!(
        WindowPage {
            items: vec!["one".to_owned()]
        }
        .items
        .len(),
        1
    );
    assert_eq!(WindowError::NotFound, WindowError::NotFound);
    assert_eq!(
        ConversationId("652f1a3b4c5d6e7f8a9b0c1d".to_owned())
            .0
            .len(),
        24
    );
    drop(ConversationBackEnd.window(&(), asked));
    drop(
        ConversationBackEnd
            .purge_conversation(&(), ConversationId("652f1a3b4c5d6e7f8a9b0c1d".to_owned())),
    );
}

#[test]
fn the_watch_backend_answers_ok_unit() {
    let answered = poll_once(WatchBackEnd.watch(
        &(),
        WatchRequest {
            topic: "x".to_owned(),
        },
    ))
    .unwrap();
    assert_eq!(answered, Ok(()));
}

/// The transports never suspend, so one poll answers them.
fn poll_once<Answering>(answering: Answering) -> Option<Answering::Output>
where
    Answering: Future,
{
    let mut pinned = pin!(answering);
    let mut polling = PollContext::from_waker(Waker::noop());
    match pinned.as_mut().poll(&mut polling) {
        Poll::Ready(answer) => Some(answer),
        Poll::Pending => None,
    }
}

/// The driver beside this one exercises the same behavior again through the emitted TypeScript.
#[test]
fn every_reader_form_and_query_backend_answers_as_declared() {
    assert_eq!(
        poll_once(GateBackEnd.check_gate(&(), "missing".to_owned())).unwrap(),
        Err(GateError::NotFound)
    );
    assert_eq!(
        poll_once(GateBackEnd.check_gate(&(), "locked".to_owned())).unwrap(),
        Err(GateError::Locked)
    );
    assert_eq!(
        poll_once(GateBackEnd.check_gate(&(), "g1".to_owned())).unwrap(),
        Ok(GateStatus { open: true })
    );
    assert_eq!(
        poll_once(GateBackEnd.check_gate_unmapped(&(), "g1".to_owned())).unwrap(),
        Err(GateError::NotFound)
    );

    assert_eq!(
        poll_once(VaultBackEnd.check_vault(&(), "missing".to_owned())).unwrap(),
        Err(VaultError::NotFound)
    );
    assert_eq!(
        poll_once(VaultBackEnd.check_vault(&(), "locked".to_owned())).unwrap(),
        Err(VaultError::Locked)
    );
    assert_eq!(
        poll_once(VaultBackEnd.check_vault(&(), "v1".to_owned())).unwrap(),
        Ok(VaultStatus { open: true })
    );
    assert_eq!(
        poll_once(VaultBackEnd.check_vault_unmapped(&(), "v1".to_owned())).unwrap(),
        Err(VaultError::NotFound)
    );

    assert_eq!(
        poll_once(ArchiveBackEnd.check_archive(&(), "missing".to_owned())).unwrap(),
        Err(ArchiveError::NotFound)
    );
    assert_eq!(
        poll_once(ArchiveBackEnd.check_archive(&(), "locked".to_owned())).unwrap(),
        Err(ArchiveError::Locked { retry_after: 30 })
    );
    assert_eq!(
        poll_once(ArchiveBackEnd.check_archive(&(), "a1".to_owned())).unwrap(),
        Ok(ArchiveStatus { archived: true })
    );

    assert_eq!(
        poll_once(SealBackEnd.check_seal(&(), "missing".to_owned())).unwrap(),
        Err(SealError::NotFound)
    );
    assert_eq!(
        poll_once(SealBackEnd.check_seal(&(), "locked".to_owned())).unwrap(),
        Err(SealError::Locked)
    );
    assert_eq!(
        poll_once(SealBackEnd.check_seal(&(), "s1".to_owned())).unwrap(),
        Ok(SealStatus { sealed: true })
    );
    assert_eq!(
        poll_once(SealBackEnd.check_seal_unmapped(&(), "s1".to_owned())).unwrap(),
        Err(SealError::NotFound)
    );

    assert_eq!(
        poll_once(SearchBackEnd.search(&(), Some(10), Some(true))).unwrap(),
        Ok(SearchEcho {
            limit: Some(10),
            verbose: Some(true)
        })
    );
    assert_eq!(
        poll_once(SearchBackEnd.search_body(&(), None, None)).unwrap(),
        Ok(SearchEcho {
            limit: None,
            verbose: None
        })
    );

    assert_eq!(
        poll_once(LabelBackEnd.get_label(&(), "acme".to_owned(), "missing".to_owned())).unwrap(),
        Err(LabelError::NotFound)
    );
    assert_eq!(
        poll_once(LabelBackEnd.get_label(&(), "acme".to_owned(), "priority".to_owned())).unwrap(),
        Ok(LabelStatus {
            label: "acme/priority".to_owned()
        })
    );
}

/// Read only by the group beside this one.
#[test]
fn the_echo_backend_answers_the_header_it_was_bound() {
    assert_eq!(
        poll_once(EchoBackEnd.echo_range(&(), "d1".to_owned(), "bytes=0-9".to_owned())).unwrap(),
        Ok(EchoRangeResponse {
            received: "bytes=0-9".to_owned()
        })
    );
}

/// Read only by the group beside this one.
#[test]
fn the_pulse_backend_answers_alive() {
    assert_eq!(
        poll_once(PulseBackEnd.pulse(&())).unwrap(),
        Ok(PulseResponse { alive: true })
    );
}
