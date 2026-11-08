//! `CounselService`: operations that are prompts, a completion of a prompt's argument and a tool,
//! and the stand-in that answers them.

use alloc::sync::Arc;
use core::future::{Future, ready};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use tixschema::{model_schema, service_schema};

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum CounselError {
    NotFound,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Counsel {
    pub version: u32,
}

/// Guides the model through the setup of an organization.
#[service_schema(transports = ["mcp"])]
pub trait CounselService<Ctx> {
    /// Guides the model through the configuration of one organization.
    #[service_schema_op(mcp(prompt))]
    async fn configure_organization(&self, ctx: &Ctx) -> Result<String, CounselError>;

    /// Answers the current version of the counsel.
    async fn get_counsel(&self, ctx: &Ctx) -> Result<Counsel, CounselError>;

    /// Greets one person.
    #[service_schema_op(mcp(prompt, title = "A Greeting", icon = "https://example.com/hi.png", scopes = ["prompts:read"]))]
    async fn greet(&self, ctx: &Ctx, name: String) -> Result<String, CounselError>;

    /// Reviews one document type against the naming guide.
    #[service_schema_op(mcp(prompt))]
    async fn review_document_type(
        &self,
        ctx: &Ctx,
        document_type: String,
        focus: Option<String>,
    ) -> Result<Vec<counsel_service_schema::McpPromptMessage>, CounselError>;

    /// Suggests the document types that start with what the user typed.
    #[service_schema_op(mcp(completes(prompt = "review_document_type", parameter = "document_type"), scopes = ["prompts:read"]))]
    async fn suggest_document_types(
        &self,
        ctx: &Ctx,
        typed: String,
        chosen: HashMap<String, String>,
    ) -> Result<Vec<String>, CounselError>;
}

/// Answers every operation, and writes down what reached it.
#[derive(Clone, Default)]
pub struct Counsels {
    pub reached: Arc<Mutex<Vec<String>>>,
}

impl Counsels {
    fn reach(&self, call: String) {
        self.reached.lock().unwrap().push(call);
    }
}

// Every method below is synchronous work wrapped in an already-ready `Future`: nothing here waits
// on anything.
impl CounselService<()> for Counsels {
    fn configure_organization(
        &self,
        _ctx: &(),
    ) -> impl Future<Output = Result<String, CounselError>> {
        self.reach("configure-organization".to_owned());
        ready(Ok("Apply the entity types in this order.".to_owned()))
    }

    fn get_counsel(&self, _ctx: &()) -> impl Future<Output = Result<Counsel, CounselError>> {
        self.reach("get-counsel".to_owned());
        ready(Ok(Counsel { version: 3 }))
    }

    fn greet(&self, _ctx: &(), name: String) -> impl Future<Output = Result<String, CounselError>> {
        self.reach(format!("greet {name}"));
        assert!(name != "boom", "the greeter came apart");
        ready(if name == "missing" {
            Err(CounselError::NotFound)
        } else {
            Ok(format!("Say hello to {name}."))
        })
    }

    fn review_document_type(
        &self,
        _ctx: &(),
        document_type: String,
        focus: Option<String>,
    ) -> impl Future<Output = Result<Vec<counsel_service_schema::McpPromptMessage>, CounselError>>
    {
        use counsel_service_schema::{McpContent, McpPromptMessage, McpResourceContents, McpRole};
        self.reach(format!("review-document-type {document_type} {focus:?}"));
        ready(if document_type == "missing" {
            Err(CounselError::NotFound)
        } else {
            Ok(vec![
                McpPromptMessage {
                    role: McpRole::User,
                    content: McpContent::Text {
                        text: format!(
                            "Review the document type {document_type} against this guide."
                        ),
                    },
                },
                McpPromptMessage {
                    role: McpRole::User,
                    content: McpContent::Resource {
                        uri: "guide://naming".to_owned(),
                        mime_type: Some("text/markdown".to_owned()),
                        contents: McpResourceContents::Text("# Naming".to_owned()),
                    },
                },
                McpPromptMessage {
                    role: McpRole::Assistant,
                    content: McpContent::Image {
                        data: vec![0x89, b'P', b'N', b'G'],
                        mime_type: "image/png".to_owned(),
                    },
                },
            ])
        })
    }

    fn suggest_document_types(
        &self,
        _ctx: &(),
        typed: String,
        chosen: HashMap<String, String>,
    ) -> impl Future<Output = Result<Vec<String>, CounselError>> {
        self.reach(format!("suggest-document-types {typed} {}", chosen.len()));
        ready(Ok(["ID card", "Invoice", "Passport"]
            .into_iter()
            .filter(|candidate| candidate.to_lowercase().starts_with(&typed.to_lowercase()))
            .map(str::to_owned)
            .collect()))
    }
}
