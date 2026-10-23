//! The text of a Kotlin definition without the serializers that read a number or a `bool`, for a
//! test of another part of the definition: each is declared once per item under a counter that
//! runs across the items of a crate, and named on every member that holds one.

use regex::Regex;

/// `kotlin` without the `KotlinNumber{N}` objects and the annotations that name them.
pub fn without_number_readers(kotlin: &str) -> String {
    let objects = Regex::new(
        r"object KotlinNumber\d+ : KSerializer<\w+> \{.*?decodeFromJsonElement\(held, element\) \} \} ?",
    );
    let annotations = Regex::new(r"@Serializable\(with = KotlinNumber\d+::class\) ");
    if let (Ok(object_pattern), Ok(annotation_pattern)) = (&objects, &annotations) {
        let bare = object_pattern.replace_all(kotlin, "");
        annotation_pattern.replace_all(&bare, "").into_owned()
    } else {
        kotlin.to_owned()
    }
}
