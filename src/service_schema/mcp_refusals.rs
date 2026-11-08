//! What a service that asks for `mcp` is refused at expansion, each refusal naming the operation.

use super::parse::{OperationDef, OperationInputs, ServiceDef, wire_key, written};
use syn::Ident;

const TOOL_NAME_LIMIT: usize = 128;
const WRAPPED_KEY: &str = "value";

/// Every refusal of `service`, joined, or `None` where it holds none.
pub fn refusals(service: &ServiceDef) -> Option<syn::Error> {
    let mut refused: Vec<syn::Error> = Vec::new();
    for (position, operation) in service.operations.iter().enumerate() {
        refused.extend(operation_refusals(operation));
        let name = operation.mcp_tool_name();
        if let Some(first) = service.operations[..position]
            .iter()
            .find(|earlier| earlier.mcp_tool_name() == name)
        {
            refused.push(syn::Error::new(
                operation.ident.span(),
                format!(
                    "service_schema: operations `{}` and `{}` are both the tool `{name}`, and this \
                     service asks for `mcp`\n       \
                     a server lists each tool name once; write `mcp(name = \"...\")` on one of them",
                    first.ident, operation.ident
                ),
            ));
        }
    }
    refused.into_iter().reduce(|mut collected, refusal| {
        collected.combine(refusal);
        collected
    })
}

fn operation_refusals(operation: &OperationDef) -> Vec<syn::Error> {
    let ident = &operation.ident;
    let mut refused = Vec::new();
    if operation.doc.is_none() {
        refused.push(syn::Error::new(
            ident.span(),
            format!(
                "service_schema: operation `{ident}` has no rustdoc, and this service asks for \
                 `mcp`\n       \
                 a tool's description is its method's rustdoc; write a `///` comment on the method"
            ),
        ));
    }
    let name = operation.mcp_tool_name();
    if name.chars().count() > TOOL_NAME_LIMIT || !name.chars().all(is_tool_name_character) {
        refused.push(syn::Error::new(
            ident.span(),
            format!(
                "service_schema: operation `{ident}` is the tool `{name}`, and this service asks for \
                 `mcp`\n       \
                 a tool name is at most {TOOL_NAME_LIMIT} characters of `A-Z`, `a-z`, `0-9`, `_`, \
                 `-` and `.`; write `mcp(name = \"...\")` with one"
            ),
        ));
    }
    refused.extend(header_refusals(operation));
    refused.extend(scope_refusals(operation));
    refused
}

fn scope_refusals(operation: &OperationDef) -> Vec<syn::Error> {
    let ident = &operation.ident;
    let mut refused = Vec::new();
    for (position, (scope, span)) in operation.mcp.scopes.iter().enumerate() {
        let found = if scope.is_empty() {
            Some("an empty scope".to_owned())
        } else if operation.mcp.scopes[..position]
            .iter()
            .any(|(earlier, _)| earlier == scope)
        {
            Some(format!("the scope \"{scope}\" twice"))
        } else if scope
            .chars()
            .any(|letter| letter == ' ' || letter == '"' || letter == '\\' || letter.is_control())
        {
            Some(format!(
                "the scope \"{}\", which holds a space, a quote, a backslash or a control character",
                scope.escape_debug()
            ))
        } else {
            None
        };
        if let Some(problem) = found {
            refused.push(syn::Error::new(
                *span,
                format!(
                    "service_schema: operation `{ident}` writes {problem} in `mcp(scopes = [...])`\n       \
                     a scope is one token a token's `scope` claim lists, separated by spaces; write \
                     each scope once, with none of those characters"
                ),
            ));
        }
    }
    refused
}

const fn is_tool_name_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
}

fn header_refusals(operation: &OperationDef) -> Vec<syn::Error> {
    let ident = &operation.ident;
    let Some(binding) = operation.http.as_ref() else {
        return Vec::new();
    };
    let mut refused = Vec::new();
    for (list, written_under) in [
        (&binding.header_out, "the success"),
        (&binding.error_header_out, "the error"),
    ] {
        if list.iter().any(|name| name == WRAPPED_KEY) {
            refused.push(value_refusal(ident, "header_out", written_under));
        }
    }
    let message = match &operation.inputs {
        OperationInputs::Named(_) => Some("the message"),
        OperationInputs::Empty | OperationInputs::Generated(_) => None,
    };
    for header in &binding.header_in {
        if let Some(written_under) = message.filter(|_| header.name == WRAPPED_KEY) {
            refused.push(value_refusal(ident, "header_in", written_under));
        }
        if let OperationInputs::Generated(fields) = &operation.inputs
            && let Some((field, _)) = fields
                .iter()
                .find(|(field, _)| wire_key(field) == header.name)
        {
            refused.push(syn::Error::new(
                ident.span(),
                format!(
                    "service_schema: operation `{ident}` binds the `header_in` name \"{}\", and this \
                     service asks for `mcp`\n       \
                     the tool's input already holds the key \"{}\" for the argument `{}`; bind the \
                     header under another name",
                    header.name,
                    header.name,
                    written(field)
                ),
            ));
        }
    }
    refused
}

fn value_refusal(operation: &Ident, binding: &str, written_under: &str) -> syn::Error {
    syn::Error::new(
        operation.span(),
        format!(
            "service_schema: operation `{operation}` binds the name \"{WRAPPED_KEY}\" on `{binding}`, \
             and this service asks for `mcp`\n       \
             {written_under} is written under \"{WRAPPED_KEY}\" there; bind the header under \
             another name"
        ),
    )
}
