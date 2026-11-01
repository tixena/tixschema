//! An `openapi` path an operation already answers with `GET`, at the default path and at a path
//! written, and the default path a placeholder route covers.

use serde::{Deserialize, Serialize};
use tixschema::service_schema;

#[derive(Deserialize, Serialize)]
pub struct CatalogDescription {
    pub entries: u32,
}

#[derive(Deserialize, Serialize)]
pub enum CatalogError {
    DbError,
}

#[service_schema(transports = ["http_rest"], openapi)]
pub trait CatalogService<Ctx> {
    #[service_schema_op(http(method = "GET", path = "/openapi.json"))]
    async fn describe(&self, ctx: &Ctx) -> Result<CatalogDescription, CatalogError>;
}

#[service_schema(transports = ["http_rest"], openapi(path = "/v1/openapi.json"))]
pub trait ArchiveService<Ctx> {
    #[service_schema_op(http(method = "GET", path = "/v1/openapi.json"))]
    async fn index(&self, ctx: &Ctx) -> Result<CatalogDescription, CatalogError>;
}

#[service_schema(transports = ["http_rest"], openapi)]
pub trait DocumentService<Ctx> {
    #[service_schema_op(http(method = "GET", path = "/{document_id}"))]
    async fn get_document(
        &self,
        ctx: &Ctx,
        document_id: String,
    ) -> Result<CatalogDescription, CatalogError>;
}

fn main() {}
