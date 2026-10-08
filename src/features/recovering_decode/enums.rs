//! The walkers of an enum, one per form serde writes an enum in.
//!
//! A plain enum is one value, and the key naming its variant where serde flattens it. A tagged
//! enum is walked by its tag: the tag is read first, and what the variant it names holds is walked
//! where that form writes it. An untagged enum is walked as the variant serde reads the value as.

use core::iter::once;

use proc_macro2::{Ident, Span, TokenStream};
use quote::{ToTokens as _, format_ident, quote};
use syn::ext::IdentExt as _;
use syn::{Fields, ItemEnum, Type, parse_quote};

use super::aliases::Reach;
use super::{
    Arm, Handed, Keyed, Lookup, RecoveringDecode, Shape, Step, TypedPaths, Walk, Walker, Written,
    added_to, binding, flattened_walker_call, not_the_shape, path_expression, written_names,
};
use crate::features::serde::{
    parse_serde_field_attributes, parse_serde_key_omission, parse_serde_type_attributes,
};
use crate::field_type::{get_field_def, is_plain_enum};
use crate::rename_rule::resolve_rename_rule;
use crate::utils::{ident_schema_module_name, to_snake_case, type_parameters_in_scope};

/// The form serde writes an enum in, read off its attributes as the schema surfaces read them.
pub enum Tagging {
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
    pub fn of(item_enum: &ItemEnum) -> Self {
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
            quote! { ::core::option::Option::Some(#(#tags)|*) => #walked, }
        });
        let missing = missing_tag(tag, &expected);
        let unknown = self.unknown_tag(tag, &expected);
        quote! {
            let ::core::option::Option::Some(tag) = object.get(#tag) else {
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
            Shape::Fields(keyed) => vec![Arm {
                body: self.held_fields(keyed, held, segments),
                nothing: false,
                pattern: quote! { #held },
            }],
            Shape::Held(walk) => self.walker.arms(walk, held, segments, 0, walk.ty, None),
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

    /// What an enum serde writes under its variant's name answers of an object and walks in it:
    /// whether a key there names a variant, and what the first such key holds, the key being the
    /// enum's own. A plain enum is flattened in that form.
    fn external_keyed(&self) -> TokenStream {
        let expected = self.variants_expected();
        let looked_up = self
            .variants
            .iter()
            .map(|variant| self.external_variant(variant));
        let named = self.external_named();
        let keyed = self.walker.fields_method(
            true,
            true,
            &quote! {
                #(#looked_up)*
                out.push(issue("Missing", path.to_vec(), #expected, ::core::option::Option::None, ::core::option::Option::None, ::std::vec::Vec::new()));
                ::std::vec::Vec::new()
            },
        );
        quote! {
            #named
            #keyed
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
        let keyed = self.external_keyed();
        quote! {
            #walked
            #keyed
        }
    }

    /// `decode_with_{source}_named` of an externally tagged enum: whether `object` holds a key
    /// naming a variant serde reads.
    fn external_named(&self) -> TokenStream {
        let tags: Vec<&String> = self.variants.iter().flat_map(WalkedVariant::tags).collect();
        if tags.is_empty() {
            return self.walker.never_named();
        }
        self.walker.named_method(
            true,
            &quote! { object.keys().any(|key| matches!(key.as_str(), #(#tags)|*)) },
        )
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
        let segments = [quote! { ::core::result::Result::Ok(#stored.to_owned()) }];
        let walked = match &variant.content {
            Shape::Fields(keyed) => self.held_fields(keyed, &content, &segments),
            Shape::Nothing => {
                return if aliased {
                    quote! {
                        if let ::core::option::Option::Some(tag) = [#(#tags),*].into_iter().find(|&tag| object.contains_key(tag)) {
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
                quote! { ::core::option::Option::Some((tag, content)) },
                quote! {
                    [#(#tags),*]
                        .into_iter()
                        .find_map(|tag| object.get(tag).map(|content| (tag, content)))
                },
            )
        } else {
            (
                quote! { ::core::option::Option::Some(content) },
                quote! { object.get(#name) },
            )
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
        keyed: &Keyed<'_>,
        held: &Ident,
        segments: &[TokenStream],
    ) -> TokenStream {
        let inner = Ident::new("inner", Span::call_site());
        let object = self.walker.source.entries(&inner);
        let walked = self
            .walker
            .keyed(keyed, None, &inner, segments, true)
            .checked(&inner, segments);
        quote! {
            if let #object = #held {
                #walked
            }
        }
    }

    /// What `decode_with_{source}_fields` of an internally tagged enum runs: the tag read under
    /// `tag`, then the fields of the variant it names walked in the same object.
    fn internal_fields(&self, tag: &str) -> TokenStream {
        let source = self.walker.source;
        let object = Ident::new("object", Span::call_site());
        let expected = self.variants_expected();
        let every_key = quote! { object.keys().map(::std::string::String::as_str).collect() };
        let arms = self.variants.iter().map(|variant| {
            let tags = variant.tags();
            let declared = match &variant.content {
                Shape::Fields(keyed_fields) => {
                    let walked = self
                        .walker
                        .keyed(keyed_fields, Some(tag), &object, &[], true)
                        .returning(&object);
                    quote! {{ #walked }}
                }
                // serde reads the tag first, and hands the type the object without its entry.
                Shape::Held(Walk {
                    step: Step::Model,
                    ty,
                }) => {
                    let (handed, copied) = Handed::leaving(source, &object, &[tag.to_owned()]);
                    let walked =
                        flattened_walker_call(source, ty, &handed.argument(), &[], &quote! { out });
                    let keys = handed.of_the_object(&walked);
                    quote! {{
                        #copied
                        let mut declared: ::std::vec::Vec<&str> = #keys.collect();
                        declared.push(#tag);
                        declared
                    }}
                }
                Shape::Nothing => quote! { vec![#tag] },
                // serde reads any other value from the whole object, where no walk reaches it.
                Shape::Held(_) | Shape::Slots(_) => every_key.clone(),
            };
            quote! { ::core::option::Option::Some(#(#tags)|*) => #declared, }
        });
        let missing = missing_tag(tag, &expected);
        let unknown = self.unknown_tag(tag, &expected);
        quote! {
            let ::core::option::Option::Some(tag) = object.get(#tag) else {
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
                self.tagged_methods(tag, &self.adjacent_fields(tag, content))
            }
            Tagging::External => self.external_methods(),
            Tagging::Internal { tag } => self.tagged_methods(tag, &self.internal_fields(tag)),
            Tagging::Plain => {
                let whole = self.walker.read_whole(&self.walker.own_model());
                let walked = self.walker.issues_method(&quote! { #whole; });
                // With no variant serde reads, no key is the enum's own.
                if self.variants.is_empty() {
                    return self.walker.claiming_no_key(&walked);
                }
                let keyed = self.external_keyed();
                quote! {
                    #walked
                    #keyed
                }
            }
            Tagging::Untagged => self.untagged_methods(),
        }
    }

    /// The walker of an internally or adjacently tagged enum, which the key `tag` names in an
    /// object and whose fields walker runs `fields`.
    fn tagged_methods(&self, tag: &str, fields: &TokenStream) -> TokenStream {
        let walked = self.walker.object_issues_method();
        let named = self
            .walker
            .named_method(true, &quote! { object.contains_key(#tag) });
        let keyed = self.walker.fields_method(true, true, fields);
        quote! {
            #walked
            #named
            #keyed
        }
    }

    /// What lists a tag naming no variant, held under `tag`: `Invalid` where serde refuses the
    /// whole object, and `Mistyped` where it reads it.
    fn unknown_tag(&self, tag: &str, expected: &TokenStream) -> TokenStream {
        let reader = self
            .walker
            .source
            .object_reader(&Ident::new("object", Span::call_site()));
        quote! {
            let here = [path, &[::core::result::Result::Ok(#tag.to_owned())]].concat();
            out.push(match <Self as serde::Deserialize>::deserialize(#reader) {
                ::core::result::Result::Err(refused) => issue("Invalid", here, #expected, ::core::option::Option::Some(tag.clone()), ::core::option::Option::Some(refused.to_string()), ::std::vec::Vec::new()),
                ::core::result::Result::Ok(_) => issue("Mistyped", here, #expected, ::core::option::Option::Some(tag.clone()), ::core::option::Option::None, ::std::vec::Vec::new()),
            });
        }
    }

    /// What `decode_with_{source}_fields` of an untagged enum runs: the fields of the variant serde
    /// reads `object` as walked in it, or one `NoVariant` where serde reads it as none, with every
    /// key then the enum's own.
    fn untagged_fields(&self) -> TokenStream {
        let source = self.walker.source;
        let object = Ident::new("object", Span::call_site());
        let reader = source.object_reader(&object);
        let every_key = quote! { object.keys().map(::std::string::String::as_str).collect() };
        let out = quote! { out };
        let picked = self.variants.iter().map(|variant| {
            let pattern = &variant.pattern;
            let declared = match &variant.content {
                Shape::Fields(keyed) => {
                    let walked = self
                        .walker
                        .keyed(keyed, None, &object, &[], true)
                        .returning(&object);
                    quote! {{ #walked }}
                }
                Shape::Held(Walk {
                    step: Step::Model,
                    ty,
                }) => flattened_walker_call(source, ty, &object.to_token_stream(), &[], &out),
                // serde reads any other value from the whole object, where no walk reaches it.
                Shape::Held(_) | Shape::Nothing | Shape::Slots(_) => every_key.clone(),
            };
            quote! { ::core::result::Result::Ok(#pattern) => #declared, }
        });
        // serde gives none of these back, and the match on what it gives stays exhaustive.
        let unread = if self.never_read.is_empty() {
            TokenStream::new()
        } else {
            let patterns = self.never_read.iter().map(|variant| &variant.pattern);
            quote! { ::core::result::Result::Ok(#(#patterns)|*) => ::std::vec::Vec::new(), }
        };
        let lists: Vec<Ident> = self
            .variants
            .iter()
            .map(|variant| format_ident!("as_{}", variant.stem))
            .collect();
        let tried = self.variants.iter().zip(&lists).map(|(variant, list)| {
            let into = quote! { &mut #list };
            let walked = match &variant.content {
                // What the object's other keys hold is none of the variant's to list.
                Shape::Fields(keyed) => {
                    let walked = self.walker.keyed(keyed, None, &object, &[], false);
                    if !walked.lists {
                        return quote! { let #list = ::std::vec::Vec::new(); };
                    }
                    let walk = walked.walk;
                    quote! {{
                        let out = #into;
                        #walk
                    }}
                }
                Shape::Held(Walk {
                    step: Step::Model,
                    ty,
                }) => {
                    let walked =
                        flattened_walker_call(source, ty, &object.to_token_stream(), &[], &into);
                    quote! { #walked; }
                }
                Shape::Held(_) | Shape::Nothing | Shape::Slots(_) => {
                    let walked = self.variant_call(variant, &into);
                    quote! { #walked; }
                }
            };
            quote! {
                let mut #list = ::std::vec::Vec::new();
                #walked
            }
        });
        let names = self.variants.iter().map(|variant| &variant.rust_name);
        let whole = source.object_value(&object);
        quote! {
            match <Self as serde::Deserialize>::deserialize(#reader) {
                #(#picked)*
                #unread
                ::core::result::Result::Err(_) => {
                    let found = &#whole;
                    #(#tried)*
                    out.push(issue("NoVariant", path.to_vec(), &[], ::core::option::Option::Some(found.clone()), ::core::option::Option::None, vec![#((#names, #lists)),*]));
                    #every_key
                }
            }
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
            quote! { ::core::result::Result::Ok(#pattern) => #walked, }
        });
        // serde gives none of these back, and the match on what it gives stays exhaustive.
        let unread = if self.never_read.is_empty() {
            TokenStream::new()
        } else {
            let patterns = self.never_read.iter().map(|variant| &variant.pattern);
            quote! { ::core::result::Result::Ok(#(#patterns)|*) => {} }
        };
        let lists: Vec<Ident> = variants
            .iter()
            .map(|variant| format_ident!("as_{}", variant.stem))
            .collect();
        let tried = variants.iter().zip(&lists).map(|(variant, list)| {
            let walked = self.variant_call(variant, &quote! { &mut #list });
            quote! {
                let mut #list = ::std::vec::Vec::new();
                #walked;
            }
        });
        let names = variants.iter().map(|variant| &variant.rust_name);
        let walked = self.walker.issues_method(&quote! {
            match <Self as serde::Deserialize>::deserialize(#reader) {
                #(#picked)*
                #unread
                ::core::result::Result::Err(_) => {
                    #(#tried)*
                    out.push(issue("NoVariant", path.to_vec(), &[], ::core::option::Option::Some(found.clone()), ::core::option::Option::None, vec![#((#names, #lists)),*]));
                }
            }
        });
        // Nothing in an object names an untagged enum but serde reading it as one of its variants.
        let read = self
            .walker
            .source
            .object_reader(&Ident::new("object", Span::call_site()));
        let asked = self.walker.named_method(
            true,
            &quote! { <Self as serde::Deserialize>::deserialize(#read).is_ok() },
        );
        let keyed = self
            .walker
            .fields_method(true, true, &self.untagged_fields());
        let methods = variants
            .iter()
            .filter_map(|variant| self.variant_method(variant));
        quote! {
            #walked
            #asked
            #keyed
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
            return self
                .walker
                .asked(ty, &quote! { <#ty>::#issues(found, path, issue, #out) });
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
            Shape::Fields(keyed) => {
                let object = Ident::new("object", Span::call_site());
                let held_as = self.walker.held_as(&self.walker.source.object_of_found());
                let walked = self
                    .walker
                    .keyed(keyed, None, &object, &[], true)
                    .checked(&object, &[]);
                quote! {
                    #held_as
                    #walked
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
    let mut reach = Reach::new(&module_name);
    let mut walked = item_enum.clone();
    for variant in &mut walked.variants {
        reach.fields(&mut variant.fields);
    }
    let tagging = Tagging::of(&walked);
    let (never_read, variants): (Vec<WalkedVariant<'_>>, Vec<WalkedVariant<'_>>) =
        walked_variants(&walked, &module_name, &parameters)
            .into_iter()
            .partition(|variant| variant.never_read);
    let names = written_names(item_enum.to_token_stream());
    let paths = enum_paths(item_enum, &names);
    added_to(
        &item_enum.ident,
        &item_enum.generics,
        &module_name,
        &parameters,
        &Written {
            names,
            reach: &reach,
        },
        &paths,
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

/// The typed MongoDB paths of an enum: `MongoFields` for its module, and for its `impl` the const
/// and the function that build it.
#[cfg(feature = "mongodb")]
fn enum_paths(item_enum: &ItemEnum, written: &[String]) -> TypedPaths {
    super::fields::enum_paths(item_enum, written)
}

/// A build without `mongodb` writes no typed path.
#[cfg(not(feature = "mongodb"))]
fn enum_paths(_item_enum: &ItemEnum, _written: &[String]) -> TypedPaths {
    TypedPaths::default()
}

/// What lists a tag that is not there, under the key `tag` it is read from.
fn missing_tag(tag: &str, expected: &TokenStream) -> TokenStream {
    quote! {
        out.push(issue("Missing", [path, &[::core::result::Result::Ok(#tag.to_owned())]].concat(), #expected, ::core::option::Option::None, ::core::option::Option::None, ::std::vec::Vec::new()));
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
