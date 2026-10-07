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
//! struct that reaches itself is the one shape whose module used to throw as it was imported, and
//! a union written around a factory's call has to parse what the plain union parses.

#![cfg(all(unix, feature = "serde", feature = "typescript", feature = "zod"))]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

/// What the bounded driver prints: each broken bound under its key in the bound's own words, and
/// a value inside every bound as it was sent.
const BOUNDED: &str = "[[\"later: too long: maximum length is 3, got 4\",\"many.1: too short: \
                       minimum length is 2, got 1\",\"sort: too short: minimum length is 1, got \
                       0\"],[\"sort: does not match pattern '^[a-z.]*$'\"],{\"later\":\"abc\",\
                       \"many\":[\"ab\"],\"sort\":\"a.b\"},{\"later\":\"abc\",\"many\":[]}]";

/// Loads the compiled bundle and parses through the struct whose fields bound a brand.
const BOUNDED_DRIVER: &str = r#"import { DeclaredBounded$Schema } from "./lib/index.js";

const said = (value) => {
  const read = DeclaredBounded$Schema.safeParse(value);
  return read.success
    ? read.data
    : read.error.issues.map((issue) => `${issue.path.join(".")}: ${issue.message}`);
};
console.log(
  JSON.stringify([
    said({ later: "abcd", many: ["ab", "c"], sort: "" }),
    said({ later: "abc", many: [], sort: "A" }),
    said({ later: "abc", many: ["ab"], sort: "a.b" }),
    said({ later: "abc", many: [] }),
  ]),
);
"#;

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

/// Loads the compiled bundle and parses through the types that hold an optional one of
/// themselves: an absent key and a `null` both answer `undefined` under a key that is there, a
/// nested value is read all the way down, and a wrong one is refused at any depth.
const OPTIONAL_DRIVER: &str = r#"import { z } from "zod";
import {
  DeclaredAhead$SchemaFactory,
  DeclaredChain$SchemaFactory,
  DeclaredMaps$SchemaFactory,
} from "./lib/index.js";

const chain = DeclaredChain$SchemaFactory(z.string());
const maps = DeclaredMaps$SchemaFactory(z.string());
const leaf = (id) => ({ held: {}, id, rows: [] });
const mapped = maps.parse({ held: { a: leaf("b") }, id: "a", maybe: null, rows: [{ k: leaf("c") }] });
const ahead = DeclaredAhead$SchemaFactory(z.string());
const absent = chain.parse({ id: "a", named: null });
const nulled = chain.parse({ id: "a", by_key: null, many: null, named: null, next: null });
const nested = {
  id: "a",
  by_key: { k: { id: "d", named: null } },
  many: [{ id: "c", named: {} }],
  named: { n: { id: "e", named: null } },
  next: { id: "b", named: null, next: { id: "f", named: null } },
};
console.log(
  JSON.stringify([
    "next" in absent && absent.next === undefined && "by_key" in absent && "many" in absent,
    nulled.next === undefined && nulled.many === undefined && nulled.by_key === undefined,
    chain.safeParse(nested).success,
    chain.safeParse({ id: "a", named: null, next: { id: 5, named: null } }).success,
    chain.safeParse({ id: "a", named: null, by_key: { k: 5 } }).success,
    chain.safeParse({ id: "a", named: { n: 5 } }).success,
    ahead.safeParse({
      id: "a",
      behind: [{ id: "f" }, { id: "g", ahead: { id: "b", behind: [] } }],
    }).success,
    ahead.safeParse({ id: "a", behind: [{ id: "f", ahead: 5 }] }).success,
    "maybe" in mapped && mapped.maybe === undefined && mapped.held.a.id === "b",
    maps.safeParse({ held: {}, id: "a", maybe: { m: leaf("d") }, rows: [] }).success,
    maps.safeParse({ held: { a: leaf(5) }, id: "a", rows: [] }).success,
    maps.safeParse({ held: {}, id: "a", maybe: { m: leaf(5) }, rows: [] }).success,
    maps.safeParse({ held: {}, id: "a", rows: [{ k: leaf(5) }] }).success,
  ]),
);
"#;

/// What that driver prints.
const OPTIONAL_LOADED: &str =
    "[true,true,true,false,false,false,true,false,true,true,false,false,false]";

const NODE_VAR: &str = "TIXSCHEMA_NODE";

