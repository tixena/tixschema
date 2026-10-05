extern crate alloc;

#[cfg(test)]
#[path = "branded_newtype_tests/named_field.rs"]
mod named_field;

#[cfg(test)]
#[path = "branded_newtype_tests/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "branded_newtype_tests/transparent_key.rs"]
mod transparent_key;
