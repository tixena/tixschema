//! Model schema property feature module

use proc_macro2::Span;
use syn::meta::ParseNestedMeta;
use syn::{Attribute, Lit, LitStr, Type};

use crate::utils::{constraining_pattern, emittable_pattern, portable_pattern};

/// Every key the parser reads, in the order the unknown-key rejection names them. Add a new key to
/// [`parse_prop_key`], add it here too, or `no_key_the_parser_reads_is_rejected` fails.
const KNOWN_KEYS: &[&str] = &[
    "as",
    "literal",
    "minLength",
    "maxLength",
    "minimum",
    "maximum",
    "pattern",
    "preprocess",
    "check",
    "ts_optional",
    "as_number",
    "nullable",
];

/// What a refusal of a `check` written in `#[model_schema_prop(...)]` opens with.
const PROP_CHECK: &str = "`model_schema_prop` key `check`";

/// What a refusal of a `preprocess` written in `#[model_schema_prop(...)]` opens with.
const PROP_PREPROCESS: &str = "`model_schema_prop` key `preprocess`";

/// What a `check` takes, said where it is written any other way.
pub const CHECK_TAKES: &str =
    "takes an array of string literals, each naming a function: `check = [\"withinParcel\"]`";

/// What a `preprocess` takes, said where it is written any other way.
pub const PREPROCESS_TAKES: &str =
    "takes an array of string literals, each naming a function: `preprocess = [\"trimmed\"]`";

/// One function a `check` or a `preprocess` key names, and where the name is written.
#[derive(Clone, Debug)]
pub struct FunctionName {
    pub name: String,
    pub written_at: Span,
}

/// The value a `literal` key was written with, kept in the kind it was written as. The kind decides
/// which [`crate::field_type::FieldDefType`] the field collapses to and which Rust type may carry
/// it.
#[derive(Clone, Debug, PartialEq)]
pub enum LiteralValue {
    Bool(bool),
    Number(f64),
    Str(String),
}

