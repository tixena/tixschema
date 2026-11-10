//! The skills of an `mcp` service: the rules of a name, the frontmatter of a `SKILL.md`, the
//! folder read at expansion, and what an operation that writes a skill is refused for.

#![cfg(feature = "serde")]

use super::exec_service_schema;
use super::mcp_refusals::refusals;
use super::parse::{ServiceDef, parse_service};
use super::skills::{
    FolderSkill, name_problem, read_folder, read_frontmatter, written_frontmatter,
};
use super::transport::parse_arguments;
use proc_macro2::TokenStream;
use quote::ToTokens as _;
use syn::{ItemTrait, LitStr};

fn folder(path: &str) -> LitStr {
    LitStr::new(path, proc_macro2::Span::call_site())
}

fn read_error(path: &str) -> String {
    read_folder(&folder(path)).unwrap_err().to_string()
}

fn pairs(text: &str) -> Vec<(String, String)> {
    read_frontmatter(text).unwrap()
}

fn service(source: &str) -> ServiceDef {
    parse_service(&syn::parse_str::<ItemTrait>(source).unwrap()).unwrap()
}

fn refused(source: &str) -> Vec<String> {
    refusals(&service(source)).map_or_else(Vec::new, |refusal| {
        refusal.into_iter().map(|one| one.to_string()).collect()
    })
}

fn skill_service(operation: &str) -> String {
    format!(
        "pub trait DocumentService<Ctx> {{
            /// Writes the release notes.
            {operation}
        }}"
    )
}

fn expansion(arguments: &str, source: &str) -> String {
    let parsed: TokenStream = arguments.parse().unwrap();
    exec_service_schema(
        parsed,
        syn::parse_str::<ItemTrait>(source)
            .unwrap()
            .to_token_stream(),
    )
    .to_string()
}

#[test]
fn a_name_is_one_to_sixty_four_of_lowercase_digits_and_single_inner_hyphens() {
    for name in ["a", "release-notes", "r2-d2", &"a".repeat(64)] {
        assert_eq!(name_problem(name), None, "{name}");
    }
    for name in [
        "",
        "Release_Notes",
        "release notes",
        "-release",
        "release-",
        "release--notes",
        &"a".repeat(65),
    ] {
        assert!(name_problem(name).is_some(), "{name}");
    }
}

#[test]
fn the_frontmatter_a_transport_writes_is_plain_where_yaml_reads_text_and_quoted_where_it_does_not()
{
    assert_eq!(
        written_frontmatter("release-notes", "Say \"what changed\"."),
        "---\nname: release-notes\ndescription: \"Say \\\"what changed\\\".\"\n---\n\n"
    );
    for name in ["123", "true", "null", "no", "1e3"] {
        assert!(
            written_frontmatter(name, "x").starts_with(&format!("---\nname: \"{name}\"\n")),
            "{name}"
        );
    }
}

#[test]
fn a_frontmatter_reads_plain_quoted_and_block_scalars() {
    let read = pairs(
        "---\nname: pack\ndescription: >\n  Pack a parcel:\n  weigh it.\nlicense: 'M''IT'\nnote: \"a\\tb\"\nflat: one\n  two\nkeep: |-\n  x\n  y\n---\nbody\n",
    );
    let held = |key: &str| {
        read.iter()
            .find(|(held, _)| held == key)
            .map(|(_, value)| value.as_str())
    };
    assert_eq!(held("name"), Some("pack"));
    assert_eq!(held("description"), Some("Pack a parcel: weigh it.\n"));
    assert_eq!(held("license"), Some("M'IT"));
    assert_eq!(held("note"), Some("a\tb"));
    assert_eq!(held("flat"), Some("one two"));
    assert_eq!(held("keep"), Some("x\ny"));
}

#[test]
fn a_frontmatter_that_is_not_flat_scalars_is_refused() {
    for (text, said) in [
        ("no frontmatter", "does not open"),
        ("---\nname: a\n", "never closed"),
        ("---\nname: a\nname: b\n---\n", "empty or twice"),
        ("---\nmetadata:\n  author: x\n---\n", "no scalar"),
        ("---\ntags: [a, b]\n---\n", "no scalar"),
        ("---\nnote: \"open\n---\n", "does not end"),
    ] {
        let refusal = read_frontmatter(text).unwrap_err();
        assert!(refusal.contains(said), "{text}: {refusal}");
    }
}

