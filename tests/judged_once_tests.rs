//! The test binary of `judged_once_tests/`: a model held in a member whose type names one of the
//! holder's own type parameters, or `Self`, and a value behind a type parameter of a model the
//! holder fills, is read unjudged and judged once, by the type the read was asked for.

#[cfg(test)]
#[cfg(all(feature = "serde", any(feature = "typescript", feature = "zod")))]
#[path = "judged_once_tests/models.rs"]
mod models;

#[cfg(test)]
#[cfg(all(feature = "serde", any(feature = "typescript", feature = "zod")))]
#[path = "judged_once_tests/tests.rs"]
mod tests;

#[cfg(test)]
#[cfg(all(feature = "serde", any(feature = "typescript", feature = "zod")))]
#[path = "judged_once_tests/parameters.rs"]
mod parameters;