/// Metadata for `model_schema_prop` attributes applied to a field.
///
/// # Supported attributes
///
/// ## String constraints
///
/// Each applies to a field that renders a plain string — `String`, `str`, `PathBuf`, `Path`, and
/// those under any number of `Option` and transparent wrappers. A type whose schema this crate
/// writes whole (`ObjectId`, the chrono types) renders no bound and reading one is a compile error
/// rather than a silent drop.
///
/// - `pattern = "regex"` — validates the string matches the regex pattern. Must be a regex all
///   three engines read the same way; see [`crate::utils::portable_pattern`] for what that rules
///   out and what it rewrites. It must also turn some value away: a pattern every string satisfies
///   is refused where it is written rather than published as a check that checks nothing — see
///   [`crate::utils::constraining_pattern`].
///   - Zod: `.check(z.regex(/regex/))`
///   - JSON Schema: `"pattern"`
///   - Rust: `validate()` — see "Where a constraint is checked" below
///
/// - `minLength = N` — minimum string length (inclusive).
///   - Zod: `.min(N)`
///   - JSON Schema: `"minLength"`
///   - Rust: `validate()`
///
/// - `maxLength = N` — maximum string length (inclusive).
///   - Zod: `.max(N)`
///   - JSON Schema: `"maxLength"`
///   - Rust: `validate()`
///
/// ## List length
///
/// A key measures the field's own value, and a list is a value of its own: on one, `minLength`
/// and `maxLength` count its items and reach none of them, and `pattern`, `minimum` and `maximum`
/// are compile errors.
///
/// - `minLength = N` / `maxLength = N` on a list — the fewest and the most items it holds.
///   - Zod: `.min(N)` / `.max(N)` on the array
///   - JSON Schema: `"minItems"` / `"maxItems"`
///   - Rust: `validate()`
///
/// ## Numeric constraints
///
/// - `minimum = N` — minimum value for numeric fields (integer or float).
///   - Zod: `.min(N)`
///   - JSON Schema: `"minimum"`
///   - Rust: `validate()`
///
/// - `maximum = N` — maximum value for numeric fields (integer or float).
///   - Zod: `.max(N)`
///   - JSON Schema: `"maximum"`
///   - Rust: `validate()`
///
/// ## Where a constraint is checked
///
/// By the `validate()` the type publishes, and not as the payload is read.
///
/// A constraint describes the value, not the shape, so a payload carrying a value it rejects is
/// still structurally the message it claims to be — every key present, every value of its field's
/// declared type. Checking it on the read makes the two indistinguishable to whoever receives the
/// failure: "I could not parse this at all" and "I parsed it and the value broke a rule" are
/// different sentences that send a caller looking in different places. So the read admits the
/// value and `validate()` refuses it, naming the field.
///
/// A field still generates both helpers into the schema module — `validate_{field}_value()`, which
/// `validate()` calls, and `deserialize_{field}`, a serde hook an author may hang on a field of
/// their own accord. Only one position is hung with that hook automatically: a member of an
/// `#[serde(untagged)]` enum, where whether the member is admissible is what chooses which variant
/// the payload is. There the check is part of reading the value rather than part of judging it,
/// exactly as it is under `anyOf` and `z.union` on the two schema surfaces the same type
/// publishes, and `validate()` cannot stand in for it — by the time it runs the variant has
/// already been chosen.
///
/// A constrained *brand* is judged the same way. A message holding a branded field publishes a
/// `validate()` that runs the brand's own validator and reports what it said under the field that
/// held it — `'slug': too short: …`, the name being the message's to supply since the brand names
/// none of its own. That is what holds a caller building a message in Rust, where no read ever
/// ran. A member of an `#[serde(untagged)]` enum that holds a brand is held to the brand's bound as
/// it is read, the bound choosing the variant.
///
/// The same reach applies to a field whose type is an ordinary `#[model_schema()]` type: the
/// enclosing `validate()` runs the one that type published and writes its report under the field.
/// Nothing else enforces a nested bound, the read carrying no check for one at all.
///
/// ## Type overrides
///
/// - `as = Type` — name the type emitted for this field. The target must be the field's own type
///   or the value under its wrappers (`as = String` on a `Vec<String>`); any other target is a
///   compile error. Cannot be written beside `preprocess`.
/// - `literal = "value"` — emit as a literal type instead of the field's own primitive. Takes a
///   string, boolean, integer or float literal; the kind written must match what the field's Rust
///   type can carry (a boolean literal on a `bool` field, a numeric literal on a numeric field, a
///   string literal on a `String` field) or the attribute is a compile error naming the mismatch.
/// - `ts_optional` — for an `Option<T>` field, emit `field?: T` instead of `field: T | undefined`
///   (TypeScript only; a non-`Option` field is a compile error). It decides the key only on a field
///   no serde key-omission attribute speaks for, since such an attribute already writes the
///   optional key off the wire in every build. Under the `serde` feature that field does not
///   compile — the `Option`-null guard refuses it — so the one shape the flag is live on is a build
///   with the feature off.
///
/// ## Preprocessing
///
/// - `preprocess = ["fn1", "fn2"]` — the functions of the developer's own that correct the field's
///   payload before the field is read from it, in the order written, each over the answer of the
///   one before. Every Rust read of the type calls the snake case of each name over a
///   `serde_json::Value`, and the Zod schema nests them, the first outermost:
///   `z.preprocess(fn1, z.preprocess(fn2, innerSchema))`.
///
/// ## Checks
///
/// - `check = ["fn1", "fn2"]` — the functions of the developer's own that hold the field's value
///   to a rule, each run after the field's bounds, in the order written. The value is the field's
///   own, read as a bound reads it: under `Option` and the transparent wrappers, and a list whole.
///   - Rust: `validate()` calls the snake case of each name, `fn1(&value) -> Result<(),
///     Vec<String>>`, and writes each message under the field. A type declared with `decode_with`
///     lists each message as an issue of its recovering read.
///   - Zod: one `.check(…)` per name on the field's schema, calling `fn1(value, ctx)`.
///   - JSON Schema: nothing. A document calls no function.
///
/// # Example
///
/// ```rust
/// use tixschema::{model_schema, model_schema_prop};
/// use serde::{Deserialize, Serialize};
///
/// #[model_schema()]
/// #[derive(Serialize, Deserialize)]
/// pub struct User {
///     #[model_schema_prop(minLength = 3, maxLength = 50, pattern = "^[a-z]+$")]
///     pub username: String,
///
///     #[model_schema_prop(minimum = 0, maximum = 120)]
///     pub age: u32,
///
///     #[model_schema_prop(pattern = "^[0-9a-fA-F]{24}$")]
///     pub id: String,
/// }
/// ```
#[derive(Clone, Debug, Default)]
pub struct ModelSchemaPropMeta {
    pub as_number: bool,
    pub as_type: Option<Type>,
    pub attr_rejection: Option<syn::Error>,
    /// The functions `check` names, in the order written.
    pub check: Vec<FunctionName>,
    /// The `pattern` of the enum the field's variant belongs to, where the field writes one of its
    /// own in [`Self::pattern`]: the value is held to both.
    #[cfg(any(feature = "serde", feature = "zod", feature = "jsonschema"))]
    pub enum_pattern: Option<String>,
    pub literal: Option<LiteralValue>,
    /// `maxLength` written on a list, whose items it counts: moved here from [`Self::max_length`]
    /// as the field takes its metadata, so no reader of a string's length finds it.
    pub max_items: Option<usize>,
    pub max_length: Option<usize>,
    pub maximum: Option<f64>,
    /// `minLength` written on a list, as [`Self::max_items`] is `maxLength`.
    pub min_items: Option<usize>,
    pub min_length: Option<usize>,
    pub minimum: Option<f64>,
    pub nullable: bool,
    /// `pattern` in the spelling every surface reads the same way, or as it was written when it
    /// earned a [`Self::pattern_rejection`].
    pub pattern: Option<String>,
    /// What keeps `pattern` off the surfaces it was written for, spanned on the literal it was
    /// written as.
    pub pattern_rejection: Option<syn::Error>,
    /// The functions `preprocess` names, in the order written.
    pub preprocess: Vec<FunctionName>,
    pub ts_optional: bool,
}