#[test]
fn the_fixture_folder_is_read_with_each_files_digest_and_size() {
    let read = read_folder(&folder("tests/mcp_skills")).unwrap();
    assert_eq!(read.folder, "tests/mcp_skills");
    let names: Vec<&str> = read
        .skills
        .iter()
        .map(|skill| skill.name.as_str())
        .collect();
    assert_eq!(names, ["code-review", "pack-parcels"]);
    let review: &FolderSkill = &read.skills[0];
    let files: Vec<(&str, usize, bool, &str)> = review
        .files
        .iter()
        .map(|file| (file.path.as_str(), file.size, file.text, file.mime.as_str()))
        .collect();
    assert_eq!(
        files,
        [
            ("SKILL.md", 164, true, "text/markdown"),
            ("assets/seal.bin", 9, false, "application/octet-stream"),
            ("references/checklist.md", 76, true, "text/markdown"),
        ]
    );
    assert_eq!(
        review.files[0].digest,
        "sha256:58e2de02d4cda1450548bf7137c2ff9f1f9c5c5280fa9946179cd3e806d1943d"
    );
    assert_eq!(
        review.frontmatter,
        [
            ("name".to_owned(), "code-review".to_owned()),
            (
                "description".to_owned(),
                "Review code using the team's checklist.".to_owned()
            ),
        ]
    );
    assert_eq!(review.uri(), "skill://code-review/SKILL.md");
    assert_eq!(
        read.source("code-review", "SKILL.md"),
        "tests/mcp_skills/code-review/SKILL.md"
    );
}

#[test]
fn a_folder_that_cannot_be_read_as_skills_is_refused_naming_why() {
    for (path, said) in [
        ("tests/mcp_skills_absent", "cannot be read"),
        ("/tests/mcp_skills", "relative to the crate's `Cargo.toml`"),
        ("", "relative to the crate's `Cargo.toml`"),
        (
            "tests/mcp_skills_refused/no_manifest",
            "holds no `SKILL.md`",
        ),
        (
            "tests/mcp_skills_refused/name_mismatch",
            "must hold `name: code-review`",
        ),
        (
            "tests/mcp_skills_refused/bad_name",
            "holds a character other than",
        ),
        ("tests/mcp_skills_refused/nested", "no scalar"),
        (
            "tests/mcp_skills_refused/no_description",
            "`description` of 1 to 1024",
        ),
    ] {
        let refusal = read_error(path);
        assert!(refusal.contains(said), "{path}: {refusal}");
    }
}

#[test]
fn mcp_skills_is_read_as_an_argument_and_nothing_else_in_mcp_is() {
    let read = parse_arguments(
        r#"transports = ["mcp"], mcp(skills = "skills")"#.parse::<TokenStream>().unwrap(),
    )
    .unwrap();
    assert_eq!(
        read.skills.map(|written| written.value()),
        Some("skills".to_owned())
    );
    for arguments in [
        r#"mcp(prompts = "p")"#,
        r#"mcp(skills = "a", skills = "b")"#,
        "mcp(skills)",
    ] {
        assert!(
            parse_arguments(arguments.parse::<TokenStream>().unwrap()).is_err(),
            "{arguments}"
        );
    }
}

#[test]
fn a_skills_folder_on_a_service_that_does_not_ask_for_mcp_is_refused() {
    let expanded = expansion(
        r#"transports = ["amqp_rpc"], mcp(skills = "tests/mcp_skills")"#,
        &skill_service("async fn count(&self, ctx: &Ctx) -> Result<u32, VersionError>;"),
    );
    assert!(
        expanded.contains("ships skills through the `mcp` transport"),
        "{expanded}"
    );
}

#[test]
fn an_unreadable_skills_folder_stops_the_expansion_with_its_reason() {
    let expanded = expansion(
        r#"transports = ["mcp"], mcp(skills = "tests/mcp_skills_absent")"#,
        &skill_service("async fn count(&self, ctx: &Ctx) -> Result<u32, VersionError>;"),
    );
    assert!(
        expanded.contains("names a folder that cannot be read"),
        "{expanded}"
    );
}

#[test]
fn an_operation_that_writes_a_skill_is_read_with_its_name() {
    let read = service(&skill_service(
        r#"#[service_schema_op(mcp(skill = "release-notes"))]
        async fn release_notes_skill(&self, ctx: &Ctx, path: String) -> Result<String, VersionError>;"#,
    ));
    let operation = &read.operations[0];
    assert_eq!(
        operation.mcp.skill.as_ref().map(|held| held.0.as_str()),
        Some("release-notes")
    );
    assert!(!operation.is_mcp_tool());
    assert_eq!(
        refused(&skill_service(
            r#"#[service_schema_op(mcp(skill = "release-notes"))]
        async fn release_notes_skill(&self, ctx: &Ctx, path: String) -> Result<(Vec<u8>, String), VersionError>;"#,
        )),
        Vec::<String>::new()
    );
}

