//! Generic types that reach themselves, read from another package: through the declaration files
//! a real TypeScript compiler emits for the bundle, which is all a second package ever sees.
//!
//! A declaration file can write a type that reaches itself only by name, so this is the one group
//! that tells a schema type read back under a name from one the compiler has to spell out, and
//! elides to `any` where it recurs. It needs a compiler and `zod` itself: `tsc` on `PATH` or named
//! in `TIXSCHEMA_TSC`, and `zod` under `TIXSCHEMA_NODE_MODULES`'s `node_modules`. Without both it
//! stands down, saying so on the process's own stderr. `just typecheck-ts` refuses to.

#![cfg(all(unix, feature = "serde", feature = "typescript", feature = "zod"))]

use serde::{Deserialize, Serialize};
use std::env;
use std::env::temp_dir;
use std::fs;
use std::io::Write as _;
use std::io::stderr;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, id};
use std::sync::Once;
use tixschema::model_schema;

const COMPILER_VAR: &str = "TIXSCHEMA_TSC";

const MODULES_VAR: &str = "TIXSCHEMA_NODE_MODULES";

/// The second package: it names the first only through `./lib`, where the declarations are.
const CONSUMER: &str = r#"import {
  type DeclaredChoice,
  DeclaredChoice$SchemaFactory,
  DeclaredHolder$SchemaFactory,
  type DeclaredNode,
  DeclaredNode$SchemaDefault,
  DeclaredNode$SchemaFactory,
} from "./lib/index.js";
import { z } from "zod";

const node = DeclaredNode$SchemaFactory(z.string());

export function childrenFromFactory(parsed: z.infer<typeof node>): DeclaredNode<string>[] {
  return parsed.children;
}

export function childrenFromDefault(
  parsed: z.infer<typeof DeclaredNode$SchemaDefault>
): DeclaredNode<string>[] {
  return parsed.children;
}

const holder = DeclaredHolder$SchemaFactory(z.string());

export function heldFromFactory(parsed: z.infer<typeof holder>): DeclaredNode<string>[] {
  return parsed.nodes;
}

const choice = DeclaredChoice$SchemaFactory(z.string());

export function chosenFromFactory(parsed: z.infer<typeof choice>): DeclaredChoice<string> {
  return parsed;
}
"#;

const CONSUMER_PROJECT: &str = r#"{
  "compilerOptions": {
    "strict": true,
    "module": "nodenext",
    "moduleResolution": "nodenext",
    "target": "ES2022",
    "skipLibCheck": true,
    "noEmit": true
  },
  "files": ["consumer.ts"]
}
"#;

const PACKAGE: &str = r#"{ "type": "module", "private": true }
"#;

/// The first package: the bundle, compiled with declarations into `lib`.
const PROJECT: &str = r#"{
  "compilerOptions": {
    "strict": true,
    "declaration": true,
    "module": "nodenext",
    "moduleResolution": "nodenext",
    "target": "ES2022",
    "skipLibCheck": true,
    "rootDir": "src",
    "outDir": "lib"
  },
  "files": ["src/index.ts"]
}
"#;

static STOOD_DOWN: Once = Once::new();

/// An internally tagged enum whose variant holds a list of the enum.
#[model_schema(default_types(IdType = String))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum DeclaredChoice<IdType> {
    Group { items: Vec<Self> },
    Leaf { id: IdType },
}

/// A generic type that embeds the recursive one.
#[model_schema(default_types(IdType = String))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclaredHolder<IdType> {
    pub nodes: Vec<DeclaredNode<IdType>>,
}

#[model_schema(default_types(IdType = String))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclaredNode<IdType> {
    pub children: Vec<Self>,
    pub id: IdType,
}

