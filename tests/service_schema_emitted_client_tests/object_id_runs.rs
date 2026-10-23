//! What the client runs over an `ObjectId` share: the `ObjectId` README.md's "MongoDB ObjectId
//! Support" section has the consumer declare in each client's language, which the runs compile
//! beside the emitted types, and what a run prints of each payload.

use super::object_id_models::{Told, serde_told};

/// The fenced block of `language` in the section, as the README writes it.
pub fn readme_object_id(language: &str) -> String {
    let readme = include_str!("../../README.md");
    let section = &readme[readme.find("## MongoDB ObjectId Support").unwrap()..];
    let fence = format!("```{language}\n");
    let start = section.find(&fence).unwrap() + fence.len();
    let length = section[start..].find("```").unwrap();
    section[start..start + length].to_owned()
}

/// What a run prints of the payload `json` read as `read_as` where the client answers `told`:
/// `read` and the value written back, or `refused`.
pub fn printed(told: Told, read_as: &str, json: &str) -> String {
    match told {
        Told::Refused => "refused".to_owned(),
        Told::Serde => serde_told(read_as, json),
    }
}
