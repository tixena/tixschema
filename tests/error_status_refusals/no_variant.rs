//! A name in `error_status` that is no variant of the fault stops the build.

use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum DocumentError {
    NotFound,
    VersionGone { version: u32 },
}

#[service_schema(transports = ["http_rest"])]
pub trait Documents<Ctx> {
    #[service_schema_op(http(
        method = "GET",
        path = "/documents/{id}",
        error_status(NotFound = 404, VersionGone = 410, Missing = 500),
    ))]
    async fn fetch(&self, ctx: &Ctx, id: String) -> Result<u32, DocumentError>;
}

fn main() {}