fn bundle() -> String {
    [
        "import { z, type ZodType } from \"zod\";".to_owned(),
        DeclaredNode::<String>::ts_definition(),
        DeclaredNode::<String>::zod_schema(),
        DeclaredHolder::<String>::ts_definition(),
        DeclaredHolder::<String>::zod_schema(),
        DeclaredChoice::<String>::ts_definition(),
        DeclaredChoice::<String>::zod_schema(),
    ]
    .join("\n\n")
}

/// Runs the compiler over the project file `project` names in `at`, and answers whether it
/// accepted it and everything it reported, or `None` where no compiler could be started.
fn compiled(at: &Path, project: &str) -> Option<(bool, String)> {
    let named_compiler = env::var(COMPILER_VAR).ok();
    let compiler = named_compiler.clone().unwrap_or_else(|| "tsc".to_owned());
    let run = Command::new(&compiler)
        .args(["--pretty", "false", "-p", project])
        .current_dir(at)
        .output();
    let Ok(reported) = run else {
        assert!(
            named_compiler.is_none(),
            "{COMPILER_VAR} names `{compiler}`, and no compiler could be started there: {}",
            run.unwrap_err()
        );
        return None;
    };
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&reported.stdout),
        String::from_utf8_lossy(&reported.stderr)
    );
    Some((reported.status.success(), said))
}

/// The `node_modules` that holds `zod`, or `None` where the variable names none. One it names
/// without `zod` in it is a failure: somebody said where it was.
fn node_modules() -> Option<PathBuf> {
    let at = PathBuf::from(env::var(MODULES_VAR).ok()?).join("node_modules");
    assert!(
        at.join("zod").is_dir(),
        "{MODULES_VAR} names a directory whose `node_modules` holds no `zod`: {}",
        at.display()
    );
    Some(at)
}

/// Said on the process's own stderr, which `cargo test` does not capture: a stand-down is a pass
/// that proved nothing, so it has to show on a run where everything passed.
fn stand_down() {
    STOOD_DOWN.call_once(|| {
        let notice = format!(
            "\ntixschema: no TypeScript compiler or no `zod` is reachable, so the declarations of \
             a generic type that reaches itself were NOT compiled.\n  Put `tsc` on PATH or name \
             one in {COMPILER_VAR}, name in {MODULES_VAR} a directory whose `node_modules` holds \
             `zod`, and run `just typecheck-ts`, which refuses to stand down.\n\n"
        );
        drop(stderr().write_all(notice.as_bytes()));
    });
}

fn workspace() -> PathBuf {
    let at = temp_dir().join(format!("tixschema-declarations-{run}", run = id()));
    if at.exists() {
        fs::remove_dir_all(&at).unwrap();
    }
    fs::create_dir_all(at.join("src")).unwrap();
    at
}

#[test]
fn type_check_a_recursive_generic_keeps_its_type_in_another_package() {
    let Some(modules) = node_modules() else {
        stand_down();
        return;
    };
    let at = workspace();
    symlink(&modules, at.join("node_modules")).unwrap();
    fs::write(at.join("package.json"), PACKAGE).unwrap();
    fs::write(at.join("tsconfig.json"), PROJECT).unwrap();
    fs::write(at.join("tsconfig.consumer.json"), CONSUMER_PROJECT).unwrap();
    fs::write(at.join("src/index.ts"), bundle()).unwrap();
    fs::write(at.join("consumer.ts"), CONSUMER).unwrap();

    let Some((emitted, said)) = compiled(&at, "tsconfig.json") else {
        fs::remove_dir_all(&at).unwrap();
        stand_down();
        return;
    };
    assert!(emitted, "the bundle does not compile:\n{said}");
    let declarations = fs::read_to_string(at.join("lib/index.d.ts")).unwrap();
    let (accepted, refused) = compiled(&at, "tsconfig.consumer.json").unwrap();
    fs::remove_dir_all(&at).unwrap();
    assert!(
        accepted,
        "a package reading the declarations loses a type:\n{refused}\n\nThe declarations:\n\
         {declarations}"
    );
}
