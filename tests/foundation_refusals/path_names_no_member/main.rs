//! A placeholder written as the key serde reads a field from, one that names a member of a
//! flattened type, and one that names a member serde never reads, each on a declared message.

use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

/// A message under a `rename_all`.
#[model_schema()]
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Renamed {
    pub document_id: String,
    pub title: String,
}

/// What `Holder` flattens.
#[model_schema()]
#[derive(Deserialize, Serialize)]
pub struct Titled {
    pub title: String,
}

/// A message with a flattened member.
#[model_schema()]
#[derive(Deserialize, Serialize)]
pub struct Holder {
    pub document_id: String,
    #[serde(flatten)]
    pub titled: Titled,
}

/// A message with a member serde never reads.
#[model_schema()]
#[derive(Deserialize, Serialize)]
pub struct Cached {
    #[serde(skip)]
    pub cached_at: String,
    pub document_id: String,
}

/// What each operation answers.
#[model_schema()]
#[derive(Deserialize, Serialize)]
pub struct Answer {
    pub id: String,
}

/// The error each operation declares.
#[model_schema()]
#[derive(Deserialize, Serialize)]
#[serde(tag = "errorCode")]
pub enum Refused {
    Nope,
}

/// One operation for each placeholder that names no member.
#[service_schema(transports = ["http_rest"])]
pub trait Bound<Ctx> {
    #[service_schema_op(http(method = "PUT", path = "/wire/{documentId}"))]
    async fn by_wire_name(&self, ctx: &Ctx, req: Renamed) -> Result<Answer, Refused>;

    #[service_schema_op(http(method = "PUT", path = "/flattened/{title}"))]
    async fn flattened_member(&self, ctx: &Ctx, req: Holder) -> Result<Answer, Refused>;

    #[service_schema_op(http(method = "PUT", path = "/skipped/{cached_at}"))]
    async fn skipped_member(&self, ctx: &Ctx, req: Cached) -> Result<Answer, Refused>;
}

fn main() {}
