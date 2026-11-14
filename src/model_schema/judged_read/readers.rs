//! The read of a value behind a type parameter by a reader the holder hands in. A type that
//! declares type parameters gains `model_schema_read_unjudged_by{n}`, its unjudged read with
//! every value behind its parameters read by one `DeserializeSeed` per parameter, and a holder
//! that knows what fills each parameter hands in the unjudged read of what fills it. The holder
//! then judges the whole value once, through `model_schema_issues_with{n}`.

use proc_macro2::{Ident, Span, TokenStream};
use quote::{ToTokens as _, format_ident, quote};
use syn::{Attribute, DeriveInput, Field, Fields};

use super::{
    Corrects, HELD, HungOn, REMOTE, Reached, UNJUDGED_READ, derives_deserialize, generated,
    hook_fallbacks, hooked, kept_by_the_copy, names_a_read_hook, naming_the_type, told_by,
    walked_read, words_written_as,
};
use crate::features::serde::read_bounds::ReadBounds;
use crate::features::serde::{
    consume_unread_value, derive_reads_a_default, has_serde_read_hook, parse_serde_key_omission,
    serde_conversion,
};
use crate::field_type::get_field_def;
use crate::model_schema::parameter_walk::{
    self, REARRANGED_LABEL, REARRANGED_NOTE, leaf_arguments, rearranged,
};
use crate::model_schema::valid_type::{relabelled, renames_the_container};
use crate::model_schema::{HeldEnd, HeldShape, held_shape_reaching, tokens_name_any};
use crate::utils::{unclaimed_parameter, written_names};

/// The alias of the item a copy named after the item reads the item's own type through.
const OWN: &str = "ModelSchemaOwn";

/// What a read written for a scope has declared so far: the names it took, and the arity of each
/// generic model it reads through `model_schema_read_unjudged_by{n}`.
pub(super) struct Declaring {
    pub arities: Vec<usize>,
    pub taken: Vec<String>,
}

/// One member of the item, where its read hands a reader on: its place and how it is read.
struct Handed {
    /// Whether serde builds a missing value of the member with `Default`.
    defaulted: bool,
    index: usize,
    reached: Reached,
    variant: Option<String>,
}

/// The copy of an item the read with readers derives serde's read on.
struct HandedCopy<'parts> {
    copied_as: &'parts Ident,
    derive: &'parts syn::Path,
    /// The hook hung on each member read as the item's own read reads it.
    hung: &'parts [HungOn],
    /// `Self`, as the copy names it.
    named: &'parts TokenStream,
    predicates: &'parts [TokenStream],
    /// The type each member read with readers is read as, by place.
    read_as: &'parts [(Option<String>, usize, TokenStream)],
    reader_bounds: &'parts [TokenStream],
    readers: &'parts [(String, Ident)],
    /// Whether the copy is renamed to the item's name, where serde would name it by its own.
    renamed: bool,
}

/// What every function of an item's read by readers is written with.
struct ItemReaders {
    /// `Self` as the read names it.
    named: TokenStream,
    /// The bounds of the item's own read.
    predicates: Vec<TokenStream>,
    /// The name of the deserializer's type.
    reader: Ident,
    /// The bound each reader answers.
    reader_bounds: Vec<TokenStream>,
    /// Each type parameter's reader, in the order the item declares them.
    readers: Vec<(String, Ident)>,
    /// The names the item writes, and those taken beside them.
    taken: Vec<String>,
    written: Vec<String>,
}

/// What a read written beside a holder reads with: the parameters every type it declares carries,
/// the reader handed in for each parameter where there is one, and the bounds the impls of those
/// types are written under.
pub(super) struct ReadScope {
    /// The parameters each type the read declares is generic over, the readers among them.
    pub carried: Vec<Ident>,
    /// The bounds of the struct of each type the read declares.
    pub declared: Vec<TokenStream>,
    /// `Self` as the read names it.
    pub named: TokenStream,
    /// The bounds of the impl of each type the read declares.
    pub predicates: Vec<TokenStream>,
    /// The item's own type parameters a walk ends on: those a reader is handed in for.
    pub reach: Vec<String>,
    /// The name of the deserializer's type in each function the read declares.
    pub reader: Ident,
    /// The reader handed in for each of `reach`.
    pub readers: Vec<(String, Ident)>,
}

impl Declaring {
    /// A name for a type the read declares, none it took before.
    fn named(&mut self, stem: &str) -> Ident {
        let named = unclaimed_parameter(stem, &self.taken);
        self.taken.push(named.to_string());
        named
    }

    pub const fn of(taken: Vec<String>) -> Self {
        Self {
            arities: Vec::new(),
            taken,
        }
    }
}