#[cfg(any(feature = "serde", feature = "zod"))]
impl ModelSchemaPropMeta {
    /// Every pattern the value is held to: the field's own, then its enum's where both are written.
    pub fn patterns(&self) -> impl Iterator<Item = &str> {
        self.pattern
            .iter()
            .chain(&self.enum_pattern)
            .map(String::as_str)
    }
}

/// What the parser cannot read is recorded as [`ModelSchemaPropMeta::attr_rejection`] rather than
/// dropped, and the field is emitted as though the attribute had been left off.
pub fn parse_model_schema_prop_attributes(attrs: &[Attribute]) -> ModelSchemaPropMeta {
    let mut meta = ModelSchemaPropMeta::default();

    for attr in attrs {
        if attr.path().is_ident("model_schema_prop")
            && let Err(rejection) =
                attr.parse_nested_meta(|nested| parse_prop_key(&nested, &mut meta))
        {
            meta.attr_rejection.get_or_insert(rejection);
        }
    }

    meta
}

/// Reads one `key` or `key = value` of a `model_schema_prop` attribute into `meta`.
fn parse_prop_key(nested: &ParseNestedMeta, meta: &mut ModelSchemaPropMeta) -> syn::Result<()> {
    if nested.path.is_ident("as") {
        meta.as_type = Some(nested.value()?.parse::<Type>()?);
    } else if nested.path.is_ident("literal") {
        meta.literal = Some(literal_prop_value(nested)?);
    } else if nested.path.is_ident("minLength") {
        meta.min_length = Some(nested.value()?.parse::<syn::LitInt>()?.base10_parse()?);
    } else if nested.path.is_ident("maxLength") {
        meta.max_length = Some(nested.value()?.parse::<syn::LitInt>()?.base10_parse()?);
    } else if nested.path.is_ident("minimum") {
        meta.minimum = Some(numeric_bound(nested, "minimum")?);
    } else if nested.path.is_ident("maximum") {
        meta.maximum = Some(numeric_bound(nested, "maximum")?);
    } else if nested.path.is_ident("pattern") {
        let lit: LitStr = nested.value()?.parse()?;
        // A refused pattern is recorded as written: the guards that answer for what a `pattern`
        // may sit on read that one was given, not what it says.
        match portable_pattern(&lit)
            .and_then(|portable| constraining_pattern(&lit, portable))
            .and_then(|constraining| emittable_pattern(&lit, constraining))
        {
            Ok(pattern) => meta.pattern = Some(pattern),
            Err(rejection) => {
                meta.pattern_rejection = Some(rejection);
                meta.pattern = Some(lit.value());
            }
        }
    } else if nested.path.is_ident("preprocess") {
        meta.preprocess = named_functions(nested, PROP_PREPROCESS, PREPROCESS_TAKES)?;
    } else if nested.path.is_ident("check") {
        meta.check = named_functions(nested, PROP_CHECK, CHECK_TAKES)?;
    } else if nested.path.is_ident("ts_optional") {
        meta.ts_optional = true;
    } else if nested.path.is_ident("as_number") {
        meta.as_number = true;
    } else if nested.path.is_ident("nullable") {
        meta.nullable = true;
    } else {
        return Err(unknown_key_rejection(nested));
    }
    Ok(())
}

/// The `f64` a numeric bound was written as: both bounds reach a numeric comparison in the Rust
/// validator and a numeric literal in the Zod and JSON schemas, so a non-number is one no surface
/// can carry.
fn numeric_bound(nested: &ParseNestedMeta, key: &str) -> syn::Result<f64> {
    let lit: syn::Lit = nested.value()?.parse()?;
    if let syn::Lit::Int(int) = &lit {
        int.base10_parse()
    } else if let syn::Lit::Float(float) = &lit {
        float.base10_parse()
    } else {
        Err(syn::Error::new_spanned(
            &lit,
            format!("`model_schema_prop` key `{key}` takes an integer or float literal"),
        ))
    }
}

