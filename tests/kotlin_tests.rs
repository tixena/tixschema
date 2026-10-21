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

/// The models an enum with a variant of several slots is written on, under each tagging, and a
/// tuple in each place a writer names its wrapper more than once.
#[cfg(test)]
#[cfg(feature = "kotlin")]
#[path = "kotlin_tests/tuple_variants_models.rs"]
mod tuple_variants_models;

/// An enum with a variant of several slots, its slots written and read by one tuple wrapper, and
/// one wrapper per tuple shape in each place a writer names a tuple more than once.
#[cfg(test)]
#[cfg(feature = "kotlin")]
#[path = "kotlin_tests/tuple_variants.rs"]
mod tuple_variants;

/// A struct whose members are all flattened, one of them a map, and its twin with a member of its
/// own. Declared after the module that declares the member both flatten.
#[cfg(test)]
#[cfg(feature = "kotlin")]
#[path = "kotlin_tests/flattened_models.rs"]
mod flattened_models;