impl HandedCopy<'_> {
    /// The members of one variant of the copy, or of the struct.
    fn members(&self, fields: &mut Fields, variant: Option<&str>) {
        for (index, field) in fields.iter_mut().enumerate() {
            field.attrs = kept_by_the_copy(&field.attrs);
            field.vis = syn::Visibility::Inherited;
            if let Some((_, _, written)) = self
                .read_as
                .iter()
                .find(|(at_variant, at, _)| *at == index && at_variant.as_deref() == variant)
                && let Ok(ty) = syn::parse2::<syn::Type>(written.clone())
            {
                field.ty = ty;
                continue;
            }
            let hooked_on = self
                .hung
                .iter()
                .find(|hook| hook.index == index && hook.variant.as_deref() == variant);
            if hooked_on.is_some_and(|hook| hook.replaces) {
                field.attrs.retain(|attr| !names_a_read_hook(attr));
            }
            field
                .attrs
                .extend(hooked_on.into_iter().flat_map(|hook| hook.attrs.clone()));
        }
    }

    /// The copy of `input`, named `copied_as`: generic over the item's parameters and their
    /// readers as well, each member `read_as` names written as the type it is read as there, each
    /// other member with the hook `hung` on it.
    fn written(&self, input: &DeriveInput) -> TokenStream {
        let mut copy = input.clone();
        copy.ident = self.copied_as.clone();
        copy.vis = syn::Visibility::Inherited;
        copy.attrs = kept_by_the_copy(&input.attrs);
        // The readers follow the item's own parameters, which a default may not precede.
        for param in copy.generics.type_params_mut() {
            param.default = None;
        }
        for (_, read) in self.readers {
            copy.generics.params.push(syn::parse_quote! { #read });
        }
        match &mut copy.data {
            syn::Data::Struct(held) => self.members(&mut held.fields, None),
            syn::Data::Enum(held) => {
                for variant in &mut held.variants {
                    variant.attrs = kept_by_the_copy(&variant.attrs);
                    variant.discriminant = None;
                    let named_as = variant.ident.to_string();
                    self.members(&mut variant.fields, Some(&named_as));
                }
            }
            syn::Data::Union(_) => {}
        }
        let bound = {
            let all = self.reader_bounds.iter().chain(self.predicates);
            syn::LitStr::new(&quote! { #(#all),* }.to_string(), Span::call_site())
        };
        let declared = naming_the_type(copy.to_token_stream(), self.named);
        let derive = self.derive;
        let named_as = (self.renamed && !renames_the_container(&input.attrs)).then(|| {
            let written = syn::LitStr::new(&input.ident.to_string(), Span::call_site());
            quote! { #[serde(rename = #written)] }
        });
        generated(quote! {
            #[derive(#derive)]
            #[serde(bound(deserialize = #bound))]
            #named_as
            #declared
        })
    }
}

impl ItemReaders {
    /// The bounds the item and its parameters are declared under, `Self` written out.
    fn declared(&self, input: &DeriveInput) -> Vec<TokenStream> {
        let mut declared: Vec<TokenStream> = input
            .generics
            .type_params()
            .flat_map(|param| {
                let ident = &param.ident;
                param
                    .bounds
                    .iter()
                    .map(move |bound| quote! { #ident: #bound })
            })
            .collect();
        declared.extend(
            input
                .generics
                .where_clause
                .iter()
                .flat_map(|clause| &clause.predicates)
                .map(|predicate| naming_the_type(predicate.to_token_stream(), &self.named)),
        );
        declared
    }

    /// What the read of `input` names: a reader for each of its type parameters, beside the
    /// names it writes, and the bounds of its own read.
    fn of(input: &DeriveInput, bounds: &ReadBounds) -> Self {
        let name = &input.ident;
        let (_, type_generics, _) = input.generics.split_for_impl();
        let named = quote! { #name #type_generics };
        let written = written_names(input.to_token_stream());
        let reader = unclaimed_parameter("D", &written);
        let mut taken = written.clone();
        taken.push(reader.to_string());
        let mut readers = Vec::new();
        for parameter in input.generics.type_params() {
            let read = unclaimed_parameter(&format!("{}Read", parameter.ident), &taken);
            taken.push(read.to_string());
            readers.push((parameter.ident.to_string(), read));
        }
        let reader_bounds = readers
            .iter()
            .map(|(parameter, read)| {
                let held = Ident::new(parameter, Span::call_site());
                quote! { #read: serde::de::DeserializeSeed<'de, Value = #held> + ::core::default::Default }
            })
            .collect();
        let predicates = bounds
            .predicates
            .iter()
            .map(|predicate| naming_the_type(predicate.to_token_stream(), &named))
            .collect();
        Self {
            named,
            predicates,
            reader,
            reader_bounds,
            readers,
            taken,
            written,
        }
    }

    /// The scope the members of `input` are read with readers in.
    fn scope(&self, input: &DeriveInput) -> ReadScope {
        let declared = self.declared(input);
        let mut predicates = declared.clone();
        predicates.extend(self.predicates.iter().cloned());
        let mut carried: Vec<Ident> = input
            .generics
            .type_params()
            .map(|param| param.ident.clone())
            .collect();
        carried.extend(self.readers.iter().map(|(_, read)| read.clone()));
        ReadScope {
            carried,
            declared,
            named: self.named.clone(),
            predicates,
            reach: self
                .readers
                .iter()
                .map(|(parameter, _)| parameter.clone())
                .collect(),
            reader: self.reader.clone(),
            readers: self.readers.clone(),
        }
    }

    /// The signature of `model_schema_read_unjudged_by{n}`.
    fn signature(&self) -> TokenStream {
        let (reader, reader_bounds, predicates) =
            (&self.reader, &self.reader_bounds, &self.predicates);
        let reader_idents = self.readers.iter().map(|(_, read)| read);
        let by = read_by(self.readers.len());
        quote! {
            /// The read alone, with a reader handed in for each type parameter, for a holder that
            /// knows what fills each and judges the whole value once.
            pub fn #by<'de, #reader #(, #reader_idents)*>(deserializer: #reader) -> ::core::result::Result<Self, #reader::Error>
            where
                #reader: serde::Deserializer<'de>,
                #(#reader_bounds,)*
                #(#predicates,)*
        }
    }
}

impl ReadScope {
    /// `<P1, R1, ..>`, the parameters each declared type carries, or nothing for none.
    fn generics(&self) -> TokenStream {
        let carried = &self.carried;
        if carried.is_empty() {
            TokenStream::new()
        } else {
            quote! { <#(#carried),*> }
        }
    }

    /// The read of the generic model `leaf` through `model_schema_read_unjudged_by{n}`, handed a
    /// reader of each of `arguments`, each declared here.
    pub fn handed_read(
        &self,
        leaf: &TokenStream,
        arguments: &[syn::Type],
        declaring: &mut Declaring,
    ) -> TokenStream {
        declaring.arities.push(arguments.len());
        let value = declaring.named("Value");
        let mut items = TokenStream::new();
        let mut handed = Vec::new();
        for argument in arguments {
            // The reader of a parameter is handed on as it is: a reader built around it would make
            // each level of a recursive model read through a type larger than the one above.
            if let Some(handed_on) = self.reader_of(argument) {
                handed.push(quote! { #handed_on });
                continue;
            }
            let read_named = declaring.named("Read");
            let read = self.read_of(argument, declaring);
            items.extend(self.seed(&read_named, &value, argument, &read));
            let carried = &self.carried;
            handed.push(quote! { #read_named<_ #(, #carried)*> });
        }
        let (by, reader) = (read_by(arguments.len()), &self.reader);
        quote! {
            {
                #items
                <#leaf>::#by::<#reader, #(#handed),*>(deserializer)
            }
        }
    }

    /// The type `held_as` a value of `leaf` is read as, by `read`, its `Deserialize`, and its
    /// `Default` where it is `defaulted`.
    fn held_type(
        &self,
        held_as: &Ident,
        leaf: &syn::Type,
        read: &TokenStream,
        defaulted: bool,
    ) -> TokenStream {
        let (generics, phantom, reader) = (self.generics(), self.phantom(), &self.reader);
        let carried = &self.carried;
        let written = naming_the_type(leaf.to_token_stream(), &self.named);
        let declared = &self.declared;
        let declared_where = (!declared.is_empty()).then(|| quote! { where #(#declared),* });
        let mut predicates = self.predicates.clone();
        predicates.extend(self.reader_bounds());
        let read_where = (!predicates.is_empty()).then(|| quote! { where #(#predicates),* });
        let default = defaulted.then(|| {
            let mut defaults = declared.clone();
            defaults.push(quote! { #written: ::core::default::Default });
            quote! {
                impl #generics ::core::default::Default for #held_as #generics where #(#defaults),* {
                    fn default() -> Self {
                        #held_as(::core::default::Default::default(), ::core::marker::PhantomData)
                    }
                }
            }
        });
        quote! {
            struct #held_as #generics (#written, #phantom) #declared_where;
            impl<'de #(, #carried)*> serde::Deserialize<'de> for #held_as #generics #read_where {
                fn deserialize<#reader>(deserializer: #reader) -> ::core::result::Result<Self, #reader::Error>
                where
                    #reader: serde::Deserializer<'de>,
                {
                    (#read).map(|held| #held_as(held, ::core::marker::PhantomData))
                }
            }
            #default
        }
    }

    /// The types `reached` reads its models and its values behind a parameter as, each declared
    /// here, and its type with each written where it holds them. A member serde builds a missing
    /// value of with `Default`, where it is `defaulted`, has one written for the type it is read as
    /// where that type is the member's whole.
    pub fn held_types(
        &self,
        reached: &Reached,
        defaulted: bool,
        declaring: &mut Declaring,
    ) -> (TokenStream, TokenStream) {
        let generics = self.generics();
        let mut items = TokenStream::new();
        let mut renamed = Vec::new();
        let whole = matches!(reached.leaves.as_slice(), [(_, held_as)]
            if reached.read_as.to_token_stream().to_string() == *held_as.to_string());
        for (leaf, held_as) in &reached.leaves {
            let read = self.leaf_read(leaf, declaring);
            items.extend(self.held_type(held_as, leaf, &read, defaulted && whole));
            renamed.push((held_as.to_string(), quote! { #held_as #generics }));
        }
        let read_as = naming_the_type(
            words_written_as(reached.read_as.to_token_stream(), &renamed),
            &self.named,
        );
        (items, read_as)
    }

    /// The read of one value a walk ends on, `leaf`: through the reader handed in for it, where it
    /// is behind a parameter; through `model_schema_read_unjudged_by{n}`, handed a reader of each
    /// type argument, where it is a generic model; and through its unjudged read otherwise.
    fn leaf_read(&self, leaf: &syn::Type, declaring: &mut Declaring) -> TokenStream {
        if let Some(handed_in) = self.reader_of(leaf) {
            return seeded(handed_in);
        }
        let named = naming_the_type(leaf.to_token_stream(), &self.named);
        let arguments = syn::parse2::<syn::Type>(named.clone())
            .ok()
            .and_then(|written| {
                if let syn::Type::Path(path) = written {
                    path.path.segments.last().map(leaf_arguments)
                } else {
                    None
                }
            })
            .unwrap_or_default();
        if arguments.is_empty() {
            return quote! { <#named>::model_schema_read_unjudged(deserializer) };
        }
        self.handed_read(&named, &arguments, declaring)
    }

    /// What holds the parameters a declared type carries, which its own fields need not name.
    fn phantom(&self) -> TokenStream {
        let carried = &self.carried;
        quote! { ::core::marker::PhantomData<fn() -> (#(#carried,)*)> }
    }

    /// The expression that reads a `ty` from `deserializer` with every model and every value
    /// behind a parameter it holds read unjudged, beside the types it declares to read them.
    pub fn read_of(&self, ty: &syn::Type, declaring: &mut Declaring) -> TokenStream {
        if let Some(handed_in) = self.reader_of(ty) {
            return seeded(handed_in);
        }
        let fallback = quote! { <#ty>::model_schema_read_unjudged(deserializer) };
        let Some(shape) = self.shape_of(ty) else {
            return fallback;
        };
        if shape.wraps.is_empty()
            && matches!(shape.end, HeldEnd::Declared(_) | HeldEnd::Parameter(_))
        {
            return self.leaf_read(ty, declaring);
        }
        let Some(reached) = walked_read(ty, 0, (&shape.wraps, &shape.end), &mut declaring.taken)
        else {
            return fallback;
        };
        let (items, read_as) = self.held_types(&reached, false, declaring);
        let back = &reached.back;
        quote! {
            {
                #items
                <#read_as as serde::Deserialize>::deserialize(deserializer).map(|held_0| #back)
            }
        }
    }

    /// The bound each handed-in reader answers.
    fn reader_bounds(&self) -> Vec<TokenStream> {
        self.readers
            .iter()
            .map(|(parameter, read)| {
                let held = Ident::new(parameter, Span::call_site());
                quote! {
                    #read: serde::de::DeserializeSeed<'de, Value = #held> + ::core::default::Default
                }
            })
            .collect()
    }

    /// The reader handed in for `ty`, where it is one of the item's parameters that has one.
    fn reader_of(&self, ty: &syn::Type) -> Option<&Ident> {
        let syn::Type::Path(path) = ty else {
            return None;
        };
        let only = path.path.get_ident()?.to_string();
        self.readers
            .iter()
            .find(|(parameter, _)| *parameter == only)
            .map(|(_, read)| read)
    }

    /// The reader `read_named` of the type argument `argument`, which reads it with `read` and
    /// answers the parameter's own type, `value`, refused where the two differ.
    fn seed(
        &self,
        read_named: &Ident,
        value: &Ident,
        argument: &syn::Type,
        read: &TokenStream,
    ) -> TokenStream {
        let (carried, reader, phantom) = (&self.carried, &self.reader, self.phantom());
        let written = naming_the_type(argument.to_token_stream(), &self.named);
        let mut predicates = self.predicates.clone();
        predicates.extend(self.reader_bounds());
        quote! {
            struct #read_named<#value #(, #carried)*>(::core::marker::PhantomData<fn() -> (#value, #phantom)>);
            impl<#value #(, #carried)*> ::core::default::Default for #read_named<#value #(, #carried)*> {
                fn default() -> Self {
                    Self(::core::marker::PhantomData)
                }
            }
            impl<'de, #value #(, #carried)*> serde::de::DeserializeSeed<'de> for #read_named<#value #(, #carried)*>
            where
                #written: ModelSchemaReadFor<#value>,
                #(#predicates,)*
            {
                type Value = #value;

                fn deserialize<#reader>(self, deserializer: #reader) -> ::core::result::Result<#value, #reader::Error>
                where
                    #reader: serde::Deserializer<'de>,
                {
                    (#read).map(<#written as ModelSchemaReadFor<#value>>::model_schema_read_for)
                }
            }
        }
    }

    /// What `ty` holds as the walks of `validate()` read it: the item's parameters a reader is
    /// handed in for among what they end on.
    fn shape_of(&self, ty: &syn::Type) -> Option<HeldShape> {
        let walked = parameter_walk::scope();
        let mut def = get_field_def("_held", ty, "");
        def.resolve_self_references(&walked.type_name, &walked.parameters);
        def.erase_type_parameters(&walked.parameters);
        held_shape_reaching(ty, &def, &self.reach)
    }
}

/// Whether `tokens` write `ident`, at any depth.
fn writes(tokens: &TokenStream, ident: &Ident) -> bool {
    tokens.clone().into_iter().any(|tree| match tree {
        proc_macro2::TokenTree::Ident(written) => written == *ident,
        proc_macro2::TokenTree::Group(group) => writes(&group.stream(), ident),
        proc_macro2::TokenTree::Punct(_) | proc_macro2::TokenTree::Literal(_) => false,
    })
}

/// The read of a value by `reader`, a reader handed in, from `deserializer`.
fn seeded(reader: &Ident) -> TokenStream {
    quote! {
        serde::de::DeserializeSeed::deserialize(<#reader as ::core::default::Default>::default(), deserializer)
    }
}

/// The read that takes one reader per type parameter, on a type of `arity` of them.
pub fn read_by(arity: usize) -> Ident {
    format_ident!("model_schema_read_unjudged_by{arity}")
}

/// What the reads of a scope declare once, beside the types they read through: the read of a
/// generic model by readers, for each of `arities`, answered for a type that has none by its own
/// `Deserialize`, and what turns the type argument a reader reads into the value the model's
/// parameter holds, refused where the two differ. A reader's value is the argument it reads,
/// through the one conversion there is, wherever no model asks another.
pub(super) fn reader_fallbacks(arities: &[usize]) -> TokenStream {
    let mut listed = arities.to_vec();
    listed.sort_unstable();
    listed.dedup();
    if listed.is_empty() {
        return TokenStream::new();
    }
    let fallbacks = listed.iter().map(|arity| {
        let named = format_ident!("ModelSchemaReadBy{arity}");
        let by = read_by(*arity);
        let readers: Vec<Ident> = (1..=*arity).map(|at| format_ident!("R{at}")).collect();
        quote! {
            trait #named<'de>: serde::Deserialize<'de> {
                fn #by<D #(, #readers: serde::de::DeserializeSeed<'de>)*>(deserializer: D) -> ::core::result::Result<Self, D::Error>
                where
                    D: serde::Deserializer<'de>,
                {
                    <Self as serde::Deserialize<'de>>::deserialize(deserializer)
                }
            }
            impl<'de, T: serde::Deserialize<'de>> #named<'de> for T {}
        }
    });
    let (message, label, note) = (
        rearranged("{V}", "{Self}"),
        REARRANGED_LABEL,
        REARRANGED_NOTE,
    );
    quote! {
        #[diagnostic::on_unimplemented(message = #message, label = #label, note = #note)]
        trait ModelSchemaReadFor<V>: ::core::marker::Sized {
            fn model_schema_read_for(self) -> V;
        }
        impl<W> ModelSchemaReadFor<W> for W {
            fn model_schema_read_for(self) -> W {
                self
            }
        }
        #(#fallbacks)*
    }
}

/// Whether `attrs` write `default = "..."`: a function serde builds the member's value with, of
/// the member's own type.
fn defaults_through_a_function(attrs: &[Attribute]) -> bool {
    let mut found = false;
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("serde")) {
        attr.parse_nested_meta(|nested| {
            if nested.path.is_ident("default") && nested.input.peek(syn::Token![=]) {
                found = true;
            }
            consume_unread_value(&nested)
        })
        .unwrap_or_else(|refused| {
            log::trace!("Failed to parse a serde default attribute: {refused}");
        });
    }
    found
}

/// Whether the read of the members of a variant written with `attrs` is the variant's own.
fn variant_reads_itself(attrs: &[Attribute]) -> bool {
    has_serde_read_hook(attrs) || parse_serde_key_omission(attrs).skips_deserializing
}

/// How `field`, a member of the item, is read with a reader handed in for each parameter it holds
/// a value behind, or `None` where it names no parameter, or where serde reads it through a
/// function, builds it by one, or skips it: it is then read as the item's own read reads it.
fn handed_member(field: &Field, scope: &ReadScope, taken: &mut Vec<String>) -> Option<Reached> {
    let written = naming_the_type(field.ty.to_token_stream(), &scope.named);
    if !tokens_name_any(&written, &scope.reach) || written.to_string().contains('\'') {
        return None;
    }
    if has_serde_read_hook(&field.attrs)
        || parse_serde_key_omission(&field.attrs).skips_deserializing
        || defaults_through_a_function(&field.attrs)
    {
        return None;
    }
    let shape = scope.shape_of(&field.ty)?;
    walked_read(&field.ty, 0, (&shape.wraps, &shape.end), taken)
}

/// The members of `input` read with readers handed on, by place: none that names a `preprocess`,
/// which `corrects` lists, and that the item's read calls ahead of the member's own.
fn handed_members(
    input: &DeriveInput,
    scope: &ReadScope,
    corrects: &Corrects,
    taken: &mut Vec<String>,
) -> Vec<Handed> {
    let mut handed = Vec::new();
    let mut walk = |variant: Option<String>, fields: &Fields, names: &mut Vec<String>| {
        for (index, field) in fields.iter().enumerate() {
            if !corrects
                .named
                .of_member(variant.as_deref(), index)
                .is_empty()
            {
                continue;
            }
            if let Some(reached) = handed_member(field, scope, names) {
                handed.push(Handed {
                    defaulted: derive_reads_a_default(&field.attrs),
                    index,
                    reached,
                    variant: variant.clone(),
                });
            }
        }
    };
    match &input.data {
        syn::Data::Struct(held) => walk(None, &held.fields, taken),
        syn::Data::Enum(held) => {
            for variant in &held.variants {
                if !variant_reads_itself(&variant.attrs) {
                    walk(Some(variant.ident.to_string()), &variant.fields, taken);
                }
            }
        }
        syn::Data::Union(_) => {}
    }
    handed
}

/// Whether some slot of `input` is declared under a `cfg`, which a pattern of slots cannot repeat.
fn attributes_a_slot(input: &DeriveInput) -> bool {
    let slots = |fields: &Fields| {
        matches!(fields, Fields::Unnamed(_))
            && fields
                .iter()
                .any(|field| field.attrs.iter().any(|attr| attr.path().is_ident("cfg")))
    };
    match &input.data {
        syn::Data::Struct(held) => slots(&held.fields),
        syn::Data::Enum(held) => held.variants.iter().any(|variant| slots(&variant.fields)),
        syn::Data::Union(_) => false,
    }
}

/// The pattern that binds the members of one variant, or of the struct, and the value of the
/// same place at `path` built back from them, each member `handed` names built back from the type
/// it was read as.
fn built_back(
    path: &TokenStream,
    fields: &Fields,
    variant: Option<&str>,
    handed: &[Handed],
) -> (TokenStream, TokenStream) {
    let value_of = |index: usize, bound: &Ident| {
        handed
            .iter()
            .find(|member| member.index == index && member.variant.as_deref() == variant)
            .map_or_else(
                || quote! { #bound },
                |member| {
                    let back = &member.reached.back;
                    quote! { { let held_0 = #bound; #back } }
                },
            )
    };
    match fields {
        Fields::Named(named) => {
            let members: Vec<(&Ident, Vec<&Attribute>)> = named
                .named
                .iter()
                .filter_map(|field| {
                    let cfgs = field
                        .attrs
                        .iter()
                        .filter(|attr| attr.path().is_ident("cfg"))
                        .collect();
                    field.ident.as_ref().map(|ident| (ident, cfgs))
                })
                .collect();
            let pattern = members
                .iter()
                .map(|(ident, cfgs)| quote! { #(#cfgs)* #ident });
            // A member read as it is written back is named alone, as a holder's own code names it.
            let values = members.iter().enumerate().map(|(index, (ident, cfgs))| {
                if handed
                    .iter()
                    .any(|member| member.index == index && member.variant.as_deref() == variant)
                {
                    let value = value_of(index, ident);
                    quote! { #(#cfgs)* #ident: #value }
                } else {
                    quote! { #(#cfgs)* #ident }
                }
            });
            (
                quote! { { #(#pattern),* } },
                quote! { #path { #(#values),* } },
            )
        }
        Fields::Unnamed(slots) => {
            let bound: Vec<Ident> = (0..slots.unnamed.len())
                .map(|at| format_ident!("slot_{}", at))
                .collect();
            let values = bound
                .iter()
                .enumerate()
                .map(|(index, slot)| value_of(index, slot));
            (quote! { ( #(#bound),* ) }, quote! { #path( #(#values),* ) })
        }
        Fields::Unit => (TokenStream::new(), quote! { #path }),
    }
}

/// The item built back from `read`, its copy `copied_as`, each member `handed` names built back
/// from the type it was read as.
fn handed_back(input: &DeriveInput, copied_as: &Ident, handed: &[Handed]) -> TokenStream {
    match &input.data {
        syn::Data::Struct(held) => {
            let (pattern, value) = built_back(&quote! { Self }, &held.fields, None, handed);
            quote! {
                {
                    let #copied_as #pattern = read;
                    #value
                }
            }
        }
        syn::Data::Enum(held) => {
            let arms = held.variants.iter().map(|variant| {
                let ident = &variant.ident;
                let named_as = ident.to_string();
                let cfgs = variant
                    .attrs
                    .iter()
                    .filter(|attr| attr.path().is_ident("cfg"));
                let (pattern, value) = built_back(
                    &quote! { Self::#ident },
                    &variant.fields,
                    Some(&named_as),
                    handed,
                );
                quote! { #(#cfgs)* #copied_as::#ident #pattern => #value, }
            });
            quote! {
                match read {
                    #(#arms)*
                }
            }
        }
        syn::Data::Union(_) => quote! { read },
    }
}

/// Whether `input` is a shape whose read a copy of it with readers cannot repeat: one that declares
/// a const parameter, writes its own read bounds, converts through a type of its author's, reads a
/// missing member from its own `Default`, or declares a slot under a `cfg`.
fn read_by_its_own(input: &DeriveInput, bounds: &ReadBounds) -> bool {
    input.generics.const_params().next().is_some()
        || bounds.written_by_container
        || serde_conversion(&input.attrs).is_some()
        || derive_reads_a_default(&input.attrs)
        || attributes_a_slot(input)
}

/// The body of `model_schema_read_unjudged_by{n}` of `input`, which reads each member of
/// `handed` with readers and every other member as the item's own read reads it, or `None` where
/// no member is read with readers.
fn read_with_readers(
    input: &DeriveInput,
    item: &ItemReaders,
    bounds: &ReadBounds,
    derive: &syn::Path,
) -> Option<TokenStream> {
    let scope = item.scope(input);
    let corrects = Corrects::of(&item.written);
    let mut taken = item.taken.clone();
    let handed = handed_members(input, &scope, &corrects, &mut taken);
    if handed.is_empty() {
        return None;
    }
    let mut declaring = Declaring::of(taken);
    let mut held_types = TokenStream::new();
    let mut read_as: Vec<(Option<String>, usize, TokenStream)> = Vec::new();
    for member in &handed {
        let (items, written_as) =
            scope.held_types(&member.reached, member.defaulted, &mut declaring);
        held_types.extend(items);
        read_as.push((member.variant.clone(), member.index, written_as));
    }
    let unjudged = Ident::new(UNJUDGED_READ, Span::call_site());
    let left_out = |variant: Option<&str>, index: usize| {
        handed
            .iter()
            .any(|member| member.index == index && member.variant.as_deref() == variant)
    };
    let hooks = hooked(
        input,
        bounds,
        &unjudged,
        &item.reader,
        &item.written,
        &corrects,
        &left_out,
    );
    declaring.arities.extend(hooks.arities);
    let functions = &hooks.functions;
    let called = writes(&quote! { #held_types #functions }, &unjudged).then_some(&unjudged);
    let fallbacks = hook_fallbacks(&item.written, called, hooks.choosing, &declaring.arities);
    let copied_as = unclaimed_parameter(REMOTE, &declaring.taken);
    let read = copy_read(&copied_as, &scope, &corrects, &item.reader);
    let built = handed_back(input, &copied_as, &handed);
    let copy_of = |renamed| {
        let copy = HandedCopy {
            copied_as: &copied_as,
            derive,
            hung: &hooks.hung,
            named: &item.named,
            predicates: &item.predicates,
            read_as: &read_as,
            reader_bounds: &item.reader_bounds,
            readers: &item.readers,
            renamed,
        }
        .written(input);
        quote! {
            #copy
            let read = #read;
            #built
        }
    };
    let own = unclaimed_parameter(OWN, &declaring.taken);
    let (copied, alias) = named_after_the_item(input, &copied_as, &own, &copy_of(false))
        .unwrap_or_else(|| (copy_of(true), None));
    Some(quote! {
        #fallbacks
        #functions
        #held_types
        #alias
        ::core::result::Result::Ok({ #copied })
    })
}

/// `copied`, the copy `copied_as` and its read, with the copy named after the item, as serde's
/// sentences name the item where its own read reads it, and each word of the item's own name
/// written as `own`, an alias of the item, declared beside it. `None` where a string in `copied`
/// names the item.
fn named_after_the_item(
    input: &DeriveInput,
    copied_as: &Ident,
    own: &Ident,
    copied: &TokenStream,
) -> Option<(TokenStream, Option<TokenStream>)> {
    let ident = &input.ident;
    let aliased = words_written_as(copied.clone(), &[(ident.to_string(), quote! { #own })]);
    if written_names(aliased.clone()).contains(&ident.to_string()) {
        return None;
    }
    let alias = writes(copied, ident).then(|| {
        let (_, type_generics, _) = input.generics.split_for_impl();
        quote! { type #own #type_generics = #ident #type_generics; }
    });
    Some((
        words_written_as(aliased, &[(copied_as.to_string(), quote! { #ident })]),
        alias,
    ))
}

/// The read of the copy `copied_as`, from the payload the functions the item's own `preprocess`
/// names corrected where it names any.
fn copy_read(
    copied_as: &Ident,
    scope: &ReadScope,
    corrects: &Corrects,
    reader: &Ident,
) -> TokenStream {
    let generics = scope.generics();
    let own = &corrects.named.own;
    if own.is_empty() {
        return quote! { <#copied_as #generics as serde::Deserialize<'de>>::deserialize(deserializer)? };
    }
    let (corrected_deserializer, corrected) = (&corrects.deserializer, &corrects.corrected);
    let correction = corrects.statements(own);
    let told = told_by(reader);
    quote! {
        {
            let #corrected_deserializer = deserializer;
            #correction
            <#copied_as #generics as serde::Deserialize<'de>>::deserialize(#corrected) #told ?
        }
    }
}

/// `model_schema_read_unjudged_by{n}` of `input`, which declares type parameters, in an `impl` of
/// it: the members that hold a value behind a parameter read with the reader handed in for it,
/// each other member as the item's own read reads it. A shape no copy of the item can read with
/// readers, and an item that `walks` nothing behind its parameters, is read by the item's own
/// read, `own_read` where the macro writes one, and serde's otherwise: what fills a parameter then
/// judges itself as it is read.
fn read_by_readers(
    input: &DeriveInput,
    derive: &syn::Path,
    bounds: &ReadBounds,
    own_read: bool,
    walks: bool,
) -> TokenStream {
    let item = ItemReaders::of(input, bounds);
    let name = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
    let signature = item.signature();
    let delegated = if own_read {
        quote! { Self::model_schema_read_unjudged(deserializer) }
    } else {
        quote! { <Self as serde::Deserialize<'de>>::deserialize(deserializer) }
    };
    let body = (walks && !read_by_its_own(input, bounds))
        .then(|| read_with_readers(input, &item, bounds, derive))
        .flatten()
        .unwrap_or(delegated);
    quote! {
        impl #impl_generics #name #type_generics #where_clause {
            #signature
            {
                #body
            }
        }
    }
}

/// The reads a hook of the item's own read writes for a generic model it holds, `leaf`: through
/// `model_schema_read_unjudged_by{n}`, handed a reader of each type argument the model is written
/// with, each argument read as the hook reads that type. `None` for a model written with none.
/// The hook writes the item's name where the member writes `Self`, so no argument writes it.
pub(super) fn hooked_by_readers(
    leaf: &TokenStream,
    carried: &[Ident],
    predicates: &[TokenStream],
    declared: &[TokenStream],
    reader: &Ident,
    taken: &[String],
) -> Option<(TokenStream, Vec<usize>)> {
    let written = syn::parse2::<syn::Type>(leaf.clone()).ok()?;
    let syn::Type::Path(path) = &written else {
        return None;
    };
    let arguments = leaf_arguments(path.path.segments.last()?);
    if arguments.is_empty() {
        return None;
    }
    let scope = ReadScope {
        carried: carried.to_vec(),
        declared: declared.to_vec(),
        named: leaf.clone(),
        predicates: predicates.to_vec(),
        reach: Vec::new(),
        reader: reader.clone(),
        readers: Vec::new(),
    };
    let mut declaring = Declaring::of(taken.to_vec());
    declaring.taken.push(HELD.to_owned());
    let read = scope.handed_read(leaf, &arguments, &mut declaring);
    Some((read, declaring.arities))
}

/// `model_schema_read_unjudged_by{n}` of `input`, an item that declares type parameters and
/// derives `Deserialize`, and of `valid`, its valid type, where it has one, each in an `impl` of
/// its own. `own_read` says the macro writes the item's unjudged read. Where `walks` is false the
/// item's `model_schema_issues_with{n}` reads nothing behind its parameters, and so its read by
/// readers reads them as its own read does. Nothing for any other item, nor for one that borrows
/// or is read at `'static`.
pub(super) fn by_readers(
    input: &DeriveInput,
    valid: Option<&Ident>,
    own_read: bool,
    walks: bool,
) -> (TokenStream, TokenStream) {
    let nothing = (TokenStream::new(), TokenStream::new());
    if input.generics.type_params().next().is_none() || !derives_deserialize(&input.attrs) {
        return nothing;
    }
    let Some(derive) = super::deserialize_derive(&input.attrs) else {
        return nothing;
    };
    let bounds = super::authored_read_bounds(input);
    // A type that borrows, or is read at `'static`, is held by no hook: a holder that names it
    // reads it by its own `Deserialize`.
    if input.generics.lifetimes().next().is_some()
        || bounds
            .borrowed
            .as_ref()
            .is_none_or(|borrowed| !borrowed.is_empty())
    {
        return nothing;
    }
    let draft = read_by_readers(input, &derive, &bounds, own_read, walks);
    let wrapper = valid
        .map(|valid_name| valid_read_by(input, valid_name, &bounds))
        .unwrap_or_default();
    (draft, wrapper)
}

/// `model_schema_read_unjudged_by{n}` of the valid type `valid` of the draft `input`: the draft's,
/// wrapped, with serde's message worded with the declared name.
fn valid_read_by(input: &DeriveInput, valid: &Ident, bounds: &ReadBounds) -> TokenStream {
    let item = ItemReaders::of(input, bounds);
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
    let (named, reader, signature) = (&item.named, &item.reader, item.signature());
    let reader_idents = item.readers.iter().map(|(_, read)| read);
    let by = read_by(item.readers.len());
    let relabels = relabelled(&input.ident, valid);
    quote! {
        impl #impl_generics #valid #type_generics #where_clause {
            #signature
            {
                #relabels
                <#named>::#by::<#reader #(, #reader_idents)*>(deserializer)
                    .map(Self::model_schema_wrap_unjudged)
                    .map_err(|error| match relabelled(&::std::string::ToString::to_string(&error)) {
                        ::core::option::Option::Some(worded) => <#reader::Error as serde::de::Error>::custom(worded),
                        ::core::option::Option::None => error,
                    })
            }
        }
    }
}
