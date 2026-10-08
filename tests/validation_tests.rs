extern crate alloc;

#[cfg(test)]
#[path = "validation_tests/borrowed_text.rs"]
mod borrowed_text;

#[cfg(test)]
#[path = "validation_tests/declared_by_macro.rs"]
mod declared_by_macro;

#[cfg(test)]
#[path = "validation_tests/names_in_scope.rs"]
mod names_in_scope;

#[cfg(test)]
#[path = "validation_tests/tests.rs"]
mod tests;
