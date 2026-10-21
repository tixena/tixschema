//! The test binary of `kotlin_tests/`.

#[cfg(test)]
#[cfg(feature = "kotlin")]
#[path = "kotlin_tests/tests.rs"]
mod tests;

#[cfg(test)]
#[cfg(feature = "kotlin")]
#[path = "kotlin_tests/validate.rs"]
mod validate;

/// The two serializers of each type: the read alone, and the type's own, which judges.
#[cfg(test)]
#[cfg(feature = "kotlin")]
#[path = "kotlin_tests/read.rs"]
mod read;

/// `preprocess`: the functions a type and its members name, as the read of each calls them.
#[cfg(test)]
#[cfg(feature = "kotlin")]
#[path = "kotlin_tests/preprocess.rs"]
mod preprocess;

/// The models an enum with a variant of several slots is written on, under each tagging.
#[cfg(test)]
#[cfg(feature = "kotlin")]
#[path = "kotlin_tests/tuple_variants_models.rs"]
mod tuple_variants_models;

/// An enum with a variant of several slots, its slots written and read by one tuple wrapper.
#[cfg(test)]
#[cfg(feature = "kotlin")]
#[path = "kotlin_tests/tuple_variants.rs"]
mod tuple_variants;
