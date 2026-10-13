//! The test binary of `validation_tests/`.

extern crate alloc;

#[cfg(test)]
#[path = "validation_tests/borrowed_text.rs"]
mod borrowed_text;

#[cfg(test)]
#[path = "validation_tests/declared_by_macro.rs"]
mod declared_by_macro;

#[cfg(test)]
#[path = "validation_tests/list_bounds.rs"]
mod list_bounds;

#[cfg(test)]
#[path = "validation_tests/names_in_scope.rs"]
mod names_in_scope;

#[cfg(test)]
#[path = "validation_tests/payload_paths.rs"]
mod payload_paths;

#[cfg(test)]
#[path = "validation_tests/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "validation_tests/tuple_struct_slots.rs"]
mod tuple_struct_slots;
