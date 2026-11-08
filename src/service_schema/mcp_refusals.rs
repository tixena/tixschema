//! What a service that asks for `mcp` is refused at expansion, each refusal naming the operation.

use super::parse::{
    McpCompletesKind, OperationDef, OperationInputs, OperationOutcome, ServiceDef, option_inner,
    wire_key, written,
};
use quote::ToTokens as _;
use syn::{Ident, Type};

const TOOL_NAME_LIMIT: usize = 128;
const WRAPPED_KEY: &str = "value";

/// Every refusal of `service`, joined, or `None` where it holds none.
pub fn refusals(service: &ServiceDef) -> Option<syn::Error> {
    let mut refused: Vec<syn::Error> = Vec::new();
    for (position, operation) in service.operations.iter().enumerate() {
        refused.extend(operation_refusals(operation));
        refused.extend(resource_refusals(service, position, operation));
        refused.extend(prompt_refusals(operation));
        let kind = if operation.is_mcp_tool() {
            "tool"
        } else if operation.is_mcp_prompt() {
            "prompt"
        } else {
            continue;
        };
        let name = operation.mcp_tool_name();
        if let Some(first) = service.operations[..position].iter().find(|earlier| {
            earlier.is_mcp_tool() == operation.is_mcp_tool()
                && earlier.is_mcp_prompt() == operation.is_mcp_prompt()
                && earlier.mcp_tool_name() == name
        }) {
            refused.push(syn::Error::new(
                operation.ident.span(),
                format!(
                    "service_schema: operations `{}` and `{}` are both the {kind} `{name}`, and this \
                     service asks for `mcp`\n       \
                     a server lists each {kind} name once; write `mcp(name = \"...\")` on one of them",
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
    let kind = if operation.is_mcp_tool() {
        "tool"
    } else if operation.is_mcp_prompt() {
        "prompt"
    } else {
        return refused;
    };
    let name = operation.mcp_tool_name();
    if name.chars().count() > TOOL_NAME_LIMIT || !name.chars().all(is_tool_name_character) {
        refused.push(syn::Error::new(
            ident.span(),
            format!(
                "service_schema: operation `{ident}` is the {kind} `{name}`, and this service asks for \
                 `mcp`\n       \
                 a {kind} name is at most {TOOL_NAME_LIMIT} characters of `A-Z`, `a-z`, `0-9`, `_`, \
                 `-` and `.`; write `mcp(name = \"...\")` with one"
            ),
        ));
    }
    if operation.is_mcp_prompt() {
        refused.extend(scope_refusals(operation));
        return refused;
    }
    refused.extend(header_refusals(operation));
    refused.extend(scope_refusals(operation));
    refused.extend(interview_refusals(operation));
    refused.extend(content_refusals(operation));
    refused
}

fn interview_refusals(operation: &OperationDef) -> Vec<syn::Error> {
    let ident = &operation.ident;
    let Some(interview) = &operation.mcp.interview else {
        return Vec::new();
    };
    let mut refused = Vec::new();
    if matches!(operation.outcome, OperationOutcome::OneWay) {
        refused.push(refusal(
            ident,
            "is one way and writes `interview(...)` in `mcp(...)`",
            "an interview is an arm of the declared error, and a one way operation declares none",
        ));
    }
    if operation
        .http
        .as_ref()
        .is_some_and(|binding| !binding.error_header_out.is_empty())
    {
        refused.push(refusal(
            ident,
            "writes `interview(...)` in `mcp(...)` and binds `error_header_out`",
            "the arm that asks is the whole declared error, so no header can ride beside it",
        ));
    }
    if let Some(answers) = &interview.answers {
        let held = match &operation.inputs {
            OperationInputs::Generated(fields) => fields
                .iter()
                .find(|(field, _)| written(field) == written(answers)),
            OperationInputs::Empty | OperationInputs::Named(_) => None,
        };
        match held {
            None => refused.push(refusal(
                ident,
                &format!(
                    "writes `interview(answers = {})` in `mcp(...)`, and `{}` is no argument of it",
                    written(answers),
                    written(answers)
                ),
                "name one of the operation's own arguments, which is not the message",
            )),
            Some((_, ty)) if option_inner(ty).is_none() => refused.push(refusal(
                ident,
                &format!(
                    "asks with `{}` and its type `{}` is no `Option`",
                    written(answers),
                    type_text(ty)
                ),
                "the first call has no answer yet, so the argument is an `Option` of the form's type",
            )),
            Some(_) => {}
        }
    }
    refused
}

fn content_refusals(operation: &OperationDef) -> Vec<syn::Error> {
    let ident = &operation.ident;
    if operation.mcp.content.is_none() {
        return Vec::new();
    }
    let mut refused = Vec::new();
    match &operation.outcome {
        OperationOutcome::OneWay => refused.push(refusal(
            ident,
            "is one way and writes `content` in `mcp(...)`",
            "a content tool answers blocks, and a one way operation answers none",
        )),
        OperationOutcome::Reply {
            success,
            error: _error,
        } => {
            let text = type_text(success);
            let blocks = text
                .strip_prefix("Vec<")
                .and_then(|rest| rest.strip_suffix('>'))
                .is_some_and(|inner| inner.rsplit("::").next() == Some("McpContent"));
            if text != "(Vec<u8>,String)" && !blocks {
                refused.push(refusal(
                    ident,
                    "writes `content` in `mcp(...)` and answers a success it cannot write as blocks",
                    "answer a `(Vec<u8>, String)` of bytes and their content type, or a \
                     `Vec<McpContent>` of the type the service module declares",
                ));
            }
        }
    }
    if operation
        .http
        .as_ref()
        .is_some_and(|binding| !binding.header_out.is_empty())
    {
        refused.push(refusal(
            ident,
            "writes `content` in `mcp(...)` and binds `header_out`",
            "a content tool answers blocks, and no header rides beside them",
        ));
    }
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

fn type_text(ty: &Type) -> String {
    ty.to_token_stream().to_string().replace(' ', "")
}

fn refusal(operation: &Ident, what: &str, advice: &str) -> syn::Error {
    syn::Error::new(
        operation.span(),
        format!(
            "service_schema: operation `{operation}` {what}, and this service asks for \
             `mcp`\n       {advice}"
        ),
    )
}

/// What a resource, a resource listing or a completion operation is refused for.
fn resource_refusals(
    service: &ServiceDef,
    position: usize,
    operation: &OperationDef,
) -> Vec<syn::Error> {
    let ident = &operation.ident;
    let mcp = &operation.mcp;
    let mut refused = Vec::new();
    if !operation.is_mcp_tool()
        && !operation.is_mcp_prompt()
        && (mcp.content.is_some() || mcp.interview.is_some())
    {
        refused.push(refusal(
            ident,
            "is a resource, a listing or a completion and writes `content` or `interview(...)`",
            "only a tool answers blocks or asks the user",
        ));
    }
    let kinds = usize::from(mcp.resource.is_some())
        + usize::from(mcp.list_resources)
        + usize::from(mcp.prompt.is_some())
        + usize::from(mcp.completes.is_some());
    if kinds > 1 {
        refused.push(refusal(
            ident,
            "is more than one of `resource`, `list_resources`, `prompt` and `completes` in `mcp(...)`",
            "an operation is one of them; split it into one operation each",
        ));
        return refused;
    }
    let success = match &operation.outcome {
        OperationOutcome::Reply {
            success,
            error: _error,
        } => Some(type_text(success)),
        OperationOutcome::OneWay => None,
    };
    if let Some(resource) = &mcp.resource {
        if !matches!(success.as_deref(), Some("String" | "(Vec<u8>,String)")) {
            refused.push(refusal(
                ident,
                &format!("is the resource `{}` and answers a success a resource cannot", resource.uri),
                "answer a `String` for text, or a `(Vec<u8>, String)` of bytes and their content type",
            ));
        }
        if let Some(first) = service.operations[..position].iter().find(|earlier| {
            earlier
                .mcp
                .resource
                .as_ref()
                .is_some_and(|held| held.shape() == resource.shape())
        }) {
            refused.push(refusal(
                ident,
                &format!(
                    "is the resource `{}`, which `{}` already is",
                    resource.uri, first.ident
                ),
                "a server reads each URI through one operation; write another URI on one of them",
            ));
        }
    }
    if mcp.list_resources {
        refused.extend(listing_refusals(
            service,
            position,
            operation,
            success.as_deref(),
        ));
    }
    if let Some(completes) = &mcp.completes {
        refused.extend(completes_refusals(
            service,
            operation,
            completes,
            success.as_deref(),
        ));
    }
    refused
}

/// What an operation that lists resources is refused for.
fn listing_refusals(
    service: &ServiceDef,
    position: usize,
    operation: &OperationDef,
    success: Option<&str>,
) -> Vec<syn::Error> {
    let ident = &operation.ident;
    let mut refused = Vec::new();
    if let Some(first) = service.operations[..position]
        .iter()
        .find(|earlier| earlier.mcp.list_resources)
    {
        refused.push(refusal(
            ident,
            &format!("lists resources, which `{}` already does", first.ident),
            "a service lists its resources through one operation",
        ));
    }
    let page = success.is_some_and(|text| text.rsplit("::").next() == Some("McpResourcePage"));
    if !page {
        refused.push(refusal(
            ident,
            "lists resources and answers a success a listing cannot",
            "answer the `McpResourcePage` the service module declares",
        ));
    }
    let cursor = match &operation.inputs {
        OperationInputs::Empty => true,
        OperationInputs::Named(declared) => {
            option_inner(declared).is_some_and(|inner| type_text(inner) == "String")
        }
        OperationInputs::Generated(_) => false,
    };
    if !cursor {
        refused.push(refusal(
            ident,
            "lists resources and takes more than a cursor",
            "take no argument, or one `cursor: Option<String>`",
        ));
    }
    refused
}

fn completes_refusals(
    service: &ServiceDef,
    operation: &OperationDef,
    completes: &super::parse::McpCompletes,
    success: Option<&str>,
) -> Vec<syn::Error> {
    let ident = &operation.ident;
    let mut refused = Vec::new();
    let target = service
        .operations
        .iter()
        .find(|candidate| written(&candidate.ident) == completes.operation);
    match completes.kind {
        McpCompletesKind::Resource => {
            match target.and_then(|candidate| candidate.mcp.resource.as_ref()) {
                None => refused.push(refusal(
                    ident,
                    &format!(
                        "completes `{}`, which is no resource template of the service",
                        completes.operation
                    ),
                    "name an operation of this service that writes `mcp(resource = \"...{placeholder}...\")`",
                )),
                Some(resource)
                    if !resource
                        .placeholder_names()
                        .contains(&completes.parameter.as_str()) =>
                {
                    refused.push(refusal(
                        ident,
                        &format!(
                            "completes `{}` for `{}`, and the URI `{}` holds no such placeholder",
                            completes.operation, completes.parameter, resource.uri
                        ),
                        "name a placeholder of the template",
                    ));
                }
                Some(_) => {}
            }
        }
        McpCompletesKind::Prompt => {
            match target.and_then(|candidate| candidate.mcp.prompt.as_ref()) {
                None => refused.push(refusal(
                    ident,
                    &format!(
                        "completes `{}`, which is no prompt of the service",
                        completes.operation
                    ),
                    "name an operation of this service that writes `mcp(prompt)`",
                )),
                Some(prompt)
                    if !prompt
                        .arguments
                        .iter()
                        .any(|(argument, _)| written(argument) == completes.parameter) =>
                {
                    refused.push(refusal(
                        ident,
                        &format!(
                            "completes `{}` for `{}`, and the prompt takes no such argument",
                            completes.operation, completes.parameter
                        ),
                        "name an argument of the prompt, as the operation spells it",
                    ));
                }
                Some(_) => {}
            }
        }
    }
    if success != Some("Vec<String>") {
        refused.push(refusal(
            ident,
            "completes a parameter and answers a success a completion cannot",
            "answer a `Vec<String>` of suggestions",
        ));
    }
    let typed_and_chosen =
        matches!(&operation.inputs, OperationInputs::Generated(fields) if fields.len() == 2);
    if !typed_and_chosen {
        refused.push(refusal(
            ident,
            "completes a parameter and does not take two arguments",
            "take the text typed so far, then the values already chosen: \
             `typed: String, chosen: HashMap<String, String>`",
        ));
    }
    refused
}

/// What a prompt is refused for beyond its name: its arguments, its success, a one-way outcome, a
/// header binding and any key of `mcp(...)` a prompt cannot carry.
fn prompt_refusals(operation: &OperationDef) -> Vec<syn::Error> {
    let ident = &operation.ident;
    let Some(prompt) = &operation.mcp.prompt else {
        return Vec::new();
    };
    let mut refused = Vec::new();
    for (argument, ty) in &prompt.arguments {
        let text = type_text(ty);
        let strings =
            text == "String" || option_inner(ty).is_some_and(|inner| type_text(inner) == "String");
        if !strings {
            refused.push(refusal(
                ident,
                &format!("is a prompt and takes `{}: {text}`", written(argument)),
                "MCP sends a prompt's arguments as strings only; take a `String`, or an \
                 `Option<String>` for one that may be left out",
            ));
        }
    }
    match &operation.outcome {
        OperationOutcome::OneWay => refused.push(refusal(
            ident,
            "is one way and writes `prompt` in `mcp(...)`",
            "a prompt answers messages, and a one way operation answers none",
        )),
        OperationOutcome::Reply {
            success,
            error: _error,
        } => {
            let text = type_text(success);
            let messages = text
                .strip_prefix("Vec<")
                .and_then(|rest| rest.strip_suffix('>'))
                .is_some_and(|inner| inner.rsplit("::").next() == Some("McpPromptMessage"));
            if text != "String" && !messages {
                refused.push(refusal(
                    ident,
                    "is a prompt and answers a success a prompt cannot write as messages",
                    "answer a `String` for one user message, or a `Vec<McpPromptMessage>` of the \
                     type the service module declares",
                ));
            }
        }
    }
    if operation.http.as_ref().is_some_and(|binding| {
        !binding.header_in.is_empty()
            || !binding.header_out.is_empty()
            || !binding.error_header_out.is_empty()
    }) {
        refused.push(refusal(
            ident,
            "is a prompt and binds a header",
            "a prompt's arguments and messages cross as they are; bind no `header_in`, \
             `header_out` or `error_header_out`",
        ));
    }
    let mcp = &operation.mcp;
    let beside = [
        ("read_only", mcp.read_only),
        ("idempotent", mcp.idempotent),
        ("destructive", mcp.destructive.is_some()),
        ("open_world", mcp.open_world.is_some()),
        ("header", !mcp.header.is_empty()),
        ("mime_type", mcp.mime_type.is_some()),
        ("ttl_ms", mcp.ttl_ms.is_some()),
        ("cache_scope", mcp.cache_scope.is_some()),
        ("content", mcp.content.is_some()),
        ("interview", mcp.interview.is_some()),
    ];
    for (key, written_here) in beside {
        if written_here {
            refused.push(refusal(
                ident,
                &format!("is a prompt and writes `{key}` in `mcp(...)`"),
                "a prompt takes `title`, `name`, `icon` and `scopes` beside `prompt`",
            ));
        }
    }
    refused
}
