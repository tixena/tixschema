//! The skills of an `mcp` service: the folder `mcp(skills = "...")` names, read at expansion with
//! each file's digest and size, and the rules a skill's name and frontmatter follow.

use core::fmt::Write as _;
use core::iter::once;
use core::str::{Utf8Error, from_utf8};
use sha2::{Digest as _, Sha256};
use std::env::var_os;
use std::fs;
use std::path::{Path, PathBuf};
use syn::LitStr;

const DESCRIPTION_LIMIT: usize = 1024;
const NAME_LIMIT: usize = 64;
const MANIFEST: &str = "SKILL.md";

/// Every skill of the folder, by name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkillsFolder {
    /// The folder as written, without a trailing `/`.
    pub folder: String,
    pub skills: Vec<FolderSkill>,
}

impl SkillsFolder {
    /// Where a file is, relative to the crate's manifest.
    pub fn source(&self, skill: &str, path: &str) -> String {
        format!("{}/{skill}/{path}", self.folder)
    }
}

/// One directory of the folder: its `SKILL.md` frontmatter and its files, `SKILL.md` first.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FolderSkill {
    pub files: Vec<SkillFile>,
    pub frontmatter: Vec<(String, String)>,
    pub name: String,
}

/// One file of a skill.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkillFile {
    /// `sha256:` and the lowercase hex of the SHA-256 of the bytes.
    pub digest: String,
    pub mime: String,
    /// Where the file is, relative to the folder the service names, with `/` between parts.
    pub path: String,
    pub size: usize,
    /// Whether the bytes are UTF-8 text, which a read answers as `text` rather than `blob`.
    pub text: bool,
}

impl FolderSkill {
    /// The URI of the skill's `SKILL.md`.
    pub fn uri(&self) -> String {
        format!("skill://{}/{MANIFEST}", self.name)
    }
}

impl SkillFile {
    /// The URI a read names the file by.
    pub fn uri(&self, skill: &str) -> String {
        format!("skill://{skill}/{}", self.path)
    }
}

/// Why `name` is no skill name, or `None` where it is one: 1 to 64 characters of `a-z`, `0-9` and
/// `-`, with no `-` at either end and no two together.
pub fn name_problem(name: &str) -> Option<&'static str> {
    if name.is_empty() || name.chars().count() > NAME_LIMIT {
        return Some("is not 1 to 64 characters long");
    }
    if !name
        .chars()
        .all(|letter| letter.is_ascii_lowercase() || letter.is_ascii_digit() || letter == '-')
    {
        return Some("holds a character other than `a-z`, `0-9` and `-`");
    }
    if name.starts_with('-') || name.ends_with('-') {
        return Some("starts or ends with `-`");
    }
    if name.contains("--") {
        return Some("holds two `-` together");
    }
    None
}

/// The frontmatter text a transport writes ahead of a generated `SKILL.md`: the name, written
/// plain where YAML reads it as text, and the description as a JSON string.
pub fn written_frontmatter(name: &str, description: &str) -> String {
    let reads_as_text = !name.starts_with(|letter: char| letter.is_ascii_digit())
        && !matches!(
            name,
            "null" | "true" | "false" | "yes" | "no" | "on" | "off" | "y" | "n"
        );
    let written_name = if reads_as_text {
        name.to_owned()
    } else {
        json_string(name)
    };
    format!(
        "---\nname: {written_name}\ndescription: {}\n---\n\n",
        json_string(description)
    )
}

/// `text` as a JSON string, which YAML 1.2 reads as the same text.
pub fn json_string(text: &str) -> String {
    let mut written = String::from("\"");
    for letter in text.chars() {
        match letter {
            '"' => written.push_str("\\\""),
            '\\' => written.push_str("\\\\"),
            '\n' => written.push_str("\\n"),
            '\r' => written.push_str("\\r"),
            '\t' => written.push_str("\\t"),
            control if u32::from(control) < 0x20 => {
                let _ = write!(written, "\\u{:04x}", u32::from(control));
            }
            other => written.push(other),
        }
    }
    written.push('"');
    written
}

/// Reads the folder `named` writes, relative to the manifest of the crate being compiled.
pub fn read_folder(named: &LitStr) -> Result<SkillsFolder, syn::Error> {
    let refuse = |message: String| syn::Error::new(named.span(), message);
    let written = named.value();
    if written.is_empty() || Path::new(&written).is_absolute() {
        return Err(refuse(
            "service_schema: `mcp(skills = \"...\")` names a folder relative to the crate's \
             `Cargo.toml`\n       \
             write `mcp(skills = \"skills\")`"
                .to_owned(),
        ));
    }
    let root = var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .ok_or_else(|| {
            refuse(
                "service_schema: `mcp(skills = \"...\")` is read relative to the crate's manifest, \
                 and cargo did not say where it is"
                    .to_owned(),
            )
        })?
        .join(&written);
    let mut directories: Vec<_> = fs::read_dir(&root)
        .map_err(|error| {
            refuse(format!(
                "service_schema: `mcp(skills = \"{written}\")` names a folder that cannot be read: \
                 {error}\n       \
                 the folder is relative to the crate's `Cargo.toml`, and holds one directory per \
                 skill"
            ))
        })?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_type()
                .is_ok_and(|kind| kind.is_dir() || kind.is_symlink())
        })
        .collect();
    directories.sort_by_key(fs::DirEntry::file_name);
    let mut skills = Vec::new();
    for directory in directories {
        let name = directory.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || !directory.path().is_dir() {
            continue;
        }
        skills.push(read_skill(&directory.path(), &name, &written).map_err(&refuse)?);
    }
    if skills.is_empty() {
        return Err(refuse(format!(
            "service_schema: `mcp(skills = \"{written}\")` names a folder that holds no skill\n       \
             write one directory for each skill, each holding a `SKILL.md`"
        )));
    }
    Ok(SkillsFolder {
        folder: written.trim_end_matches('/').to_owned(),
        skills,
    })
}

