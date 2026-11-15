//! The walk of a value behind a type parameter. A type that declares type parameters gains
//! `model_schema_issues_with{n}`, its `validation_issues()` with one check per parameter run on
//! every value behind it, and its holder, which knows what fills each parameter, hands it the
//! checks of what fills them. A holder that reads a generic model calls its
//! `model_schema_issues_with{n}` with the walk of each type argument, so it judges what the model
//! holds behind its parameters once, with every message at its full path.

use core::cell::{Cell, RefCell};
use std::collections::HashMap;

use proc_macro2::{Ident, Span, TokenStream};
use quote::{ToTokens as _, format_ident, quote};
use syn::Item;

use crate::model_schema::prefixed_guard_message;
use crate::utils::{declared, to_snake_case, type_parameters_in_scope, written_type};

/// The label of the refusal [`rearranged`] words.
pub const REARRANGED_LABEL: &str =
    "write the generic model the alias names, with its parameters filled, in place of the alias";

/// The note of the refusal [`rearranged`] words.
pub const REARRANGED_NOTE: &str = "for `type PagesOf<Item> = Page<Page<Item>>`, a member that holds `PagesOf<Parcel>` is written `Page<Page<Parcel>>`";

thread_local! {
    static WALK_SCOPE: RefCell<WalkScope> = RefCell::new(WalkScope::default());
    /// Whether the item being expanded runs a rule, or walks a declared type, where no value
    /// behind a parameter is counted.
    static RULED: Cell<bool> = const { Cell::new(false) };
    /// The walk of the values behind the item's parameters, where it is all `validate()` runs.
    static PARAMETERS_ALONE: RefCell<Option<TokenStream>> = const { RefCell::new(None) };
    /// Each alias that rearranges the parameters of a generic model, by name.
    static REARRANGING: RefCell<HashMap<String, Rearranging>> = RefCell::new(HashMap::new());
}

/// An alias that writes the parameters of a generic model in another shape than the model
/// declares them, as its expansion recorded it: its own parameters, and the type it names.
#[derive(Clone)]
struct Rearranging {
    parameters: Vec<String>,
    target: String,
}

/// The item being expanded as its walks read it.
#[derive(Clone, Default)]
pub struct WalkScope {
    /// The item's type parameters, in the order it declares them.
    pub parameters: Vec<String>,
    /// The item's name, which a member written `Self` holds at the item's own parameters.
    pub type_name: String,
}

/// What the refusal of a type argument that is not what fills the model's parameter says, where
/// `model` stands for what fills it and `written` for the argument: the member's type is an alias
/// that writes the parameters of the generic model it names in another shape.
pub fn rearranged(model: &str, written: &str) -> String {
    format!(
        "model_schema: `{model}` fills a type parameter of the generic model this member holds, \
         where the member's type writes `{written}`: the member's type is an alias that \
         rearranges the parameters of the model it names, and a holder hands the model what \
         fills each of its own parameters"
    )
}

/// Reads `item`'s type parameters and name for the walks of its expansion.
pub fn enter(item: &Item) {
    let declared_as = if let Item::Struct(item_struct) = item {
        Some((&item_struct.generics, &item_struct.ident))
    } else if let Item::Enum(item_enum) = item {
        Some((&item_enum.generics, &item_enum.ident))
    } else if let Item::Type(alias) = item {
        Some((&alias.generics, &alias.ident))
    } else {
        None
    };
    let entered = declared_as.map_or_else(WalkScope::default, |(generics, name)| WalkScope {
        parameters: type_parameters_in_scope(generics),
        type_name: name.to_string(),
    });
    WALK_SCOPE.with(|scope| *scope.borrow_mut() = entered);
    RULED.with(|ruled| ruled.set(false));
    PARAMETERS_ALONE.with(|alone| alone.borrow_mut().take());
}

/// Notes that the item being expanded runs a rule, or walks a declared type: what it publishes of
/// `validate()` is then what it would publish with no value behind a parameter walked.
pub fn note_ruled() {
    RULED.with(|ruled| ruled.set(true));
}

/// Whether [`note_ruled`] was called for the item being expanded.
pub fn ruled() -> bool {
    RULED.with(Cell::get)
}

/// Keeps `walk`, the body of `model_schema_issues_with{n}` of an item that runs nothing but the
/// checks of what fills its parameters, for [`parameters_alone`].
pub fn keep_parameters_alone(walk: TokenStream) {
    PARAMETERS_ALONE.with(|alone| *alone.borrow_mut() = Some(walk));
}

/// What [`keep_parameters_alone`] kept, taken.
pub fn parameters_alone() -> Option<TokenStream> {
    PARAMETERS_ALONE.with(|alone| alone.borrow_mut().take())
}

