//! `decode_with_foundation!()` and `mongodb_foundation!()`: the support libraries of a crate's
//! recovering decode, each written once at the crate root.

use proc_macro2::TokenStream;

#[cfg(feature = "serde")]
use crate::features::recovering_decode::{decode_with_foundation, mongodb_foundation};

/// What `decode_with_foundation!()` writes: the `decode_with` library and its marker, and nothing
/// in a build without `serde`, where no type has a recovering decode.
pub fn exec_decode_with_foundation(input: &TokenStream) -> TokenStream {
    refused_arguments(input, "decode_with_foundation").unwrap_or_else(decode_with_library)
}

/// What `mongodb_foundation!()` writes: the MongoDB library and its marker under `mongodb`, and
/// nothing without it.
pub fn exec_mongodb_foundation(input: &TokenStream) -> TokenStream {
    refused_arguments(input, "mongodb_foundation").unwrap_or_else(mongodb_library)
}

/// The refusal of anything written between the parentheses, spanned on it: the macro takes no
/// arguments.
fn refused_arguments(input: &TokenStream, macro_name: &str) -> Option<TokenStream> {
    (!input.is_empty()).then(|| {
        syn::Error::new_spanned(
            input,
            format!(
                "tixschema: `{macro_name}!` takes no arguments: write \
                 `tixschema::{macro_name}!();` at the crate root"
            ),
        )
        .to_compile_error()
    })
}

#[cfg(feature = "serde")]
fn decode_with_library() -> TokenStream {
    decode_with_foundation()
}

#[cfg(not(feature = "serde"))]
fn decode_with_library() -> TokenStream {
    TokenStream::new()
}

#[cfg(feature = "serde")]
fn mongodb_library() -> TokenStream {
    mongodb_foundation()
}

#[cfg(not(feature = "serde"))]
fn mongodb_library() -> TokenStream {
    TokenStream::new()
}
