//! Generic types that reach themselves, read from another package: through the declaration files
//! a real TypeScript compiler emits for the bundle, which is all a second package ever sees.
//!
//! A declaration file can write a type that reaches itself only by name, so this is the one group
//! that tells a schema type read back under a name from one the compiler has to spell out, and
//! elides to `any` where it recurs. It needs a compiler and `zod` itself: `tsc` on `PATH` or named
//! in `TIXSCHEMA_TSC`, and `zod` under `TIXSCHEMA_NODE_MODULES`'s `node_modules`. Without both it
//! stands down, saying so on the process's own stderr. `just typecheck-ts` refuses to.
//!
//! The same bundle is then loaded under `node` (on `PATH`, or named in `TIXSCHEMA_NODE`): a tuple
//! struct that reaches itself is the one shape whose module used to throw as it was imported.

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

/// What the driver prints: each tuple struct takes a nested value and refuses one that is wrong
/// two levels down.
const LOADED: &str = "[true,false,true,false]";

/// Loads the compiled bundle and parses through the two tuple structs that reach themselves.
const DRIVER: &str = r#"import { z } from "zod";
import { DeclaredPlainSlots$Schema, DeclaredSlots$SchemaFactory } from "./lib/index.js";

const slots = DeclaredSlots$SchemaFactory(z.string());
console.log(
  JSON.stringify([
    DeclaredPlainSlots$Schema.safeParse(["a", [["b", []]]]).success,
    DeclaredPlainSlots$Schema.safeParse(["a", [["b", [7]]]]).success,
    slots.safeParse(["a", [["b", []]]]).success,
    slots.safeParse(["a", [[5, []]]]).success,
  ]),
);
"#;

const MODULES_VAR: &str = "TIXSCHEMA_NODE_MODULES";

const NODE_VAR: &str = "TIXSCHEMA_NODE";

/// The second package: it names the first only through `./lib`, where the declarations are.
const CONSUMER: &str = r#"import {
  type DeclaredChoice,
  DeclaredChoice$SchemaFactory,
  DeclaredHolder$SchemaFactory,
  type DeclaredNode,
  DeclaredNode$SchemaDefault,
  DeclaredNode$SchemaFactory,
  type DeclaredSlots,
  DeclaredSlots$SchemaFactory,
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

const slots = DeclaredSlots$SchemaFactory(z.string());

export function slotsFromFactory(parsed: z.infer<typeof slots>): DeclaredSlots<string>[] {
  return parsed[1];
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

/// A tuple struct whose second slot holds a list of the struct.
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclaredPlainSlots(pub String, pub Vec<Self>);

/// The same under a type parameter.
#[model_schema(default_types(IdType = String))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclaredSlots<IdType>(pub IdType, pub Vec<Self>);

fn bundle() -> String {
    [
        "import { z, type ZodType } from \"zod\";".to_owned(),
        DeclaredNode::<String>::ts_definition(),
        DeclaredNode::<String>::zod_schema(),
        DeclaredHolder::<String>::ts_definition(),
        DeclaredHolder::<String>::zod_schema(),
        DeclaredChoice::<String>::ts_definition(),
        DeclaredChoice::<String>::zod_schema(),
        DeclaredPlainSlots::ts_definition(),
        DeclaredPlainSlots::zod_schema(),
        DeclaredSlots::<String>::ts_definition(),
        DeclaredSlots::<String>::zod_schema(),
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
            "\ntixschema: no TypeScript compiler, no `zod` or no `node` is reachable, so the \
             bundle of types that reach themselves was NOT compiled or loaded.\n  Put `tsc` and \
             `node` on PATH or name them in {COMPILER_VAR} and {NODE_VAR}, name in {MODULES_VAR} \
             a directory whose `node_modules` holds `zod`, and run `just typecheck-ts`, which \
             refuses to stand down.\n\n"
        );
        drop(stderr().write_all(notice.as_bytes()));
    });
}

/// The first package in a directory of its own for the check `named`, compiled with its
/// declarations, or `None` where the compiler or `zod` is not reachable.
fn built(named: &str) -> Option<PathBuf> {
    let Some(modules) = node_modules() else {
        stand_down();
        return None;
    };
    let at = temp_dir().join(format!("tixschema-declarations-{named}-{run}", run = id()));
    if at.exists() {
        fs::remove_dir_all(&at).unwrap();
    }
    fs::create_dir_all(at.join("src")).unwrap();
    symlink(&modules, at.join("node_modules")).unwrap();
    fs::write(at.join("package.json"), PACKAGE).unwrap();
    fs::write(at.join("tsconfig.json"), PROJECT).unwrap();
    fs::write(at.join("src/index.ts"), bundle()).unwrap();

    let Some((emitted, said)) = compiled(&at, "tsconfig.json") else {
        fs::remove_dir_all(&at).unwrap();
        stand_down();
        return None;
    };
    assert!(emitted, "the bundle does not compile:\n{said}");
    Some(at)
}

#[test]
fn type_check_a_recursive_generic_keeps_its_type_in_another_package() {
    let Some(at) = built("consumer") else {
        return;
    };
    fs::write(at.join("tsconfig.consumer.json"), CONSUMER_PROJECT).unwrap();
    fs::write(at.join("consumer.ts"), CONSUMER).unwrap();
    let declarations = fs::read_to_string(at.join("lib/index.d.ts")).unwrap();
    let (accepted, refused) = compiled(&at, "tsconfig.consumer.json").unwrap();
    fs::remove_dir_all(&at).unwrap();
    assert!(
        accepted,
        "a package reading the declarations loses a type:\n{refused}\n\nThe declarations:\n\
         {declarations}"
    );
}

#[test]
fn type_check_a_tuple_struct_that_reaches_itself_loads_and_parses() {
    let Some(at) = built("loaded") else {
        return;
    };
    fs::write(at.join("run.mjs"), DRIVER).unwrap();
    let named_node = env::var(NODE_VAR).ok();
    let node = named_node.clone().unwrap_or_else(|| "node".to_owned());
    let run = Command::new(&node).arg("run.mjs").current_dir(&at).output();
    fs::remove_dir_all(&at).unwrap();
    let Ok(ran) = run else {
        assert!(
            named_node.is_none(),
            "{NODE_VAR} names `{node}`, and no runtime could be started there: {}",
            run.unwrap_err()
        );
        stand_down();
        return;
    };
    assert_eq!(
        String::from_utf8_lossy(&ran.stdout).trim(),
        LOADED,
        "the bundle does not load and parse under node:\n{}",
        String::from_utf8_lossy(&ran.stderr)
    );
}