/// The second package: it names the first only through `./lib`, where the declarations are.
const CONSUMER: &str = r#"import {
  type DeclaredAhead,
  DeclaredAhead$SchemaFactory,
  type DeclaredChain,
  DeclaredChain$SchemaFactory,
  type DeclaredChoice,
  DeclaredChoice$SchemaFactory,
  DeclaredHolder$SchemaFactory,
  type DeclaredMaps,
  DeclaredMaps$SchemaDefault,
  DeclaredMaps$SchemaFactory,
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

const chain = DeclaredChain$SchemaFactory(z.string());

export function nextFromFactory(parsed: z.infer<typeof chain>): DeclaredChain<string> | undefined {
  return parsed.next;
}

export function keyedFromFactory(
  parsed: z.infer<typeof chain>
): Partial<Record<string, DeclaredChain<string>>> | undefined {
  return parsed.by_key;
}

const maps = DeclaredMaps$SchemaFactory(z.string());

export function heldMapFromFactory(
  parsed: z.infer<typeof maps>
): Partial<Record<string, DeclaredMaps<string>>> {
  return parsed.held;
}

export function maybeMapFromDefault(
  parsed: z.infer<typeof DeclaredMaps$SchemaDefault>
): Partial<Record<string, DeclaredMaps<string>>> | undefined {
  return parsed.maybe;
}

export function rowsFromFactory(
  parsed: z.infer<typeof maps>
): Partial<Record<string, DeclaredMaps<string>>>[] {
  return parsed.rows;
}

const ahead = DeclaredAhead$SchemaFactory(z.string());

export function aheadFromFactory(
  parsed: z.infer<typeof ahead>
): (DeclaredAhead<string> | undefined)[] {
  return parsed.behind.map((held) => held.ahead);
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

/// A struct whose fields bound a brand over a string: one declared below it, a list of one
/// declared above, and an optional one.
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclaredBounded {
    #[model_schema_prop(maxLength = 3)]
    pub later: DeclaredLabel,
    #[model_schema_prop(minLength = 2)]
    pub many: Vec<DeclaredPath>,
    #[model_schema_prop(minLength = 1, pattern = "^[a-z.]*$")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort: Option<DeclaredPath>,
}

#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DeclaredLabel(pub String);

#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DeclaredPath(pub String);

/// One of a cycle of two, declared above the type its list holds.
#[model_schema(default_types(IdType = String))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclaredAhead<IdType> {
    pub behind: Vec<DeclaredBehind<IdType>>,
    pub id: IdType,
}

/// The other of that cycle, holding an optional one of the type declared above it.
#[model_schema(default_types(IdType = String))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclaredBehind<IdType> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ahead: Option<Box<DeclaredAhead<IdType>>>,
    pub id: IdType,
}

/// A generic type holding an optional one of itself, an optional list and an optional map of
/// itself, and a map of itself that is `null` where there is none.
#[model_schema(default_types(IdType = String))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclaredChain<IdType> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by_key: Option<HashMap<String, Self>>,
    pub id: IdType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub many: Option<Vec<Self>>,
    #[model_schema_prop(nullable)]
    pub named: Option<HashMap<String, Self>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<Box<Self>>,
}

/// A generic type holding a map of itself, an optional map of itself below it, and a list of maps
/// of itself: the three a builder cannot read its own factory's type for.
#[model_schema(default_types(IdType = String))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclaredMaps<IdType> {
    pub held: HashMap<String, Self>,
    pub id: IdType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maybe: Option<HashMap<String, Self>>,
    pub rows: Vec<HashMap<String, Self>>,
}

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
        // `z` bound as a constant, which names no type: the weakest binding a consumer's own
        // module may hand the bundle.
        "import * as zod from \"zod\";\nimport type { ZodType, $brand } from \"zod\";\nimport type { \
         SomeType } from \"zod/v4/core\";\n\nconst z = zod;"
            .to_owned(),
        DeclaredPath::ts_definition(),
        DeclaredPath::zod_schema(),
        DeclaredBounded::ts_definition(),
        DeclaredBounded::zod_schema(),
        DeclaredLabel::ts_definition(),
        DeclaredLabel::zod_schema(),
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
        DeclaredChain::<String>::ts_definition(),
        DeclaredChain::<String>::zod_schema(),
        DeclaredMaps::<String>::ts_definition(),
        DeclaredMaps::<String>::zod_schema(),
        DeclaredAhead::<String>::ts_definition(),
        DeclaredAhead::<String>::zod_schema(),
        DeclaredBehind::<String>::ts_definition(),
        DeclaredBehind::<String>::zod_schema(),
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