#[test]
fn every_rule_a_skill_operation_breaks_is_refused_naming_the_operation() {
    let cases: [(&str, &str); 9] = [
        (
            r#"#[service_schema_op(mcp(skill = "Release_Notes"))]
               async fn notes(&self, ctx: &Ctx, path: String) -> Result<String, VersionError>;"#,
            "is the skill `Release_Notes`, which holds a character other than",
        ),
        (
            r#"#[service_schema_op(mcp(skill = "release--notes"))]
               async fn notes(&self, ctx: &Ctx, path: String) -> Result<String, VersionError>;"#,
            "holds two `-` together",
        ),
        (
            r#"#[service_schema_op(mcp(skill = "notes"))]
               async fn notes(&self, ctx: &Ctx, path: String, depth: u32) -> Result<String, VersionError>;"#,
            "takes more than the path of a file",
        ),
        (
            r#"#[service_schema_op(mcp(skill = "notes"))]
               async fn notes(&self, ctx: &Ctx, path: u32) -> Result<String, VersionError>;"#,
            "takes more than the path of a file",
        ),
        (
            r#"#[service_schema_op(mcp(skill = "notes"))]
               async fn notes(&self, ctx: &Ctx, path: String) -> Result<u32, VersionError>;"#,
            "answers a success a skill's file cannot",
        ),
        (
            r#"#[service_schema_op(one_way, mcp(skill = "notes"))]
               async fn notes(&self, ctx: &Ctx, path: String);"#,
            "is one way",
        ),
        (
            r#"#[service_schema_op(http(method = "GET", path = "/notes", header_in("x-ledger" = path)), mcp(skill = "notes"))]
               async fn notes(&self, ctx: &Ctx, path: String) -> Result<String, VersionError>;"#,
            "binds a header",
        ),
        (
            r#"#[service_schema_op(mcp(skill = "notes", scopes = ["documents:read"]))]
               async fn notes(&self, ctx: &Ctx, path: String) -> Result<String, VersionError>;"#,
            "writes `scopes`",
        ),
        (
            r#"#[service_schema_op(mcp(skill = "notes", resource = "guide://notes"))]
               async fn notes(&self, ctx: &Ctx, path: String) -> Result<String, VersionError>;"#,
            "is more than one of",
        ),
    ];
    for (operation, said) in cases {
        let found = refused(&skill_service(operation));
        assert!(
            found
                .iter()
                .any(|one| one.contains(said) && one.contains("operation `notes`")),
            "{said}: {found:?}"
        );
    }
}

#[test]
fn a_description_over_a_thousand_and_twenty_four_characters_is_refused() {
    let long = "x".repeat(1025);
    let source = format!(
        "pub trait DocumentService<Ctx> {{
            /// {long}
            #[service_schema_op(mcp(skill = \"notes\"))]
            async fn notes(&self, ctx: &Ctx, path: String) -> Result<String, VersionError>;
        }}"
    );
    let found = refused(&source);
    assert!(
        found
            .iter()
            .any(|one| one.contains("is over 1024 characters")),
        "{found:?}"
    );
}

#[test]
fn a_skill_name_the_folder_also_holds_or_another_operation_writes_is_refused() {
    let mut read = service(&skill_service(
        r#"#[service_schema_op(mcp(skill = "code-review"))]
        async fn notes(&self, ctx: &Ctx, path: String) -> Result<String, VersionError>;"#,
    ));
    read.skills = Some(read_folder(&folder("tests/mcp_skills")).unwrap());
    let found: Vec<String> = refusals(&read)
        .map(|refusal| refusal.into_iter().map(|one| one.to_string()).collect())
        .unwrap_or_default();
    assert!(
        found
            .iter()
            .any(|one| one.contains("which the skills folder also holds")),
        "{found:?}"
    );

    let twice = refused(
        "pub trait DocumentService<Ctx> {
            /// First.
            #[service_schema_op(mcp(skill = \"notes\"))]
            async fn first(&self, ctx: &Ctx, path: String) -> Result<String, VersionError>;

            /// Second.
            #[service_schema_op(mcp(skill = \"notes\"))]
            async fn second(&self, ctx: &Ctx, path: String) -> Result<String, VersionError>;
        }",
    );
    assert!(
        twice
            .iter()
            .any(|one| one
                .contains("operation `second` is the skill `notes`, which `first` already is")),
        "{twice:?}"
    );
}