fn read_skill(directory: &Path, name: &str, folder: &str) -> Result<FolderSkill, String> {
    let located = format!("`{folder}/{name}`");
    if let Some(problem) = name_problem(name) {
        return Err(format!(
            "service_schema: the skill directory {located} is named `{name}`, which {problem}\n       \
             a skill's name is 1 to 64 characters of `a-z`, `0-9` and `-`, with no `-` at either \
             end and no two together"
        ));
    }
    let mut paths = Vec::new();
    files_below(directory, "", &mut paths)?;
    paths.sort();
    let manifest = paths
        .iter()
        .position(|path| path == MANIFEST)
        .ok_or_else(|| format!("service_schema: the skill {located} holds no `{MANIFEST}`"))?;
    let first = paths.remove(manifest);
    paths.insert(0, first);
    let mut files = Vec::new();
    let mut frontmatter = Vec::new();
    for path in paths {
        let bytes = fs::read(directory.join(&path)).map_err(|error| {
            format!("service_schema: `{folder}/{name}/{path}` cannot be read: {error}")
        })?;
        if path == MANIFEST {
            let manifest_text = from_utf8(&bytes).map_err(|unread: Utf8Error| {
                format!("service_schema: `{folder}/{name}/{MANIFEST}` is not UTF-8 text: {unread}")
            })?;
            frontmatter = read_frontmatter(manifest_text).map_err(|problem| {
                format!("service_schema: `{folder}/{name}/{MANIFEST}` {problem}")
            })?;
        }
        let text = from_utf8(&bytes).is_ok();
        files.push(SkillFile {
            digest: format!("sha256:{}", hex(&Sha256::digest(&bytes))),
            mime: mime_of(&path, text),
            path,
            size: bytes.len(),
            text,
        });
    }
    let held = |key: &str| {
        frontmatter
            .iter()
            .find(|(held, _)| held == key)
            .map(|(_, value)| value.as_str())
    };
    if held("name") != Some(name) {
        return Err(format!(
            "service_schema: the frontmatter of `{folder}/{name}/{MANIFEST}` must hold `name: {name}`, \
             the name of its directory"
        ));
    }
    match held("description") {
        Some(text) if !text.is_empty() && text.chars().count() <= DESCRIPTION_LIMIT => {}
        _ => {
            return Err(format!(
                "service_schema: the frontmatter of `{folder}/{name}/{MANIFEST}` must hold a \
                 `description` of 1 to {DESCRIPTION_LIMIT} characters"
            ));
        }
    }
    Ok(FolderSkill {
        files,
        frontmatter,
        name: name.to_owned(),
    })
}

fn files_below(directory: &Path, prefix: &str, into: &mut Vec<String>) -> Result<(), String> {
    let entries = fs::read_dir(directory).map_err(|error| {
        format!(
            "service_schema: `{}` cannot be read: {error}",
            directory.display()
        )
    })?;
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name().into_string().map_err(|unread| {
            format!(
                "service_schema: a file name under `{}` is not UTF-8: {}",
                directory.display(),
                unread.display()
            )
        })?;
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        let relative = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        if path.is_dir() {
            files_below(&path, &relative, into)?;
        } else {
            into.push(relative);
        }
    }
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    let mut written = String::new();
    for byte in bytes {
        let _ = write!(written, "{byte:02x}");
    }
    written
}

fn mime_of(path: &str, text: bool) -> String {
    let extension = path.rsplit_once('.').map_or("", |(_, extension)| extension);
    let known = match extension.to_ascii_lowercase().as_str() {
        "md" | "markdown" => "text/markdown",
        "json" => "application/json",
        "yaml" | "yml" => "application/yaml",
        "html" => "text/html",
        "css" => "text/css",
        "js" | "mjs" => "text/javascript",
        "csv" => "text/csv",
        "xml" => "application/xml",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "pdf" => "application/pdf",
        _ if text => "text/plain",
        _ => "application/octet-stream",
    };
    known.to_owned()
}