/// What `driver` prints once the bundle built for the check `named` is loaded under `node`, with
/// what it wrote to its stderr, or `None` where the bundle or the runtime is not reachable.
fn loaded(named: &str, driver: &str) -> Option<(String, String)> {
    let at = built(named)?;
    fs::write(at.join("run.mjs"), driver).unwrap();
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
        return None;
    };
    Some((
        String::from_utf8_lossy(&ran.stdout).trim().to_owned(),
        String::from_utf8_lossy(&ran.stderr).into_owned(),
    ))
}

#[test]
fn type_check_a_generic_that_holds_an_optional_one_of_itself_loads_and_parses() {
    let Some((printed, failed)) = loaded("optional", OPTIONAL_DRIVER) else {
        return;
    };
    assert_eq!(
        printed, OPTIONAL_LOADED,
        "the bundle does not load and parse under node:\n{failed}"
    );
}

#[test]
fn type_check_a_tuple_struct_that_reaches_itself_loads_and_parses() {
    let Some((printed, failed)) = loaded("loaded", DRIVER) else {
        return;
    };
    assert_eq!(
        printed, LOADED,
        "the bundle does not load and parse under node:\n{failed}"
    );
}

/// A bound written on a field typed with a brand reaches the brand's schema as a check: the bundle
/// compiles under the compiler, and zod refuses what breaks it in the bound's own words.
#[test]
fn type_check_a_bound_on_a_field_typed_with_a_brand_loads_and_parses() {
    let Some((printed, failed)) = loaded("bounded", BOUNDED_DRIVER) else {
        return;
    };
    assert_eq!(
        printed, BOUNDED,
        "the bundle does not load and parse under node:\n{failed}"
    );
}

/// The union is built over a type parameter, and the factory's call is the argument: the text
/// the README shows for the member.
#[test]
fn a_union_around_a_factorys_call_is_built_over_a_type_parameter() {
    let zod = DeclaredChain::<String>::zod_schema();
    for member in [
        "  get next() { return (<Reached$ extends SomeType>(reached$: Reached$) => \
         z.union([z.null().transform(() => undefined), reached$, \
         z.undefined()]).prefault(undefined))(DeclaredChain$SchemaFactory(idType)); },",
        "  get many() { return (<Reached$ extends SomeType>(reached$: Reached$) => \
         z.union([z.null().transform(() => undefined), z.array(reached$), \
         z.undefined()]).prefault(undefined))(DeclaredChain$SchemaFactory(idType)); },",
        "  get named() { return (<Reached$ extends SomeType>(reached$: Reached$) => \
         z.union([z.record(z.string(), reached$), \
         z.null()]))(DeclaredChain$SchemaSelf(idType)); },",
    ] {
        assert!(zod.contains(member), "missing `{member}` in: {zod}");
    }
}

/// A map reads its own type off what it holds, which is the builder's own return type where it
/// holds the type itself: the builder states that type through the self view instead.
#[test]
fn a_map_of_the_type_itself_is_read_through_the_self_view() {
    let zod = DeclaredMaps::<String>::zod_schema();
    for written in [
        "function DeclaredMaps$SchemaSelf<IdType extends ZodType>(\n  idType: IdType,\n): \
         ZodType<DeclaredMaps<IdType[\"_zod\"][\"output\"]>>;",
        "  get held() { return z.record(z.string(), DeclaredMaps$SchemaSelf(idType)); },",
        "  get maybe() { return (<Reached$ extends SomeType>(reached$: Reached$) => \
         z.union([z.null().transform(() => undefined), z.record(z.string(), reached$), \
         z.undefined()]).prefault(undefined))(DeclaredMaps$SchemaSelf(idType)); },",
        "  get rows() { return z.array(z.record(z.string(), DeclaredMaps$SchemaSelf(idType))); },",
    ] {
        assert!(zod.contains(written), "missing `{written}` in: {zod}");
    }
    let node = DeclaredNode::<String>::zod_schema();
    assert!(
        node.contains("  get children() { return z.array(DeclaredNode$SchemaFactory(idType)); },")
            && !node.contains("$SchemaSelf"),
        "a list of the type itself keeps the factory's own type. Got: {node}"
    );
}
