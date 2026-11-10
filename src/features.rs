//! Feature detection and conditional compilation utilities for tixschema.
//!
//! This module provides compile-time feature detection and utilities for handling
//! different feature combinations in the macro expansion process.

// Not gated on the `serde` feature: every build asks it what serde writes, and without the feature
// `serde_attributes` yields no attribute.
pub mod serde;

#[cfg(feature = "serde")]
pub mod recovering_decode;

#[cfg(feature = "zod")]
pub mod zod;

#[cfg(feature = "zod")]
pub mod js_literal;

pub mod jsonschema;

#[cfg(feature = "mongodb")]
pub mod object_id;

#[cfg(feature = "chrono")]
pub mod chrono;

#[cfg(any(
    feature = "dart",
    feature = "swift",
    feature = "kotlin",
    feature = "chrono"
))]
pub mod text_forms;

#[cfg(feature = "dart")]
pub mod dart;

#[cfg(all(
    feature = "serde",
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
pub mod member_keys;

#[cfg(feature = "swift")]
pub mod swift;

#[cfg(feature = "kotlin")]
pub mod kotlin;

#[cfg(feature = "arky")]
pub mod arky;

/// Module for parsing model_schema_prop attributes
pub mod model_schema_prop;

// Gated on `serde`, because `#[service_schema]` is: the emitters this module names live in
// `crate::service_schema`.
#[cfg(feature = "serde")]
pub mod service_schema;

/// The text serde reads a float map key from, as a regular expression that matches the whole text.
#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
pub const JSON_NUMBER: &str = "^-?(0|[1-9][0-9]*)([.][0-9]+)?([eE][+-]?[0-9]+)?$";

/// Feature detection utilities.
#[cfg(test)]
pub struct Features;

#[cfg(test)]
impl Features {
    /// Get a description of enabled features for debugging.
    pub fn enabled_features() -> Vec<&'static str> {
        let mut features = Vec::new();

        if Self::has_serde() {
            features.push("serde");
        }
        if Self::has_zod() {
            features.push("zod");
        }
        if Self::has_mongodb() {
            features.push("mongodb");
        }
        if Self::has_bson() {
            features.push("bson");
        }
        if Self::has_typescript() {
            features.push("typescript");
        }
        if Self::has_chrono() {
            features.push("chrono");
        }
        if Self::has_indexmap() {
            features.push("indexmap");
        }
        if Self::has_dart() {
            features.push("dart");
        }
        if Self::has_swift() {
            features.push("swift");
        }
        if Self::has_kotlin() {
            features.push("kotlin");
        }
        if Self::has_arky() {
            features.push("arky");
        }

        if features.is_empty() {
            features.push("minimal");
        }

        features
    }

    /// Check if `arky` feature is enabled.
    pub const fn has_arky() -> bool {
        cfg!(feature = "arky")
    }

    pub const fn has_bson() -> bool {
        cfg!(feature = "bson")
    }

    /// Check if `chrono` feature is enabled.
    pub const fn has_chrono() -> bool {
        cfg!(feature = "chrono")
    }

    /// Check if `dart` feature is enabled.
    pub const fn has_dart() -> bool {
        cfg!(feature = "dart")
    }

    /// Check if `indexmap` feature is enabled.
    pub const fn has_indexmap() -> bool {
        cfg!(feature = "indexmap")
    }

    pub const fn has_kotlin() -> bool {
        cfg!(feature = "kotlin")
    }

    /// Check if `mongodb` feature is enabled.
    pub const fn has_mongodb() -> bool {
        cfg!(feature = "mongodb")
    }

    /// Check if serde feature is enabled.
    pub const fn has_serde() -> bool {
        cfg!(feature = "serde")
    }

    pub const fn has_swift() -> bool {
        cfg!(feature = "swift")
    }

    /// Check if typescript feature is enabled.
    pub const fn has_typescript() -> bool {
        cfg!(feature = "typescript")
    }

    /// Check if zod feature is enabled.
    pub const fn has_zod() -> bool {
        cfg!(feature = "zod")
    }
}

// Note: Proc-macro crates cannot export macro_rules! macros
// Instead, we use cfg attributes directly where needed

/// The statements that write into `written` the keys of each member the client text names by a
/// placeholder, as [`member_keys::filled`] writes them.
#[cfg(all(
    feature = "serde",
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
pub fn unmet_keys_filled(
    written: &proc_macro2::Ident,
    module: &proc_macro2::Ident,
    quoted: fn(&str) -> String,
    quote: char,
    escaped: &[(char, &str)],
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    member_keys::filled(written, module, quoted, quote, escaped)
}

/// With no `serde`, nothing is flattened.
#[cfg(all(
    not(feature = "serde"),
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
pub fn unmet_keys_filled(
    _written: &proc_macro2::Ident,
    _module: &proc_macro2::Ident,
    _quoted: fn(&str) -> String,
    _quote: char,
    _escaped: &[(char, &str)],
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    (
        proc_macro2::TokenStream::new(),
        proc_macro2::TokenStream::new(),
    )
}

#[cfg(test)]
mod tests;
