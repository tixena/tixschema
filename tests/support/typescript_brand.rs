//! The brand a TypeScript valid type carries. Only a build with `zod` writes it: with no schema to
//! make a branded value, a build with no `zod` writes the valid type unbranded.

/// What the valid type `name` writes after `Readonly<{name}Draft>`.
pub fn brand_after(name: &str) -> String {
    if cfg!(feature = "zod") {
        format!(" & $brand<\"{name}\">")
    } else {
        String::new()
    }
}