/// The [`LiteralValue`] a `literal` key was written as, kept in whichever of the four kinds the
/// author wrote — the kind [`crate::model_schema`](macro@crate::model_schema)'s own guard then
/// measures against the field's declared Rust type.
fn literal_prop_value(nested: &ParseNestedMeta) -> syn::Result<LiteralValue> {
    let lit: Lit = nested.value()?.parse()?;
    if let Lit::Str(str_lit) = &lit {
        Ok(LiteralValue::Str(str_lit.value()))
    } else if let Lit::Bool(bool_lit) = &lit {
        Ok(LiteralValue::Bool(bool_lit.value()))
    } else if let Lit::Int(int_lit) = &lit {
        Ok(LiteralValue::Number(int_lit.base10_parse()?))
    } else if let Lit::Float(float_lit) = &lit {
        Ok(LiteralValue::Number(float_lit.base10_parse()?))
    } else {
        Err(syn::Error::new_spanned(
            &lit,
            "`model_schema_prop` key `literal` takes a string, boolean, integer or float literal",
        ))
    }
}

/// The functions a `check` or a `preprocess` of `#[model_schema_prop]` names, written as the value
/// of the key `nested` stands at.
fn named_functions(
    nested: &ParseNestedMeta,
    subject: &str,
    takes: &str,
) -> syn::Result<Vec<FunctionName>> {
    if !nested.input.peek(syn::Token![=]) {
        return Err(check_refusal(&nested.path, subject, takes));
    }
    let written: syn::Expr = nested.value()?.parse()?;
    function_names(&written, subject, takes)
}

/// The functions a `check` or a `preprocess` names, read off the value it is written with, for
/// `#[model_schema]` and `#[model_schema_prop]` alike. `subject` opens a refusal, the attribute
/// and the key, and `takes` says what the key is written with.
pub fn function_names(
    written: &syn::Expr,
    subject: &str,
    takes: &str,
) -> syn::Result<Vec<FunctionName>> {
    let syn::Expr::Array(array) = written else {
        return Err(check_refusal(written, subject, takes));
    };
    if array.elems.is_empty() {
        return Err(check_refusal(
            written,
            subject,
            "names no function: write one name at least, or drop the key",
        ));
    }
    let mut named: Vec<FunctionName> = Vec::with_capacity(array.elems.len());
    for element in &array.elems {
        let read = check_name(element, subject)?;
        if named.iter().any(|earlier| earlier.name == read.name) {
            return Err(check_refusal(
                element,
                subject,
                &format!(
                    "names `{}` twice: the name is written twice, and a function runs once",
                    read.name
                ),
            ));
        }
        named.push(read);
    }
    Ok(named)
}

/// The name one element of a `check` or of a `preprocess` carries: the function is called under
/// it in every language, so it is a string literal that spells an identifier.
fn check_name(element: &syn::Expr, subject: &str) -> syn::Result<FunctionName> {
    let syn::Expr::Lit(syn::ExprLit {
        lit: Lit::Str(written),
        attrs: _attrs,
    }) = element
    else {
        return Err(check_refusal(
            element,
            subject,
            "takes string literals: each element is a string literal that names a function",
        ));
    };
    let name = written.value();
    let mut letters = name.chars();
    let opens = letters
        .next()
        .is_some_and(|letter| letter.is_ascii_alphabetic() || letter == '_');
    if opens && letters.all(|letter| letter.is_ascii_alphanumeric() || letter == '_') {
        return Ok(FunctionName {
            name,
            written_at: written.span(),
        });
    }
    Err(check_refusal(
        element,
        subject,
        &format!(
            "names `{name}`, and a name is an identifier: ASCII letters, digits and `_`, not led \
             by a digit. The name is written as a function's name in every language"
        ),
    ))
}

/// A refusal of a `check` or of a `preprocess`, spanned on what was written.
fn check_refusal(written: impl quote::ToTokens, subject: &str, refusal: &str) -> syn::Error {
    syn::Error::new_spanned(written, format!("{subject} {refusal}"))
}

/// Rejects a key the parser does not read, spanned on the name as written.
fn unknown_key_rejection(nested: &ParseNestedMeta) -> syn::Error {
    let path = &nested.path;
    nested.error(format!(
        "unknown `model_schema_prop` key `{}`. This attribute is this crate's own, so a key it \
         does not read reaches no emitter: the field would be written as though the key had been \
         left off, unconstrained on every surface. Valid keys: {}",
        quote::quote!(#path),
        KNOWN_KEYS.join(", ")
    ))
}

#[cfg(test)]
mod tests;
