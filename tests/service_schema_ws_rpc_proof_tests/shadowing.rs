//! Unit structs named after standard names a transport's half expands to. Each half imports
//! them, so a half that compiles takes none of them for the standard name. The author's own
//! signatures write `Option`, `String` and `Vec`, which is why those three are not among them.

pub struct Box;
pub struct Clone;
pub struct Default;
pub struct Err;
pub struct None;
pub struct Ok;
pub struct Send;
pub struct Sized;
pub struct Some;
pub struct Sync;
