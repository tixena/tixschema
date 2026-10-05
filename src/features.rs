//! Feature detection and conditional compilation utilities for tixschema.
//!
//! This module provides compile-time feature detection and utilities for handling
//! different feature combinations in the macro expansion process.

// Not gated on the `serde` feature: the attributes that decide whether a key reaches the wire are
// written on the item under every toggle, and the surfaces describe that wire in every build. What
// the feature gates is everything else the module reads — renaming, tagging, and the guards.
pub mod serde;

#[cfg(feature = "serde")]
pub mod recovering_decode;

#[cfg(feature = "zod")]
pub mod zod;

#[cfg(feature = "jsonschema")]
pub mod jsonschema;

#[cfg(feature = "mongodb")]
pub mod object_id;

#[cfg(feature = "chrono")]
pub mod chrono;

#[cfg(feature = "dart")]
pub mod dart;

#[cfg(feature = "swift")]
pub mod swift;

#[cfg(feature = "kotlin")]
pub mod kotlin;

/// Module for parsing model_schema_prop attributes
pub mod model_schema_prop;

// Gated on `serde`, because `#[service_schema]` is: the emitters this module names live in
// `crate::service_schema`. Not gated on `typescript`: the module holds every language's
// accessors, each gated individually.
#[cfg(feature = "serde")]
pub mod service_schema;

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
        if Self::has_jsonschema() {
            features.push("jsonschema");
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
        if Self::has_dart() {
            features.push("dart");
        }
        if Self::has_swift() {
            features.push("swift");
        }
        if Self::has_kotlin() {
            features.push("kotlin");
        }

        if features.is_empty() {
            features.push("minimal");
        }

        features
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

    /// Check if jsonschema feature is enabled.
    pub const fn has_jsonschema() -> bool {
        cfg!(feature = "jsonschema")
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

#[cfg(test)]
mod tests;