/// The `key: value` pairs of the frontmatter `text` opens with. A value is a plain, single-quoted
/// or double-quoted scalar, or a `|` or `>` block.
pub fn read_frontmatter(text: &str) -> Result<Vec<(String, String)>, String> {
    let mut lines = text.strip_prefix('\u{feff}').unwrap_or(text).lines();
    if lines.next().map(str::trim_end) != Some("---") {
        return Err("does not open with a `---` frontmatter".to_owned());
    }
    let mut body: Vec<&str> = Vec::new();
    let mut closed = false;
    for line in lines {
        if line.trim_end() == "---" {
            closed = true;
            break;
        }
        body.push(line);
    }
    if !closed {
        return Err("has a frontmatter that is never closed by `---`".to_owned());
    }
    let mut pairs: Vec<(String, String)> = Vec::new();
    let mut at = 0;
    while at < body.len() {
        let line = body[at];
        at += 1;
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        if line.starts_with([' ', '\t']) {
            return Err(format!(
                "holds a nested or misplaced frontmatter line `{}`",
                line.trim()
            ));
        }
        let (written_key, written_rest) = line
            .split_once(':')
            .ok_or_else(|| format!("holds a frontmatter line that is no `key: value`: `{line}`"))?;
        let key = written_key.trim();
        if key.is_empty() || pairs.iter().any(|(held, _)| held == key) {
            return Err(format!("holds the frontmatter key `{key}` empty or twice"));
        }
        let rest = written_rest.trim();
        let continuation: Vec<&str> = body[at..]
            .iter()
            .copied()
            .take_while(|next| next.is_empty() || next.starts_with([' ', '\t']))
            .collect();
        let value = if rest.starts_with(['|', '>']) {
            at += continuation.len();
            block_scalar(rest, &continuation)
        } else if rest.is_empty() {
            if continuation.iter().any(|next| !next.trim().is_empty()) {
                return Err(format!(
                    "holds a frontmatter value of `{key}` that is no scalar; only scalar values are read"
                ));
            }
            String::new()
        } else {
            let value = quoted_or_plain(rest, key)?;
            if rest.starts_with(['"', '\'']) {
                if continuation.iter().any(|next| !next.trim().is_empty()) {
                    return Err(format!(
                        "holds a quoted value of `{key}` over several lines"
                    ));
                }
                value
            } else {
                at += continuation.len();
                let folded: Vec<&str> = continuation
                    .iter()
                    .map(|next| next.trim())
                    .filter(|next| !next.is_empty())
                    .collect();
                once(value.as_str())
                    .chain(folded)
                    .collect::<Vec<_>>()
                    .join(" ")
            }
        };
        pairs.push((key.to_owned(), value));
    }
    Ok(pairs)
}

fn block_scalar(header: &str, lines: &[&str]) -> String {
    let literal = header.starts_with('|');
    let strip = header.contains('-');
    let indent = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);
    let mut rows: Vec<&str> = lines
        .iter()
        .map(|line| {
            if line.len() >= indent {
                &line[indent..]
            } else {
                ""
            }
        })
        .collect();
    while rows.last().is_some_and(|row| row.trim().is_empty()) {
        rows.pop();
    }
    let mut text = if literal {
        rows.join("\n")
    } else {
        let mut folded = String::new();
        let mut break_pending = false;
        for (position, row) in rows.iter().enumerate() {
            if row.trim().is_empty() {
                folded.push('\n');
                break_pending = false;
            } else {
                if position > 0 && break_pending {
                    folded.push(' ');
                }
                folded.push_str(row.trim_end());
                break_pending = true;
            }
        }
        folded
    };
    if !strip && !text.is_empty() {
        text.push('\n');
    }
    text
}

fn quoted_or_plain(rest: &str, key: &str) -> Result<String, String> {
    if let Some(inner) = rest.strip_prefix('"') {
        let body = inner.strip_suffix('"').ok_or_else(|| {
            format!("holds a double-quoted value of `{key}` that does not end on its line")
        })?;
        return unescaped(body)
            .ok_or_else(|| format!("holds an escape in `{key}` that is not read"));
    }
    if let Some(inner) = rest.strip_prefix('\'') {
        let body = inner.strip_suffix('\'').ok_or_else(|| {
            format!("holds a single-quoted value of `{key}` that does not end on its line")
        })?;
        return Ok(body.replace("''", "'"));
    }
    if rest.starts_with(['[', '{', '&', '*', '!', '%', '@', '`']) {
        return Err(format!(
            "holds a frontmatter value of `{key}` that is no scalar; only scalar values are read"
        ));
    }
    Ok(rest
        .split_once(" #")
        .map_or(rest, |(value, _)| value)
        .trim_end()
        .to_owned())
}

fn unescaped(body: &str) -> Option<String> {
    let mut text = String::new();
    let mut letters = body.chars();
    while let Some(letter) = letters.next() {
        if letter != '\\' {
            text.push(letter);
            continue;
        }
        match letters.next()? {
            '"' => text.push('"'),
            '\\' => text.push('\\'),
            '/' => text.push('/'),
            'n' => text.push('\n'),
            't' => text.push('\t'),
            'r' => text.push('\r'),
            '0' => text.push('\0'),
            ' ' => text.push(' '),
            'u' => {
                let digits: String = letters.by_ref().take(4).collect();
                text.push(char::from_u32(u32::from_str_radix(&digits, 16).ok()?)?);
            }
            _ => return None,
        }
    }
    Some(text)
}
