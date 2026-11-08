//! `<Service>Schema::mcp_tools()`, `wrapped_{operation}()` and `MCP_NAMES`: the tool list of an
//! `mcp` service, each tool built from its operation's message, success, headers and rustdoc.

use super::parse::{
    BodyKind, HttpShape, McpCompletesKind, McpResourceUri, OperationDef, OperationInputs,
    OperationOutcome, PathSegment, ServiceDef, is_unit_type, option_inner, tuple_elements,
    wire_key, written,
};
use super::transport::http_rest::openapi::message_type;
use super::transport::mcp::prompts::prompt_entries;
use super::transport::mcp::resource_entries;
use crate::features::jsonschema::in_flight_type;
use crate::model_schema::needs_wrapper;
use crate::model_schema::type_json_schema_value;
use proc_macro2::TokenStream;
use quote::{ToTokens as _, format_ident, quote, quote_spanned};
use syn::{Ident, Type};

const DEFS_PREFIX: &str = "#/$defs/";

/// The members of `<Service>Schema` the `mcp` transport reads.
pub fn registry_items(service: &ServiceDef) -> TokenStream {
    let wrapped = service.operations.iter().map(wrapped_method);
    let forms = service.operations.iter().filter_map(interview_methods);
    let tools = tools_method(service);
    let instructions = service.doc.as_ref().map_or_else(
        || quote! { ::core::option::Option::None },
        |text| quote! { ::core::option::Option::Some(#text) },
    );
    let listed = service.operations.iter().filter_map(|operation| {
        if operation.is_mcp_tool() {
            return Some(("tool", operation.mcp_tool_name().to_owned()));
        }
        if operation.is_mcp_prompt() {
            return Some(("prompt", operation.mcp_tool_name().to_owned()));
        }
        if let Some((skill, _)) = &operation.mcp.skill {
            return Some(("skill", skill.clone()));
        }
        let resource = operation.mcp.resource.as_ref()?;
        let kind = if resource.is_template() {
            "resource template"
        } else {
            "resource"
        };
        Some((kind, resource.uri.clone()))
    });
    let folder_skills = service
        .skills
        .iter()
        .flat_map(|folder| folder.skills.iter())
        .map(|skill| ("skill", skill.name.clone()));
    let (kinds, names): (Vec<_>, Vec<_>) = listed.chain(folder_skills).unzip();
    let scopes = scopes_method(service);
    let listed_resources = resources_methods(service);
    let listed_prompts = prompts_method(service);
    let listed_skills = skills_methods(service);
    let sorted = sorted_fn();
    quote! {
        #sorted

        #listed_skills

        #tools

        #listed_resources

        #listed_prompts

        #scopes

        #(#wrapped)*

        #(#forms)*

        #[doc = " The trait's rustdoc, which the server answers as its `instructions`."]
        pub const MCP_INSTRUCTIONS: ::core::option::Option<&'static str> = #instructions;

        #[doc = " Every tool, prompt, resource and resource template the service lists, as `(<kind>, <name>)`."]
        pub const MCP_NAMES: &'static [(&'static str, &'static str)] = &[#((#kinds, #names)),*];
    }
}

/// `mcp_sorted(value)`: `value` with the members of every object in key order, whatever order the
/// `serde_json` of the invoking crate keeps them in, so each server writes the bytes Rust does.
fn sorted_fn() -> TokenStream {
    quote! {
        fn mcp_sorted(value: ::serde_json::Value) -> ::serde_json::Value {
            match value {
                ::serde_json::Value::Object(members) => {
                    let mut held: ::std::vec::Vec<(::std::string::String, ::serde_json::Value)> =
                        members.into_iter().collect();
                    held.sort_by(|left, right| left.0.cmp(&right.0));
                    ::serde_json::Value::Object(
                        held.into_iter()
                            .map(|(key, member)| (key, Self::mcp_sorted(member)))
                            .collect(),
                    )
                }
                ::serde_json::Value::Array(items) => ::serde_json::Value::Array(
                    items.into_iter().map(Self::mcp_sorted).collect(),
                ),
                other => other,
            }
        }
    }
}

/// `mcp_resources()` and `mcp_resource_templates()`: the entries `resources/list` and
/// `resources/templates/list` answer for the service, as the servers in other languages write them.
fn resources_methods(service: &ServiceDef) -> TokenStream {
    let (resources, templates) = resource_entries(service);
    quote! {
        #[doc = " The service's resources with a fixed URI, in the order the trait declares them."]
        pub fn mcp_resources() -> ::std::vec::Vec<::serde_json::Value> {
            ::std::vec![#(Self::mcp_sorted(::serde_json::json!(#resources))),*]
        }

        #[doc = " The service's resource templates, in the order the trait declares them."]
        pub fn mcp_resource_templates() -> ::std::vec::Vec<::serde_json::Value> {
            ::std::vec![#(Self::mcp_sorted(::serde_json::json!(#templates))),*]
        }
    }
}

/// `mcp_prompts()`: the entries `prompts/list` answers for the service, as the servers in other
/// languages write them.
fn prompts_method(service: &ServiceDef) -> TokenStream {
    let prompts = prompt_entries(service);
    quote! {
        #[doc = " The service's prompts, in the order the trait declares them."]
        pub fn mcp_prompts() -> ::std::vec::Vec<::serde_json::Value> {
            ::std::vec![#(Self::mcp_sorted(::serde_json::json!(#prompts))),*]
        }
    }
}

/// `mcp_skills()`, `mcp_skill_file(uri)` and `mcp_skill_files()`: the entries `skills/list` answers
/// for the service, and the content of each file of its skills folder, as the servers in other
/// languages write them.
fn skills_methods(service: &ServiceDef) -> TokenStream {
    let folder = service
        .skills
        .iter()
        .flat_map(|folder| folder.skills.iter());
    let entries: Vec<TokenStream> = folder
        .clone()
        .map(|skill| {
            let uri = skill.uri();
            let members = skill.frontmatter.iter().map(|(key, value)| quote! { #key: #value });
            let files = skill.files.iter().map(|file| {
                let (file_uri, digest, size) = (file.uri(&skill.name), &file.digest, file.size);
                quote! { { "uri": #file_uri, "digest": #digest, "size": #size } }
            });
            quote! { { "uri": #uri, "frontmatter": { #(#members),* }, "resources": [#(#files),*] } }
        })
        .chain(service.operations.iter().filter_map(|operation| {
            let (name, _) = operation.mcp.skill.as_ref()?;
            let uri = format!("skill://{name}/SKILL.md");
            let description = operation.doc.as_deref().unwrap_or_default();
            Some(quote! {
                { "uri": #uri, "frontmatter": { "name": #name, "description": #description }, "resources": "dynamic" }
            })
        }))
        .collect();
    let files: Vec<TokenStream> = service
        .skills
        .iter()
        .flat_map(|held| held.skills.iter().map(move |skill| (held, skill)))
        .flat_map(|(held, skill)| {
            skill.files.iter().map(move |file| {
                let uri = file.uri(&skill.name);
                let (mime, text) = (&file.mime, file.text);
                let source = format!("/{}", held.source(&skill.name, &file.path));
                quote! {
                    (#uri, #mime, #text, ::core::include_bytes!(::core::concat!(::core::env!("CARGO_MANIFEST_DIR"), #source)) as &[u8])
                }
            })
        })
        .collect();
    let held = files.len();
    quote! {
        #[doc = " The service's skills, in the order `skills/list` answers them: the folder's, then the"]
        #[doc = " ones its operations write."]
        pub fn mcp_skills() -> ::std::vec::Vec<::serde_json::Value> {
            ::std::vec![#(Self::mcp_sorted(::serde_json::json!(#entries))),*]
        }

        const MCP_SKILL_FILES: [(&'static str, &'static str, bool, &'static [u8]); #held] = [#(#files),*];

        #[doc = " What `resources/read` answers for the file of a skills folder at `uri`."]
        pub fn mcp_skill_file(uri: &str) -> ::core::option::Option<::serde_json::Value> {
            let (_, mime, text, bytes) = Self::MCP_SKILL_FILES.iter().find(|held| held.0 == uri)?;
            ::core::option::Option::Some(Self::mcp_sorted(if *text {
                ::serde_json::json!({
                    "uri": uri,
                    "mimeType": mime,
                    "text": ::std::string::String::from_utf8_lossy(bytes),
                })
            } else {
                ::serde_json::json!({ "uri": uri, "mimeType": mime, "blob": Self::mcp_base64(bytes) })
            }))
        }

        #[doc = " Every file of the skills folder, as `resources/read` answers it."]
        pub fn mcp_skill_files() -> ::std::vec::Vec<::serde_json::Value> {
            Self::MCP_SKILL_FILES
                .iter()
                .filter_map(|held| Self::mcp_skill_file(held.0))
                .collect()
        }

        fn mcp_base64(bytes: &[u8]) -> ::std::string::String {
            const ALPHABET: &[u8] =
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
            let letter = |group: u32, shift: u32| -> char {
                let at = usize::try_from((group >> shift) & 63).unwrap_or(0);
                ALPHABET.get(at).copied().map_or('=', char::from)
            };
            let mut text = ::std::string::String::new();
            for chunk in bytes.chunks(3) {
                let byte = |at: usize| u32::from(chunk.get(at).copied().unwrap_or(0));
                let group = (byte(0) << 16_u32) | (byte(1) << 8_u32) | byte(2);
                text.push(letter(group, 18));
                text.push(letter(group, 12));
                text.push(if chunk.len() > 1 { letter(group, 6) } else { '=' });
                text.push(if chunk.len() > 2 { letter(group, 0) } else { '=' });
            }
            text
        }
    }
}

/// The arms of `mcp_scopes` for `prompts/get`.
fn prompt_arms(service: &ServiceDef) -> Vec<TokenStream> {
    service
        .operations
        .iter()
        .filter(|operation| operation.is_mcp_prompt() && !operation.mcp.scopes.is_empty())
        .map(|operation| {
            let name = operation.mcp_tool_name();
            let scopes = scopes_of(operation);
            quote! { ("prompts/get", ::core::option::Option::Some(#name)) => #scopes, }
        })
        .collect()
}

/// The arms of `mcp_scopes` for `completion/complete`: a resource by its URI, a prompt by its name.
fn completion_arms(service: &ServiceDef) -> Vec<TokenStream> {
    service
        .operations
        .iter()
        .filter(|operation| operation.mcp.completes.is_some() && !operation.mcp.scopes.is_empty())
        .filter_map(|operation| {
            let completes = operation.mcp.completes.as_ref()?;
            let target = service
                .operations
                .iter()
                .find(|candidate| written(&candidate.ident) == completes.operation)?;
            let uri = match completes.kind {
                McpCompletesKind::Resource => target.mcp.resource.as_ref()?.uri.as_str(),
                McpCompletesKind::Prompt => {
                    target.mcp.prompt.as_ref()?;
                    target.mcp_tool_name()
                }
            };
            let scopes = scopes_of(operation);
            Some(quote! {
                ("completion/complete", ::core::option::Option::Some(#uri)) => #scopes,
            })
        })
        .collect()
}

fn scopes_of(operation: &OperationDef) -> TokenStream {
    let scopes = operation.mcp.scopes.iter().map(|(scope, _)| scope);
    quote! { &[#(#scopes),*] }
}

/// `mcp_scopes(method, name)`: the scopes the request `serve` routes to an operation needs.
fn scopes_method(service: &ServiceDef) -> TokenStream {
    let tools = service
        .operations
        .iter()
        .filter(|operation| operation.is_mcp_tool() && !operation.mcp.scopes.is_empty())
        .map(|operation| {
            let name = operation.mcp_tool_name();
            let scopes = scopes_of(operation);
            quote! { ("tools/call", ::core::option::Option::Some(#name)) => #scopes, }
        });
    let prompts = prompt_arms(service);
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
    let fixed =
        resourced().filter(|operation| operation.mcp.resource.is_some() && !is_template(operation));
    let templates = resourced().filter(|operation| is_template(operation));
    let checks: Vec<TokenStream> = fixed
        .chain(templates)
        .filter_map(|operation| {
            let resource = operation.mcp.resource.as_ref()?;
            let scopes = scopes_of(operation);
            Some(if resource.is_template() {
                let parts = resource.segments.iter().map(|segment| match segment {
                    PathSegment::Literal(text) => quote! { (#text, false) },
                    PathSegment::Placeholder(name) => quote! { (#name, true) },
                });
                quote! {
                    if matches_template(&[#(#parts),*], uri) {
                        return #scopes;
                    }
                }
            } else {
                let uri = &resource.uri;
                quote! {
                    if uri == #uri {
                        return #scopes;
                    }
                }
            })
        })
        .collect();
    let matcher = resourced().any(is_template).then(uri_matcher);
    let reads = (!checks.is_empty()).then(|| {
        quote! {
            ("resources/read", ::core::option::Option::Some(uri)) => {
                #(#checks)*
                &[]
            }
        }
    });
    let listing = resourced()
        .find(|operation| operation.mcp.list_resources && !operation.mcp.scopes.is_empty())
        .map(|operation| {
            let scopes = scopes_of(operation);
            quote! { ("resources/list", _) => #scopes, }
        });
    let completions = completion_arms(service);
    quote! {
        #[doc = " The scopes a caller needs for the request `method` and `name` name, `[]` for none."]
        pub fn mcp_scopes(
            method: &str,
            name: ::core::option::Option<&str>,
        ) -> &'static [&'static str] {
            #matcher
            match (method, name) {
                #(#tools)*
                #(#prompts)*
                #reads
                #listing
                #(#completions)*
                _ => &[],
            }
        }
    }
}

/// `matches_template(template, uri)`: whether `uri` is the template, each placeholder one
/// non-empty run of text up to the next `/` and the literal parts compared as written.
fn uri_matcher() -> TokenStream {
    quote! {
        fn matches_template(template: &[(&str, bool)], uri: &str) -> bool {
            let mut rest = uri;
            for (text, placeholder) in template {
                if *placeholder {
                    let end = rest.find('/').unwrap_or(rest.len());
                    if end == 0 {
                        return false;
                    }
                    rest = &rest[end..];
                } else if let ::core::option::Option::Some(remainder) = rest.strip_prefix(text) {
                    rest = remainder;
                } else {
                    return false;
                }
            }
            rest.is_empty()
        }
    }
}

pub fn wrapped_ident(operation: &OperationDef) -> syn::Ident {
    format_ident!(
        "wrapped_{}",
        written(&operation.ident),
        span = operation.ident.span()
    )
}

/// `wrapped_{operation}()`: whether the tool's arguments object holds the message under `value`.
fn wrapped_method(operation: &OperationDef) -> TokenStream {
    let name = wrapped_ident(operation);
    let binds_a_header = !HttpShape::of(operation).header_in.is_empty();
    let answer = match &operation.inputs {
        OperationInputs::Named(declared) if binds_a_header => {
            let _: &Type = declared;
            quote! { true }
        }
        OperationInputs::Named(declared) => needs_wrapper::of_type(declared),
        OperationInputs::Empty | OperationInputs::Generated(_) => quote! { false },
    };
    let doc = format!(
        " Whether the tool `{}` holds its message under `value`.",
        operation.mcp_tool_name()
    );
    quote! {
        #[doc = #doc]
        pub const fn #name() -> bool {
            #answer
        }
    }
}

/// A content tool's success is the pair of bytes and their content type.
pub fn is_content_pair(operation: &OperationDef) -> bool {
    operation.mcp.content.is_some()
        && matches!(
            &operation.outcome,
            OperationOutcome::Reply { success, error: _error }
                if matches!(success.as_ref(), Type::Tuple(_))
        )
}

/// A content tool's success is a list of blocks.
pub fn is_content_list(operation: &OperationDef) -> bool {
    operation.mcp.content.is_some()
        && matches!(
            &operation.outcome,
            OperationOutcome::Reply { success, error: _error }
                if !matches!(success.as_ref(), Type::Tuple(_))
        )
}

/// A prompt's success is a list of messages.
pub fn is_prompt_messages(operation: &OperationDef) -> bool {
    operation.is_mcp_prompt()
        && matches!(
            &operation.outcome,
            OperationOutcome::Reply { success, error: _error }
                if !matches!(success.as_ref(), Type::Path(path) if path.path.is_ident("String"))
        )
}

/// One `const` per form an operation asks with, held to `mcp_form_flat()` of the form's type: the
/// service macro cannot read the type, so the compiler refuses a form that is not flat.
pub fn form_assertions(service: &ServiceDef) -> TokenStream {
    let assertions = service.operations.iter().filter_map(|operation| {
        let answers = operation.mcp.interview.as_ref()?.answers.as_ref()?;
        let OperationInputs::Generated(fields) = &operation.inputs else {
            return None;
        };
        let (_, ty) = fields
            .iter()
            .find(|(field, _)| written(field) == written(answers))?;
        let form = option_inner(ty)?;
        let message = format!(
            "`{}` asks with `{}`, whose type `{}` is not a flat form: MCP takes text, numbers, \
             yes or no, and choices",
            written(&operation.ident),
            written(answers),
            form.to_token_stream().to_string().replace(' ', "")
        );
        Some(quote_spanned! { operation.ident.span()=>
            const _: () = assert!(<#form>::mcp_form_flat(), #message);
        })
    });
    let arms = service.operations.iter().filter_map(|operation| {
        let interview = operation.mcp.interview.as_ref()?;
        let OperationOutcome::Reply { error, success: _success } = &operation.outcome else {
            return None;
        };
        let arm = interview.arm.to_string();
        let message = format!(
            "`{}` asks with the arm `{arm}`, which serde does not write as a tagged variant of `{}`",
            written(&operation.ident),
            error.to_token_stream().to_string().replace(' ', "")
        );
        Some(quote_spanned! { operation.ident.span()=>
            const _: () = assert!(<#error>::mcp_variant_tag(#arm).is_some(), #message);
        })
    });
    quote! { #(#assertions)* #(#arms)* }
}

/// The name `{prefix}_{operation}` a method of the registry is published under.
fn method_ident(operation: &OperationDef, prefix: &str) -> Ident {
    format_ident!(
        "{prefix}_{}",
        written(&operation.ident),
        span = operation.ident.span()
    )
}

/// `mcp_arm_tag_{operation}()`, `mcp_arm_wire_{operation}()` and `mcp_arm_content_{operation}()`:
/// the key serde tags the arm an operation asks with, the name it writes the arm under and the key
/// it holds the arm's members under.
fn arm_tag_methods(operation: &OperationDef, error: &Type, arm: &str) -> TokenStream {
    let (tag_name, wire_name, content_name) = (
        method_ident(operation, "mcp_arm_tag"),
        method_ident(operation, "mcp_arm_wire"),
        method_ident(operation, "mcp_arm_content"),
    );
    let content_doc = format!(
        " The key serde holds the members of the arm `{}` asks with under, empty where the enum has none.",
        operation.mcp_tool_name()
    );
    let tag_doc = format!(
        " The key serde tags the arm `{}` asks with, empty where the enum has none.",
        operation.mcp_tool_name()
    );
    let wire_doc = format!(
        " The name serde writes the arm `{}` asks with.",
        operation.mcp_tool_name()
    );
    quote! {
        #[doc = #tag_doc]
        pub const fn #tag_name() -> &'static str {
            match <#error>::mcp_variant_tag(#arm) {
                ::core::option::Option::Some((tag, _, _)) => tag,
                ::core::option::Option::None => "",
            }
        }

        #[doc = #wire_doc]
        pub const fn #wire_name() -> &'static str {
            match <#error>::mcp_variant_tag(#arm) {
                ::core::option::Option::Some((_, wire, _)) => wire,
                ::core::option::Option::None => "",
            }
        }

        #[doc = #content_doc]
        pub const fn #content_name() -> &'static str {
            match <#error>::mcp_variant_tag(#arm) {
                ::core::option::Option::Some((_, _, content)) => content,
                ::core::option::Option::None => "",
            }
        }
    }
}

/// What the dispatcher asks of the types only the declaration can name: whether the declared error
/// is the arm that asks, the link and message that arm holds, the form's schema and the read of
/// its answer.
fn interview_methods(operation: &OperationDef) -> Option<TokenStream> {
    let interview = operation.mcp.interview.as_ref()?;
    let OperationOutcome::Reply {
        error,
        success: _success,
    } = &operation.outcome
    else {
        return None;
    };
    let arm = &interview.arm;
    let asks = method_ident(operation, "mcp_asks");
    let doc = format!(
        " Whether the error `{}` answered is the arm that asks the user.",
        operation.mcp_tool_name()
    );
    let tagged = arm_tag_methods(operation, error, &arm.to_string());
    let Some(answers) = interview.answers.as_ref() else {
        let link_doc = format!(
            " The link and the message of the arm `{}` asks the user with, where the error is it.",
            operation.mcp_tool_name()
        );
        return Some(quote! {
            #[doc = #link_doc]
            pub fn #asks(error: &#error) -> ::core::option::Option<(&str, &str)> {
                if let #error::#arm { url, message, .. } = error {
                    return ::core::option::Option::Some((url.as_str(), message.as_str()));
                }
                ::core::option::Option::None
            }

            #tagged
        });
    };
    let OperationInputs::Generated(fields) = &operation.inputs else {
        return None;
    };
    let (_, ty) = fields
        .iter()
        .find(|(field, _)| written(field) == written(answers))?;
    let form = option_inner(ty)?;
    let (schema_name, read_name) = (
        method_ident(operation, "mcp_form"),
        method_ident(operation, "mcp_read_form"),
    );
    let schema_doc = format!(
        " The schema of the form `{}` asks with.",
        operation.mcp_tool_name()
    );
    let run = run();
    let schema = described(&operation.ident.to_string(), form);
    Some(quote! {
        #[doc = #doc]
        pub fn #asks(error: &#error) -> bool {
            ::core::matches!(error, #error::#arm { .. })
        }

        #tagged

        #[doc = #schema_doc]
        pub fn #schema_name() -> ::serde_json::Value {
            #run
            let schema = #schema;
            let mut root = match schema {
                ::serde_json::Value::Object(members) => members,
                other => {
                    let mut members = ::serde_json::Map::new();
                    members.insert("type".to_owned(), ::serde_json::Value::String("object".to_owned()));
                    members.insert("allOf".to_owned(), ::serde_json::Value::Array(::std::vec![other]));
                    members
                }
            };
            if let ::core::option::Option::Some(::serde_json::Value::Object(properties)) =
                root.get_mut("properties")
            {
                for property in properties.values_mut() {
                    let present = property
                        .get("anyOf")
                        .and_then(::serde_json::Value::as_array)
                        .filter(|branches| branches.len() == 2)
                        .and_then(|branches| branches.iter().find(|branch| branch["type"] != "null"))
                        .cloned();
                    if let ::core::option::Option::Some(branch) = present {
                        *property = branch;
                    }
                }
            }
            hoisted_defs.retain(|_, described| !described.is_null());
            if !hoisted_defs.is_empty() {
                root.insert("$defs".to_owned(), ::serde_json::Value::Object(hoisted_defs));
            }
            ::serde_json::Value::Object(root)
        }

        #[doc = " Reads the answers of a form, or says why they do not read."]
        pub fn #read_name(content: &::serde_json::Value) -> ::core::result::Result<(), ::std::string::String> {
            ::serde_json::from_value::<#form>(content.clone())
                .map(|_| ())
                .map_err(|rejected| rejected.to_string())
        }
    })
}

fn tools_method(service: &ServiceDef) -> TokenStream {
    let tools = service
        .operations
        .iter()
        .filter(|operation| operation.is_mcp_tool())
        .map(tool_value);
    let helpers = helpers();
    quote! {
        #[doc = " The service's tools, one per operation in the order the trait declares them."]
        pub fn mcp_tools() -> ::std::vec::Vec<::serde_json::Value> {
            #helpers
            ::std::vec![#(Self::mcp_sorted(#tools)),*]
        }
    }
}

/// One tool as the `Tool` of the MCP schema writes it.
fn tool_value(operation: &OperationDef) -> TokenStream {
    let name = operation.mcp_tool_name();
    let title = operation.mcp_tool_title();
    let description = operation.doc.clone().unwrap_or_default();
    let input = input_schema(operation);
    let output = output_schema(operation).map(|schema| {
        quote! { tool.insert("outputSchema".to_owned(), #schema); }
    });
    let annotations = annotations(operation);
    let icons = operation.mcp.icon.as_ref().map(|source| {
        quote! {
            tool.insert(
                "icons".to_owned(),
                ::serde_json::json!([{ "src": #source }]),
            );
        }
    });
    quote! {
        {
            let mut tool = ::serde_json::Map::new();
            tool.insert("name".to_owned(), ::serde_json::Value::String(#name.to_owned()));
            tool.insert("title".to_owned(), ::serde_json::Value::String(#title.to_owned()));
            tool.insert(
                "description".to_owned(),
                ::serde_json::Value::String(#description.to_owned()),
            );
            tool.insert("inputSchema".to_owned(), #input);
            #output
            #annotations
            #icons
            ::serde_json::Value::Object(tool)
        }
    }
}

fn annotations(operation: &OperationDef) -> Option<TokenStream> {
    let mcp = &operation.mcp;
    let mut hints: Vec<TokenStream> = Vec::new();
    if mcp.read_only {
        hints.push(quote! { "readOnlyHint": true });
    }
    if let Some(destructive) = mcp.destructive {
        hints.push(quote! { "destructiveHint": #destructive });
    }
    if mcp.idempotent {
        hints.push(quote! { "idempotentHint": true });
    }
    if let Some(open_world) = mcp.open_world {
        hints.push(quote! { "openWorldHint": #open_world });
    }
    (!hints.is_empty()).then(|| {
        quote! {
            tool.insert("annotations".to_owned(), ::serde_json::json!({ #(#hints),* }));
        }
    })
}

/// The statements that bind the run a schema is described in: the names a rendering of a type
/// reads.
fn run() -> TokenStream {
    let in_flight_type = in_flight_type();
    quote! {
        let mut in_flight: #in_flight_type = ::std::vec::Vec::new();
        let mut hoisted_defs = ::serde_json::Map::new();
        let pointer_prefix: &str = #DEFS_PREFIX;
    }
}

/// `ty` described in the run [`run`] binds.
fn described(subject: &str, ty: &Type) -> TokenStream {
    let schema = type_json_schema_value(subject, ty);
    quote! {
        {
            let in_flight = &mut in_flight;
            let hoisted_defs = &mut hoisted_defs;
            #schema
        }
    }
}

/// One header, as the name it is a property under, its schema, and whether it is required.
fn header_property(subject: &str, name: &str, ty: &Type) -> TokenStream {
    let schema = described(subject, ty);
    let required = option_inner(ty).is_none();
    quote! { (#name, #schema, #required) }
}

fn input_schema(operation: &OperationDef) -> TokenStream {
    let subject = operation.ident.to_string();
    let shape = HttpShape::of(operation);
    let run = run();
    let message = message_type(operation).map_or_else(
        || quote! { ::serde_json::json!({ "type": "object" }) },
        |declared| described(&subject, &declared),
    );
    let headers = shape
        .header_in
        .iter()
        .map(|bound| header_property(&subject, &bound.name, &bound.ty));
    let wrapped = match &operation.inputs {
        OperationInputs::Named(_) => {
            let method = wrapped_ident(operation);
            quote! { Self::#method() }
        }
        OperationInputs::Empty | OperationInputs::Generated(_) => quote! { false },
    };
    let marks = header_marks(operation, &shape);
    let answers = answers_key(operation);
    let hidden = answers.iter();
    quote! {
        {
            #run
            let message = #message;
            let headers = ::std::vec![#(#headers),*];
            arguments_object(message, #wrapped, headers, &[#(#marks),*], &[#(#hidden),*], hoisted_defs)
        }
    }
}

/// The wire key of the argument an `interview(..., answers = ...)` carries the form's answers in:
/// the model cannot fill it, so the tool's input leaves it out.
pub fn answers_key(operation: &OperationDef) -> Option<String> {
    let answers = operation.mcp.interview.as_ref()?.answers.as_ref()?;
    match &operation.inputs {
        OperationInputs::Generated(fields) => fields
            .iter()
            .find(|(field, _)| written(field) == written(answers))
            .map(|(field, _)| wire_key(field)),
        OperationInputs::Empty | OperationInputs::Named(_) => None,
    }
}

/// Each `mcp(header(...))` as the property it marks and the header it is mirrored into.
fn header_marks(operation: &OperationDef, shape: &HttpShape) -> Vec<TokenStream> {
    operation
        .mcp
        .header
        .iter()
        .filter_map(|(argument, header)| {
            let name = written(argument);
            let key = match &operation.inputs {
                OperationInputs::Generated(fields) => fields
                    .iter()
                    .find(|(field, _)| written(field) == name)
                    .map(|(field, _)| wire_key(field)),
                OperationInputs::Empty | OperationInputs::Named(_) => None,
            }
            .or_else(|| {
                shape
                    .header_in
                    .iter()
                    .find(|bound| written(&bound.parameter) == name)
                    .map(|bound| bound.name.clone())
            })?;
            Some(quote! { (#key, #header) })
        })
        .collect()
}

fn output_schema(operation: &OperationDef) -> Option<TokenStream> {
    let OperationOutcome::Reply {
        success,
        error: _error,
    } = &operation.outcome
    else {
        return None;
    };
    if operation.mcp.content.is_some() {
        return None;
    }
    let shape = HttpShape::of(operation);
    let subject = operation.ident.to_string();
    let elements: Vec<Type> = tuple_elements(success)
        .map(|elements| elements.iter().cloned().collect())
        .unwrap_or_default();
    let (body, first_header): (Type, usize) = if shape.header_out.is_empty() {
        if is_unit_type(success) {
            return None;
        }
        (success.as_ref().clone(), 0)
    } else if matches!(shape.body_kind, BodyKind::Bytes) {
        let (bytes, content_type) = (elements.first()?, elements.get(1)?);
        (syn::parse_quote! { (#bytes, #content_type) }, 2)
    } else {
        (elements.first()?.clone(), 1)
    };
    let run = run();
    let body_schema = described(&subject, &body);
    let headers = shape
        .header_out
        .iter()
        .zip(elements.iter().skip(first_header))
        .map(|(name, ty)| header_property(&subject, name, ty));
    Some(quote! {
        {
            #run
            let success = #body_schema;
            let headers = ::std::vec![#(#headers),*];
            output_object(success, headers, hoisted_defs)
        }
    })
}

/// The two functions every tool's schemas are finished by, written once inside `mcp_tools()`:
/// the arguments object of an input, and the object of an output.
fn helpers() -> TokenStream {
    let objects = object_helpers();
    quote! {
        type Members = ::serde_json::Map<::std::string::String, ::serde_json::Value>;
        type Headers = ::std::vec::Vec<(&'static str, ::serde_json::Value, bool)>;

        fn text(written: &str) -> ::serde_json::Value {
            ::serde_json::Value::String(written.to_owned())
        }

        fn members_of(schema: ::serde_json::Value) -> Members {
            match schema {
                ::serde_json::Value::Object(members) => members,
                other => {
                    let mut members = Members::new();
                    members.insert("allOf".to_owned(), ::serde_json::Value::Array(::std::vec![other]));
                    members
                }
            }
        }

        fn beside_value(message: ::serde_json::Value, headers: Headers) -> Members {
            let mut properties = Members::new();
            properties.insert("value".to_owned(), message);
            let mut required = ::std::vec![text("value")];
            for (name, schema, needed) in headers {
                properties.insert(name.to_owned(), schema);
                if needed {
                    required.push(text(name));
                }
            }
            let mut root = Members::new();
            root.insert("additionalProperties".to_owned(), ::serde_json::Value::Bool(false));
            root.insert("properties".to_owned(), ::serde_json::Value::Object(properties));
            root.insert("required".to_owned(), ::serde_json::Value::Array(required));
            root
        }

        #objects
    }
}

/// The functions that finish an input and an output schema, written once inside `mcp_tools()`.
fn object_helpers() -> TokenStream {
    quote! {
        fn arguments_object(
            message: ::serde_json::Value,
            wrapped: bool,
            headers: Headers,
            marks: &[(&str, &str)],
            hidden: &[&str],
            defs: Members,
        ) -> ::serde_json::Value {
            let mut root = if wrapped {
                beside_value(message, headers)
            } else {
                let mut root = members_of(message);
                if !headers.is_empty() {
                    let mut properties = match root.remove("properties") {
                        ::core::option::Option::Some(::serde_json::Value::Object(held)) => held,
                        _ => Members::new(),
                    };
                    let mut required = match root.remove("required") {
                        ::core::option::Option::Some(::serde_json::Value::Array(held)) => held,
                        _ => ::std::vec::Vec::new(),
                    };
                    for (name, schema, needed) in headers {
                        properties.insert(name.to_owned(), schema);
                        if needed {
                            required.push(text(name));
                        }
                    }
                    root.insert("properties".to_owned(), ::serde_json::Value::Object(properties));
                    root.insert("required".to_owned(), ::serde_json::Value::Array(required));
                }
                root
            };
            for (key, header) in marks {
                if let ::core::option::Option::Some(::serde_json::Value::Object(properties)) =
                    root.get_mut("properties")
                    && let ::core::option::Option::Some(::serde_json::Value::Object(property)) =
                        properties.get_mut(*key)
                {
                    property.insert("x-mcp-header".to_owned(), text(header));
                }
            }
            for key in hidden {
                if let ::core::option::Option::Some(::serde_json::Value::Object(properties)) =
                    root.get_mut("properties")
                {
                    properties.remove(*key);
                }
                if let ::core::option::Option::Some(::serde_json::Value::Array(required)) =
                    root.get_mut("required")
                {
                    required.retain(|named| named.as_str() != ::core::option::Option::Some(*key));
                }
            }
            root.entry("type".to_owned()).or_insert_with(|| text("object"));
            defs_joined(root, defs)
        }

        fn output_object(
            success: ::serde_json::Value,
            headers: Headers,
            defs: Members,
        ) -> ::serde_json::Value {
            let root = if headers.is_empty() {
                members_of(success)
            } else {
                let mut root = beside_value(success, headers);
                root.insert("type".to_owned(), text("object"));
                root
            };
            defs_joined(root, defs)
        }

        fn defs_joined(mut root: Members, mut defs: Members) -> ::serde_json::Value {
            defs.retain(|_, described| !described.is_null());
            if !defs.is_empty() {
                root.insert("$defs".to_owned(), ::serde_json::Value::Object(defs));
            }
            ::serde_json::Value::Object(root)
        }
    }
}
