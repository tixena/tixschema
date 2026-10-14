//! The test binary of `judged_once_tests/`: a model held in a member whose type names one of the
//! holder's own type parameters, or `Self`, is read unjudged and judged once, by the type the read
//! was asked for.

#[cfg(test)]
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[path = "judged_once_tests/models.rs"]
mod models;

#[cfg(test)]
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[path = "judged_once_tests/tests.rs"]
mod tests;
