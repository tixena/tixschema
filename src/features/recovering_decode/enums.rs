//! The walkers of an enum, one per form serde writes an enum in.
//!
//! A plain enum is one value. A tagged enum is walked by its tag: the tag is read first, and what
//! the variant it names holds is walked where that form writes it. An untagged enum is walked as
//! the variant serde reads the value as.

use core::iter::once;

use proc_macro2::{Ident, Span, TokenStream};
use quote::{ToTokens as _, format_ident, quote};
use syn::ext::IdentExt as _;
use syn::{Fields, ItemEnum, Type, parse_quote};

use super::{
    Arm, Lookup, RecoveringDecode, Shape, Step, Walk, WalkedField, Walker, added_to, binding,
    not_the_shape, path_expression, undeclared_keys, written_names,
};
use crate::features::serde::{
    parse_serde_field_attributes, parse_serde_key_omission, parse_serde_type_attributes,
};
use crate::field_type::{get_field_def, is_plain_enum};
use crate::rename_rule::resolve_rename_rule;
use crate::utils::{ident_schema_module_name, to_snake_case, type_parameters_in_scope};

/// The form serde writes an enum in, read off its attributes as the schema surfaces read them.
enum Tagging {
    /// `tag` and `content`: the variant's name under one key, what it holds under another.
    Adjacent { content: String, tag: String },
    /// No attribute: the variant's name as the one key of an object, over what it holds.
    External,
    /// `tag` alone: the variant's name under a key of the object its fields are written in.
    Internal { tag: String },
    /// No attribute, and no variant holding a value: the variant's name, as text.
    Plain,
    /// `untagged`: what the variant holds, and nothing naming it.
    Untagged,
}

impl Tagging {
    fn of(item_enum: &ItemEnum) -> Self {
        let container = parse_serde_type_attributes(&item_enum.attrs);
        if container.untagged {
            return Self::Untagged;
        }
        match (container.tag, container.content) {
            (None, None) if is_plain_enum(item_enum) => Self::Plain,
            (None, None) => Self::External,
            (Some(tag), None) => Self::Internal { tag },
            (tag, Some(content)) => Self::Adjacent {
                content,
                tag: tag.unwrap_or_else(|| "type".to_owned()),
            },
        }
    }
}

/// One variant as its enum's walker reads it.
struct WalkedVariant<'item> {
    /// Every `alias`: a tag serde reads the variant under beside its name.
    aliases: Vec<String>,
    /// What the variant holds, in the form serde writes it beside the variant's name.
    content: Shape<'item>,
    /// The name serde writes the variant under.
    name: String,
    /// serde reads no value as the variant: it is under `skip_deserializing` or `skip`.
    never_read: bool,
    /// The variant as a pattern, whatever it holds: `Self::Email { .. }`.
    pattern: TokenStream,
    rust_name: String,
    /// The variant's name inside the names of what the flag adds for it.
    stem: String,
}

impl WalkedVariant<'_> {
    /// Every tag serde reads the variant under: its name, then its aliases in the order written.
    fn tags(&self) -> impl Iterator<Item = &String> {
        once(&self.name).chain(&self.aliases)
    }
}

/// The walker of one enum for one source: what the type's own walker names, and the enum's
/// variants.
struct EnumWalker<'walk> {
    /// The variants serde reads no value as, which are no variant to the walker.
    never_read: &'walk [WalkedVariant<'walk>],
    /// The variants serde reads, in the order declared.
    variants: &'walk [WalkedVariant<'walk>],
    walker: &'walk Walker<'walk>,
}

