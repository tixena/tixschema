//! What `mcp_scopes` answers, read once from a declaration for the servers that write the same
//! answer in TypeScript, Dart and Kotlin.

use crate::service_schema::parse::{
    McpCompletesKind, McpResourceUri, OperationDef, PathSegment, ServiceDef, written,
};

/// How a resource read names its URI.
pub enum ReadUri {
    Fixed(String),
    Template(Vec<(String, bool)>),
}

/// The scopes each request a server routes needs, in the order `mcp_scopes` tests them.
pub struct ScopeTable {
    pub completions: Vec<(String, Vec<String>)>,
    pub listing: Option<Vec<String>>,
    pub prompts: Vec<(String, Vec<String>)>,
    pub reads: Vec<(ReadUri, Vec<String>)>,
    pub tools: Vec<(String, Vec<String>)>,
}

impl ScopeTable {
    pub fn has_templates(&self) -> bool {
        self.reads
            .iter()
            .any(|(uri, _)| matches!(uri, ReadUri::Template(_)))
    }

    pub fn of(service: &ServiceDef) -> Self {
        let resourced = || {
            service
                .operations
                .iter()
                .filter(|operation| !operation.is_mcp_tool() && !operation.is_mcp_prompt())
        };
        let is_template = |operation: &OperationDef| {
            operation
                .mcp
                .resource
                .as_ref()
                .is_some_and(McpResourceUri::is_template)
        };
        let tools = service
            .operations
            .iter()
            .filter(|operation| operation.is_mcp_tool() && !operation.mcp.scopes.is_empty())
            .map(|operation| (operation.mcp_tool_name().to_owned(), scopes_of(operation)))
            .collect();
        let prompts = service
            .operations
            .iter()
            .filter(|operation| operation.is_mcp_prompt() && !operation.mcp.scopes.is_empty())
            .map(|operation| (operation.mcp_tool_name().to_owned(), scopes_of(operation)))
            .collect();
        let fixed = resourced()
            .filter(|operation| operation.mcp.resource.is_some() && !is_template(operation));
        let templates = resourced().filter(|operation| is_template(operation));
        let reads = fixed
            .chain(templates)
            .filter_map(|operation| {
                let resource = operation.mcp.resource.as_ref()?;
                let uri = if resource.is_template() {
                    ReadUri::Template(
                        resource
                            .segments
                            .iter()
                            .map(|segment| match segment {
                                PathSegment::Literal(text) => (text.clone(), false),
                                PathSegment::Placeholder(name) => (name.clone(), true),
                            })
                            .collect(),
                    )
                } else {
                    ReadUri::Fixed(resource.uri.clone())
                };
                Some((uri, scopes_of(operation)))
            })
            .collect();
        let listing = resourced()
            .find(|operation| operation.mcp.list_resources && !operation.mcp.scopes.is_empty())
            .map(scopes_of);
        let completions = resourced()
            .filter(|operation| !operation.mcp.scopes.is_empty())
            .filter_map(|operation| {
                let completes = operation.mcp.completes.as_ref()?;
                let target = service
                    .operations
                    .iter()
                    .find(|candidate| written(&candidate.ident) == completes.operation)?;
                let uri = match completes.kind {
                    McpCompletesKind::Resource => target.mcp.resource.as_ref()?.uri.clone(),
                    McpCompletesKind::Prompt => {
                        target.mcp.prompt.as_ref()?;
                        target.mcp_tool_name().to_owned()
                    }
                };
                Some((uri, scopes_of(operation)))
            })
            .collect();
        Self {
            completions,
            listing,
            prompts,
            reads,
            tools,
        }
    }
}

fn scopes_of(operation: &OperationDef) -> Vec<String> {
    operation
        .mcp
        .scopes
        .iter()
        .map(|(scope, _)| scope.clone())
        .collect()
}