/// The item being expanded, as [`enter`] read it.
pub fn scope() -> WalkScope {
    WALK_SCOPE.with(|scope| scope.borrow().clone())
}

/// The walk that takes one check per type parameter, on a type of `arity` of them.
pub fn issues_with(arity: usize) -> Ident {
    format_ident!("model_schema_issues_with{arity}")
}

/// The check `model_schema_issues_with{n}` runs on each value behind `parameter`.
pub fn check_of(parameter: &str) -> Ident {
    Ident::new(
        &format!("model_schema_issues_of_{}", to_snake_case(parameter)),
        Span::call_site(),
    )
}

/// The type arguments of the declared type a walk ends on, `segment`: those it is written with, or
/// the item's own parameters where it is written `Self`, which holds the item at them.
pub fn leaf_arguments(segment: &syn::PathSegment) -> Vec<syn::Type> {
    if let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments {
        return arguments
            .args
            .iter()
            .filter_map(|argument| match argument {
                syn::GenericArgument::Type(ty) => Some(ty.clone()),
                syn::GenericArgument::Lifetime(_)
                | syn::GenericArgument::Const(_)
                | syn::GenericArgument::AssocType(_)
                | syn::GenericArgument::AssocConst(_)
                | syn::GenericArgument::Constraint(_)
                | _ => None,
            })
            .collect();
    }
    if segment.ident != "Self" {
        return Vec::new();
    }
    scope()
        .parameters
        .iter()
        .filter_map(|parameter| syn::parse_str::<syn::Type>(parameter).ok())
        .collect()
}

/// `ty` as an argument is written where a walk names it: `Self` is the item at its own
/// parameters, which the draft's members hold as the valid type.
pub fn argument_written(ty: &syn::Type) -> TokenStream {
    let walked = scope();
    let name = Ident::new(&walked.type_name, Span::call_site());
    let parameters: Vec<Ident> = walked
        .parameters
        .iter()
        .map(|parameter| Ident::new(parameter, Span::call_site()))
        .collect();
    let named = if parameters.is_empty() {
        quote! { #name }
    } else {
        quote! { #name<#(#parameters),*> }
    };
    self_written_as(ty.to_token_stream(), &named)
}

fn self_written_as(tokens: TokenStream, named: &TokenStream) -> TokenStream {
    tokens
        .into_iter()
        .flat_map(|token| match token {
            proc_macro2::TokenTree::Group(group) => {
                let mut inner = proc_macro2::Group::new(
                    group.delimiter(),
                    self_written_as(group.stream(), named),
                );
                inner.set_span(group.span());
                TokenStream::from(proc_macro2::TokenTree::Group(inner))
            }
            proc_macro2::TokenTree::Ident(ident) if ident == "Self" => named.clone(),
            other @ (proc_macro2::TokenTree::Ident(_)
            | proc_macro2::TokenTree::Literal(_)
            | proc_macro2::TokenTree::Punct(_)) => TokenStream::from(other),
        })
        .collect()
}

/// What a walk that hands checks to a generic model declares beside it: what turns the value a
/// check is handed into the type argument it was written for, refused where the two differ, and
/// what answers `model_schema_issues_with{n}` for each of `arities` on a type that has no such
/// walk, the value then being nothing the check is ever handed.
pub fn issues_with_fallbacks(arities: &[usize]) -> TokenStream {
    if arities.is_empty() {
        return TokenStream::new();
    }
    let fallbacks = arities.iter().map(|arity| {
        let named = format_ident!("UnpublishedIssuesWith{arity}");
        let walk = issues_with(*arity);
        let held: Vec<Ident> = (1..=*arity).map(|at| format_ident!("Held{at}")).collect();
        quote! {
            trait #named {
                fn #walk<#(#held: ?::core::marker::Sized),*>(
                    &self,
                    #(_: &dyn ::core::ops::Fn(&ModelSchemaUnreached<#held>) -> ::std::vec::Vec<::std::string::String>),*
                ) -> ::std::vec::Vec<::std::string::String> {
                    ::std::vec::Vec::new()
                }
            }
            impl<T: ?::core::marker::Sized> #named for &T {}
        }
    });
    let (message, label, note) = (
        rearranged("{Self}", "{W}"),
        REARRANGED_LABEL,
        REARRANGED_NOTE,
    );
    quote! {
        #[diagnostic::on_unimplemented(message = #message, label = #label, note = #note)]
        trait ModelSchemaWrittenAs<W: ?::core::marker::Sized> {
            fn model_schema_written(&self) -> &W;
        }
        impl<W: ?::core::marker::Sized> ModelSchemaWrittenAs<W> for W {
            fn model_schema_written(&self) -> &W {
                self
            }
        }
        enum ModelSchemaUnreached<W: ?::core::marker::Sized> {
            Never(::core::convert::Infallible, ::core::marker::PhantomData<W>),
        }
        impl<W: ?::core::marker::Sized> ModelSchemaWrittenAs<W> for ModelSchemaUnreached<W> {
            fn model_schema_written(&self) -> &W {
                match self {
                    Self::Never(never, _) => match *never {},
                }
            }
        }
        #(#fallbacks)*
    }
}