impl EnumWalker<'_> {
    /// What `decode_with_{source}_fields` of an adjacently tagged enum runs: the tag read under
    /// `tag`, then what the variant it names holds walked under `content`. Both keys are the
    /// enum's own.
    fn adjacent_fields(&self, tag: &str, content: &str) -> TokenStream {
        let expected = self.variants_expected();
        let arms = self.variants.iter().map(|variant| {
            let tags = variant.tags();
            let walked = self.keyed_content(&variant.content, content);
            quote! { Some(#(#tags)|*) => #walked, }
        });
        let missing = missing_tag(tag, &expected);
        let unknown = self.unknown_tag(tag, &expected);
        quote! {
            let Some(tag) = object.get(#tag) else {
                #missing
                return vec![#tag, #content];
            };
            match tag.as_str() {
                #(#arms)*
                _ => { #unknown }
            }
            vec![#tag, #content]
        }
    }

    /// The arms what a variant holds is matched by, held under `held` at `segments`.
    fn content_arms(
        &self,
        content: &Shape<'_>,
        held: &Ident,
        segments: &[TokenStream],
    ) -> Vec<Arm> {
        match content {
            Shape::Fields { declared, fields } => vec![Arm {
                body: self.held_fields(declared, fields, held, segments),
                nothing: false,
                pattern: quote! { #held },
            }],
            Shape::Held(walk) => self.walker.arms(walk, held, segments, 0, walk.ty),
            Shape::Nothing => Vec::new(),
            Shape::Slots(slots) => {
                let items = binding("items", 0);
                let positions = self.walker.positions(slots, &items, segments, 0);
                vec![
                    Arm {
                        body: quote! {{ #positions }},
                        nothing: false,
                        pattern: self.walker.source.items(&items),
                    },
                    not_the_shape(
                        held,
                        &path_expression(segments),
                        &self.content_expected(content),
                        "not an array",
                    ),
                ]
            }
        }
    }

    /// What an issue names as expected where what a variant holds is missing, or is no array where
    /// the variant holds several values.
    fn content_expected(&self, content: &Shape<'_>) -> TokenStream {
        if let Shape::Held(walk) = content {
            self.walker.expected(walk.ty)
        } else if let Shape::Slots(slots) = content {
            let types = slots.iter().map(|slot| slot.ty);
            let tuple: Type = parse_quote! { (#(#types,)*) };
            self.walker.expected(&tuple)
        } else {
            self.walker.own_model()
        }
    }

    /// The walker of an externally tagged enum: a unit variant's tag as text, or an object whose
    /// one key names a variant over what it holds. A value in any other form is read whole.
    fn external_methods(&self) -> TokenStream {
        let source = self.walker.source;
        let fields = source.method("fields");
        let expected = self.variants_expected();
        let (units, holding): (Vec<&WalkedVariant<'_>>, Vec<&WalkedVariant<'_>>) = self
            .variants
            .iter()
            .partition(|variant| matches!(variant.content, Shape::Nothing));
        let named_alone = if units.is_empty() {
            TokenStream::new()
        } else {
            let (text, tags) = (
                source.text(&Ident::new("tag", Span::call_site())),
                units.iter().flat_map(|variant| variant.tags()),
            );
            quote! { #text if matches!(tag.as_str(), #(#tags)|*) => {} }
        };
        let named_by_key = if holding.is_empty() {
            TokenStream::new()
        } else {
            let (object, tags) = (
                source.entries(&Ident::new("object", Span::call_site())),
                holding.iter().flat_map(|variant| variant.tags()),
            );
            quote! {
                #object if object.len() == 1 && object.keys().any(|key| matches!(key.as_str(), #(#tags)|*)) => {
                    Self::#fields(object, path, issue, out);
                }
            }
        };
        let whole = self.walker.read_whole(&expected);
        let walked = self.walker.issues_method(&quote! {
            match found {
                #named_alone
                #named_by_key
                _ => #whole,
            }
        });
        let looked_up = self
            .variants
            .iter()
            .map(|variant| self.external_variant(variant));
        let keyed = self.walker.fields_method(
            true,
            true,
            &quote! {
                #(#looked_up)*
                out.push(issue("Missing", path.to_vec(), #expected, None, None, Vec::new()));
                Vec::new()
            },
        );
        quote! {
            #walked
            #keyed
        }
    }

    /// What an externally tagged enum's fields walker runs for one variant: where `object` holds
    /// a key naming the variant, what is under it is walked, and that key is the enum's own. A
    /// variant with aliases is found under the first of its tags `object` holds, bound as `tag`.
    fn external_variant(&self, variant: &WalkedVariant<'_>) -> TokenStream {
        let name = &variant.name;
        let aliased = !variant.aliases.is_empty();
        let tags: Vec<&String> = variant.tags().collect();
        let stored = if aliased {
            quote! { tag }
        } else {
            quote! { #name }
        };
        let content = Ident::new("content", Span::call_site());
        let segments = [quote! { Ok(#stored.to_owned()) }];
        let walked = match &variant.content {
            Shape::Fields { declared, fields } => {
                self.held_fields(declared, fields, &content, &segments)
            }
            Shape::Nothing => {
                return if aliased {
                    quote! {
                        if let Some(tag) = [#(#tags),*].into_iter().find(|&tag| object.contains_key(tag)) {
                            return vec![tag];
                        }
                    }
                } else {
                    quote! {
                        if object.contains_key(#name) {
                            return vec![#name];
                        }
                    }
                };
            }
            held @ (Shape::Held(_) | Shape::Slots(_)) => {
                Walker::listed(&content, &self.content_arms(held, &content, &segments))
            }
        };
        let (there, found) = if aliased {
            (
                quote! { Some((tag, content)) },
                quote! {
                    [#(#tags),*]
                        .into_iter()
                        .find_map(|tag| object.get(tag).map(|content| (tag, content)))
                },
            )
        } else {
            (quote! { Some(content) }, quote! { object.get(#name) })
        };
        quote! {
            if let #there = #found {
                #walked
                return vec![#stored];
            }
        }
    }

    /// What a variant's named fields list where `held` holds the object they are written in. Held
    /// in any other form they list nothing, and the read carries serde's refusal alone.
    fn held_fields(
        &self,
        declared: &[String],
        fields: &[WalkedField<'_>],
        held: &Ident,
        segments: &[TokenStream],
    ) -> TokenStream {
        let inner = Ident::new("inner", Span::call_site());
        let object = self.walker.source.entries(&inner);
        let walks = fields
            .iter()
            .map(|field| self.walker.field(field, &inner, segments));
        let undeclared = undeclared_keys(&inner, declared, segments);
        quote! {
            if let #object = #held {
                #(#walks)*
                #undeclared
            }
        }
    }

    /// What `decode_with_{source}_fields` of an internally tagged enum runs: the tag read under
    /// `tag`, then the fields of the variant it names walked in the same object.
    fn internal_fields(&self, tag: &str) -> TokenStream {
        let object = Ident::new("object", Span::call_site());
        let keyed = self.walker.source.method("fields");
        let expected = self.variants_expected();
        let every_key = quote! { object.keys().map(String::as_str).collect() };
        let arms = self.variants.iter().map(|variant| {
            let tags = variant.tags();
            let declared = match &variant.content {
                Shape::Fields { declared, fields } => {
                    let walks = fields
                        .iter()
                        .map(|field| self.walker.field(field, &object, &[]));
                    quote! {{
                        #(#walks)*
                        vec![#tag, #(#declared),*]
                    }}
                }
                Shape::Held(Walk {
                    step: Step::Model,
                    ty,
                }) => quote! {{
                    let mut declared = <#ty>::#keyed(object, path, issue, out);
                    declared.push(#tag);
                    declared
                }},
                Shape::Nothing => quote! { vec![#tag] },
                // serde reads any other value from the whole object, where no walk reaches it.
                Shape::Held(_) | Shape::Slots(_) => every_key.clone(),
            };
            quote! { Some(#(#tags)|*) => #declared, }
        });
        let missing = missing_tag(tag, &expected);
        let unknown = self.unknown_tag(tag, &expected);
        quote! {
            let Some(tag) = object.get(#tag) else {
                #missing
                return #every_key;
            };
            match tag.as_str() {
                #(#arms)*
                _ => {
                    #unknown
                    #every_key
                }
            }
        }
    }

    /// What lists the issues of what a variant holds under `key` of the object the tag is read in.
    fn keyed_content(&self, content: &Shape<'_>, key: &str) -> TokenStream {
        if matches!(content, Shape::Nothing) {
            return quote! { {} };
        }
        let held = Ident::new("content", Span::call_site());
        let lookup = Lookup {
            // serde reads a single optional value as `None` where the key is missing.
            absence_is_read: matches!(
                content,
                Shape::Held(walk) if get_field_def("", walk.ty, "").is_optional()
            ),
            aliases: &[],
            key,
        };
        self.walker.looked_up(
            &lookup,
            &Ident::new("object", Span::call_site()),
            &[],
            &self.content_arms(content, &held, &[lookup.segment()]),
            &self.content_expected(content),
        )
    }

    /// The methods an enum's walker is called through, by the form serde writes the enum in.
    fn methods(&self, tagging: &Tagging) -> TokenStream {
        match tagging {
            Tagging::Adjacent { content, tag } => {
                self.tagged_methods(&self.adjacent_fields(tag, content))
            }
            Tagging::External => self.external_methods(),
            Tagging::Internal { tag } => self.tagged_methods(&self.internal_fields(tag)),
            // A type serde cannot flatten gets no fields walker.
            Tagging::Plain => {
                let whole = self.walker.read_whole(&self.walker.own_model());
                self.walker.issues_method(&quote! { #whole; })
            }
            Tagging::Untagged => self.untagged_methods(),
        }
    }

    /// The walker of an internally or adjacently tagged enum, whose fields walker runs `fields`.
    fn tagged_methods(&self, fields: &TokenStream) -> TokenStream {
        let walked = self.walker.object_issues_method();
        let keyed = self.walker.fields_method(true, true, fields);
        quote! {
            #walked
            #keyed
        }
    }

    /// What lists a tag naming no variant, held under `tag`: `Invalid` where serde refuses the
    /// whole object, and `Mistyped` where it reads it.
    fn unknown_tag(&self, tag: &str, expected: &TokenStream) -> TokenStream {
        let reader = self.walker.source.object_reader();
        quote! {
            let here = [path, &[Ok(#tag.to_owned())]].concat();
            out.push(match <Self as serde::Deserialize>::deserialize(#reader) {
                Err(refused) => issue("Invalid", here, #expected, Some(tag.clone()), Some(refused.to_string()), Vec::new()),
                Ok(_) => issue("Mistyped", here, #expected, Some(tag.clone()), None, Vec::new()),
            });
        }
    }

    /// The walker of an untagged enum: the walk of the variant serde reads the value as, or one
    /// `NoVariant` holding the own list of every variant serde reads where it reads it as none.
    fn untagged_methods(&self) -> TokenStream {
        let variants = self.variants;
        let reader = self.walker.source.found_reader();
        let picked = variants.iter().map(|variant| {
            let (pattern, walked) = (
                &variant.pattern,
                self.variant_call(variant, &quote! { out }),
            );
            quote! { Ok(#pattern) => #walked, }
        });
        // serde gives none of these back, and the match on what it gives stays exhaustive.
        let unread = if self.never_read.is_empty() {
            TokenStream::new()
        } else {
            let patterns = self.never_read.iter().map(|variant| &variant.pattern);
            quote! { Ok(#(#patterns)|*) => {} }
        };
        let lists: Vec<Ident> = variants
            .iter()
            .map(|variant| format_ident!("as_{}", variant.stem))
            .collect();
        let tried = variants.iter().zip(&lists).map(|(variant, list)| {
            let walked = self.variant_call(variant, &quote! { &mut #list });
            quote! {
                let mut #list = Vec::new();
                #walked;
            }
        });
        let names = variants.iter().map(|variant| &variant.rust_name);
        let walked = self.walker.issues_method(&quote! {
            match <Self as serde::Deserialize>::deserialize(#reader) {
                #(#picked)*
                #unread
                Err(_) => {
                    #(#tried)*
                    out.push(issue("NoVariant", path.to_vec(), &[], Some(found.clone()), None, vec![#((#names, #lists)),*]));
                }
            }
        });
        let methods = variants
            .iter()
            .filter_map(|variant| self.variant_method(variant));
        quote! {
            #walked
            #(#methods)*
        }
    }

    /// The call listing into `out` the issues of `found` read as `variant`: the walker of the
    /// model type it holds, or the method the enum carries for it.
    fn variant_call(&self, variant: &WalkedVariant<'_>, out: &TokenStream) -> TokenStream {
        if let Shape::Held(Walk {
            step: Step::Model,
            ty,
        }) = &variant.content
        {
            let issues = self.walker.source.method("issues");
            return quote! { <#ty>::#issues(found, path, issue, #out) };
        }
        let method = self
            .walker
            .source
            .method(&format!("variant_{}", variant.stem));
        quote! { Self::#method(found, path, issue, #out) }
    }

    /// The method listing the issues of `found` read as one variant of an untagged enum, and
    /// `None` for a variant holding one model type, whose own walker is called.
    fn variant_method(&self, variant: &WalkedVariant<'_>) -> Option<TokenStream> {
        let found = Ident::new("found", Span::call_site());
        let body = match &variant.content {
            Shape::Fields { declared, fields } => {
                let object = Ident::new("object", Span::call_site());
                let held_as = self.walker.held_as(&self.walker.source.object_of_found());
                let walks = fields
                    .iter()
                    .map(|field| self.walker.field(field, &object, &[]));
                let undeclared = undeclared_keys(&object, declared, &[]);
                quote! {
                    #held_as
                    #(#walks)*
                    #undeclared
                }
            }
            Shape::Held(Walk {
                step: Step::Model,
                ty: _ty,
            }) => return None,
            Shape::Nothing => {
                let whole = self.walker.read_whole(&self.walker.own_model());
                quote! { #whole; }
            }
            held @ (Shape::Held(_) | Shape::Slots(_)) => {
                Walker::listed(&found, &self.content_arms(held, &found, &[]))
            }
        };
        let method = self
            .walker
            .source
            .method(&format!("variant_{}", variant.stem));
        let signature = self.walker.signature();
        Some(quote! {
            fn #method #signature {
                #body
            }
        })
    }

    /// What an issue names as expected where a tag is: every tag serde reads, variant by variant
    /// in the order declared.
    fn variants_expected(&self) -> TokenStream {
        let tags = self.variants.iter().flat_map(WalkedVariant::tags);
        quote! { &[("Variants", &[#(#tags),*], 0)] }
    }
}

/// The recovering decode of an enum, read off the item as it is emitted, so a read hook tixschema
/// hangs on a variant's field is one the walker reads through.
pub fn enum_recovering_decode(item_enum: &ItemEnum) -> RecoveringDecode {
    let module_name = ident_schema_module_name(&item_enum.ident.to_string());
    let parameters = type_parameters_in_scope(&item_enum.generics);
    let tagging = Tagging::of(item_enum);
    let (never_read, variants): (Vec<WalkedVariant<'_>>, Vec<WalkedVariant<'_>>) =
        walked_variants(item_enum, &module_name, &parameters)
            .into_iter()
            .partition(|variant| variant.never_read);
    added_to(
        &item_enum.ident,
        &item_enum.generics,
        &module_name,
        &parameters,
        &written_names(item_enum.to_token_stream()),
        |walker| {
            EnumWalker {
                never_read: &never_read,
                variants: &variants,
                walker,
            }
            .methods(&tagging)
        },
    )
}

/// What lists a tag that is not there, under the key `tag` it is read from.
fn missing_tag(tag: &str, expected: &TokenStream) -> TokenStream {
    quote! {
        out.push(issue("Missing", [path, &[Ok(#tag.to_owned())]].concat(), #expected, None, None, Vec::new()));
    }
}

/// Every variant of `item_enum` as its walker reads it, in the order declared.
fn walked_variants<'item>(
    item_enum: &'item ItemEnum,
    module_name: &str,
    parameters: &[String],
) -> Vec<WalkedVariant<'item>> {
    let container = parse_serde_type_attributes(&item_enum.attrs);
    item_enum
        .variants
        .iter()
        .map(|variant| {
            let ident = &variant.ident;
            let rust_name = ident.unraw().to_string();
            let meta = parse_serde_field_attributes(&variant.attrs);
            let name = meta.rename.unwrap_or_else(|| {
                resolve_rename_rule(container.rename_all.as_deref()).apply_to_variant(&rust_name)
            });
            let pattern = match &variant.fields {
                Fields::Named(_) => quote! { Self::#ident { .. } },
                Fields::Unit => quote! { Self::#ident },
                Fields::Unnamed(_) => quote! { Self::#ident(..) },
            };
            let stem = to_snake_case(&rust_name);
            WalkedVariant {
                aliases: meta.aliases,
                content: Shape::of_variant(
                    variant,
                    container.rename_all_fields.as_deref(),
                    module_name,
                    parameters,
                ),
                name,
                never_read: parse_serde_key_omission(&variant.attrs).skips_deserializing,
                pattern,
                rust_name,
                stem,
            }
        })
        .collect()
}