/// The signature of `model_schema_issues_with{n}` on a type that declares `parameters`, whose body
/// is `body`: a check `body` never runs, behind a parameter no member holds a value of, is taken
/// unnamed.
pub fn issues_with_signature(parameters: &[String], body: &TokenStream) -> TokenStream {
    let walk = issues_with(parameters.len());
    let checks = parameters.iter().map(|parameter| {
        let (check, held) = (
            check_of(parameter),
            Ident::new(parameter, Span::call_site()),
        );
        let named = if names(body, &check) {
            quote! { #check }
        } else {
            quote! { _ }
        };
        quote! { #named: &dyn ::core::ops::Fn(&#held) -> ::std::vec::Vec<::std::string::String> }
    });
    quote! {
        pub fn #walk(&self, #(#checks),*) -> ::std::vec::Vec<::std::string::String>
    }
}

/// The call of `model_schema_issues_with{n}` that is `validation_issues()`: a check that reports
/// nothing for every parameter.
pub fn issues_with_nothing(parameters: &[String]) -> TokenStream {
    let walk = issues_with(parameters.len());
    let nothing = parameters
        .iter()
        .map(|_| quote! { &|_| ::std::vec::Vec::new() });
    quote! { self.#walk(#(#nothing),*) }
}

/// `model_schema_issues_with{n}` of a type that publishes no `validate()` of its own: `walk`, the
/// walk of the values behind its parameters, where it holds any, and nothing otherwise.
pub fn issues_with_ignoring(parameters: &[String], walk: Option<&TokenStream>) -> TokenStream {
    let body = walk.map_or_else(
        || quote! { ::std::vec::Vec::new() },
        |walked| quote! { #walked },
    );
    let signature = issues_with_signature(parameters, &body);
    quote! {
        /// `validation_issues()`, with each check handed in run on every value behind its type
        /// parameter, for a holder that knows what fills each.
        #signature {
            #body
        }
    }
}

/// `model_schema_issues_with{n}` on the valid type, which runs the draft's.
pub fn issues_with_delegated(parameters: &[String], draft: &TokenStream) -> TokenStream {
    let walk = issues_with(parameters.len());
    let checks: Vec<Ident> = parameters
        .iter()
        .map(|parameter| check_of(parameter))
        .collect();
    let body = quote! { <#draft>::#walk(self, #(#checks),*) };
    let signature = issues_with_signature(parameters, &body);
    quote! {
        /// The draft's `validation_issues()` with each check handed in run on every value behind
        /// its type parameter, for a holder that knows what fills each.
        #signature {
            #body
        }
    }
}

/// Whether `tokens` write `ident`, at any depth.
fn names(tokens: &TokenStream, ident: &Ident) -> bool {
    tokens.clone().into_iter().any(|tree| match tree {
        proc_macro2::TokenTree::Ident(written) => written == *ident,
        proc_macro2::TokenTree::Group(group) => names(&group.stream(), ident),
        proc_macro2::TokenTree::Punct(_) | proc_macro2::TokenTree::Literal(_) => false,
    })
}

/// Records the alias `alias` where it names a generic model declared above with its parameters in
/// another shape than the model's own, and forgets the name otherwise: a later declaration under
/// the same name is no such alias. An alias of a std type, one that declares no parameter, and one
/// that names the model with its own parameters in order, are no such alias.
pub fn record_alias(alias: &syn::ItemType) {
    let name = alias.ident.to_string();
    let parameters = type_parameters_in_scope(&alias.generics);
    let rearranging = rearranges(&alias.ty, &parameters).then(|| Rearranging {
        parameters: parameters.clone(),
        target: alias.ty.to_token_stream().to_string(),
    });
    REARRANGING.with(|recorded| {
        let mut held = recorded.borrow_mut();
        match rearranging {
            Some(found) => {
                held.insert(name, found);
            }
            None => {
                held.remove(&name);
            }
        }
    });
}

/// Whether `target`, an alias's type, names a generic model declared above with type arguments
/// that are not `parameters`, the alias's own, in order.
fn rearranges(target: &syn::Type, parameters: &[String]) -> bool {
    // An alias of no parameter is written with no argument, and its holder hands nothing.
    if parameters.is_empty() {
        return false;
    }
    let syn::Type::Path(path) = written_type(target) else {
        return false;
    };
    let Some(last) = path.path.segments.last() else {
        return false;
    };
    if declared(&last.ident.to_string()).is_none() {
        return false;
    }
    let arguments = leaf_arguments(last);
    if arguments.is_empty() {
        return false;
    }
    let written: Vec<String> = arguments
        .iter()
        .map(|argument| argument.to_token_stream().to_string())
        .collect();
    written != parameters
}

/// The refusal of each alias recorded by [`record_alias`] that `ty`, the type of the member
/// `label`, writes, with the type to write in its place.
pub fn rearranged_alias_errors(ty: &syn::Type, label: &str) -> Vec<TokenStream> {
    let mut errors = Vec::new();
    rearranged_in(ty, label, &mut errors);
    errors
}

fn rearranged_in(ty: &syn::Type, label: &str, errors: &mut Vec<TokenStream>) {
    let written = written_type(ty);
    match written {
        syn::Type::Path(path) => {
            for segment in &path.path.segments {
                if let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments {
                    for argument in &arguments.args {
                        if let syn::GenericArgument::Type(held) = argument {
                            rearranged_in(held, label, errors);
                        }
                    }
                }
            }
            let Some(last) = path.path.segments.last() else {
                return;
            };
            let name = last.ident.to_string();
            let Some(recorded) = REARRANGING.with(|held| held.borrow().get(&name).cloned()) else {
                return;
            };
            let arguments = leaf_arguments(last);
            let filled = filled_target(&recorded, &arguments);
            let message = prefixed_guard_message(&format!(
                "{label}: `{written}` is the alias `type {name}<{parameters}> = {target}`, which \
                 rearranges the parameters of the generic model it names, and a holder hands a \
                 generic model what fills each of its own parameters. Write `{filled}` in place \
                 of `{written}`.",
                written = compact(&quote! { #path }.to_string()),
                parameters = recorded.parameters.join(", "),
                target = compact(&recorded.target),
            ));
            errors.push(syn::Error::new_spanned(path, message).to_compile_error());
        }
        syn::Type::Array(array) => rearranged_in(&array.elem, label, errors),
        syn::Type::Slice(slice) => rearranged_in(&slice.elem, label, errors),
        syn::Type::Reference(reference) => rearranged_in(&reference.elem, label, errors),
        syn::Type::Tuple(tuple) => {
            for position in &tuple.elems {
                rearranged_in(position, label, errors);
            }
        }
        syn::Type::FnPtr(_)
        | syn::Type::Group(_)
        | syn::Type::ImplTrait(_)
        | syn::Type::Infer(_)
        | syn::Type::Macro(_)
        | syn::Type::Never(_)
        | syn::Type::Paren(_)
        | syn::Type::Ptr(_)
        | syn::Type::TraitObject(_)
        | syn::Type::Verbatim(_)
        | _ => {}
    }
}

/// The type `recorded` names with each of its parameters written as the argument it was written
/// with, `arguments`.
fn filled_target(recorded: &Rearranging, arguments: &[syn::Type]) -> String {
    let filled: Vec<(String, TokenStream)> = recorded
        .parameters
        .iter()
        .zip(arguments)
        .map(|(parameter, argument)| (parameter.clone(), argument.to_token_stream()))
        .collect();
    let target: TokenStream = recorded.target.parse().unwrap_or_default();
    compact(&words_filled(target, &filled).to_string())
}

/// A type's text as its author writes it: no space around `<`, `>` and `::`, nor ahead of `,`.
fn compact(text: &str) -> String {
    text.replace(" <", "<")
        .replace("< ", "<")
        .replace(" >", ">")
        .replace(" ,", ",")
        .replace(" :: ", "::")
}

fn words_filled(tokens: TokenStream, filled: &[(String, TokenStream)]) -> TokenStream {
    tokens
        .into_iter()
        .flat_map(|token| match token {
            proc_macro2::TokenTree::Group(group) => {
                let inner = proc_macro2::Group::new(
                    group.delimiter(),
                    words_filled(group.stream(), filled),
                );
                TokenStream::from(proc_macro2::TokenTree::Group(inner))
            }
            proc_macro2::TokenTree::Ident(ident) => filled
                .iter()
                .find(|(parameter, _)| ident == parameter)
                .map_or_else(
                    || TokenStream::from(proc_macro2::TokenTree::Ident(ident.clone())),
                    |(_, argument)| argument.clone(),
                ),
            other @ (proc_macro2::TokenTree::Literal(_) | proc_macro2::TokenTree::Punct(_)) => {
                TokenStream::from(other)
            }
        })
        .collect()
}
