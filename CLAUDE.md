# CLAUDE.md

**Note**: This project uses [bd (beads)](https://github.com/steveyegge/beads) for issue tracking. Use `bd` commands instead of markdown TODOs. See [AGENTS.md](AGENTS.md) for workflow details.

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This File Holds

Claude Code loads this file whole at the start of every session in this repository, and of the
subagents those sessions launch, so every character here is paid on every request. It holds the
commands, the gate and the rules for writing code here, and nothing else.

- A feature's behaviour, what a surface generates, and how a part of the implementation works go
  in `README.md` (what a user of the crate sees) or in the `docs/` file listed under "Related
  Documentation" (how the implementation does it). Never here.
- A task or a brief that says to update `CLAUDE.md` where a behaviour it states changes means the
  `docs/` file that holds that behaviour. Change this file only when a command, the gate or a rule
  below changes.
- `tests/instruction_file_tests.rs` fails when this file is over 40,000 characters, and when a
  relative link in this file or in `docs/` names a file that does not exist.

## Project Overview

**tixschema** is a Rust procedural macro library that generates TypeScript type definitions and Zod v4 validation schemas from Rust structs and enums. This ensures type safety and consistency between Rust backends and TypeScript frontends in Tixena applications.

## Task Management System

**MANDATORY: Use `bd` (beads) for ALL task tracking.**

- ✅ **DO**: Use `bd` commands for issue/task tracking (`bd create`, `bd ready`, `bd update`, `bd close`)
- ❌ **DO NOT**: Use TodoWrite tool, markdown TODO lists, or any other task tracking system
- 📖 **Reference**: See [AGENTS.md](AGENTS.md) for complete bd workflow and commands

**Quick bd commands:**
```bash
bd ready --json           # Check for available work
bd create "Task" -t task -p 2 --json
bd update <id> --status in_progress --json
bd close <id> --reason "Completed" --json
```

## Essential Commands

### Testing
```bash
# Quick test with default features (recommended for rapid iteration)
just quick
# or
cargo test

# Comprehensive test - every combination of the plain features (run before a release)
just test

# Test specific feature combinations
just test-minimal          # No features
just test-default          # Default features only
just test-named-features   # Key combinations

# Run a specific test by name
just test-name <TEST_NAME>
# or
cargo test <TEST_NAME>

# Run the emitted clients under their own toolchains; refuses to stand down
just test-emitted

# Lint, then run, the tests that wait on a real clock, through test-emitted
just test-slow

# Install the Kotlin jars test-emitted compiles against, into ~/.local/share/tixschema/kotlin-libs
just kotlin-libs

# Run the MongoDB operations against the server TIXSCHEMA_MONGODB_URI names; refuses to stand down
just test-mongodb
```

### Code Quality
```bash
# Check code (runs cargo check + clippy with warnings as errors)
just check

# Format code
just fmt

# Check all feature combinations without running tests
just check-all

# Audit the dependencies this package and bson3/ resolve, each from a lock file generated for it (needs cargo-audit)
just audit
```

### Build & Documentation
```bash
# Build the project
cargo build

# Build and open documentation
just docs-open

# Build docs only
just docs
```

### The Gate (what a change is held to, here and in CI)
```bash
just all
# Runs, with every feature on: one clippy lint, one build, one test run
```

### The Full Run (every few days)
```bash
just all-named     # both packages: default and all-features lints, the format check, the named feature lists
just lint-sets     # clippy over every combination of the feature sets
just test-sets     # tests over every combination of the feature sets
just test          # tests over every combination of the plain features
just test-slow     # the tests that wait on a real clock, with the emitted clients
```

No test the gate runs waits on a real clock: no sleep, no real timer, no wall-clock window. A
test that must compiles only under `cfg(slow_tests)`, which `unexpected_cfgs` declares in both
manifests and only `just test-slow` sets (`--config 'build.rustflags=["--cfg", "slow_tests"]'`);
a compiler cfg, so `--all-features` never turns it on. Every such test is a WebSocket run of the
emitted clients (heartbeats, fixed waits, and polls with a deadline), and the helpers and
imports only they use carry the same cfg. A wait for an event the script itself observes resolves
on that event and holds no deadline, as the attachment runs of `run_node_ws_headers.rs` and
`run_dart_ws_headers.rs` do: a script whose event never comes ends with no output, and the test
fails.

## Critical Development Rules

### 2. Required Derives and Imports

```rust
use tixschema::model_schema;
use serde::{Deserialize, Serialize};

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MyType { ... }
```

### 9. Generated Rust Writes Standard Names by Their Full Path

What the macros emit lands in a module the consumer wrote, where a type of theirs may be named
`Vec`, `String`, `Ok` or `Send`. Nothing inside a `quote!` writes a name the standard prelude
supplies bare: it is `::std::vec::Vec`, `::core::option::Option::Some`,
`::core::result::Result::Ok`, `::core::marker::Send`, `::core::ops::FnOnce`. Every such path starts
with `::`: a module of the consumer's may be named `core` or `std`, and a path written
`core::option::Option` resolves through that module. A path serde reads out of an attribute's text
follows the same rule (`"::core::option::Option::is_none"`). A name inside `#[derive(...)]` stays
bare: a derive resolves in the macro namespace, where no type of the consumer's reaches.

`src/emitted_names_tests.rs` reads the crate's own sources and fails on a bare name, and on a
`core` or `std` path that does not start with `::`; `tests/shadowed_core_module_tests.rs` builds
model types and a service beside a module of each name. A test that
declares a model type named `Vec` or `String` needs a test binary of its own
(`tests/shadowed_vec_tests.rs`): the registry is keyed by ident for the whole crate, so a field
written `Vec` in any later expansion would be read as that type.

## Common Pitfalls

1. **Type name ambiguity** → Ensure Rust type names and TypeScript output names don't collide with built-in types
2. **Missing required derives** → Compilation errors
3. **Non-string HashMap keys** → Compilation errors
4. **Forgetting to add types to entities enum** → Types not included in generated output
5. **Using Zod v3** → Generated schemas use v4 syntax and won't work
6. **Testing without feature combinations** → May break in different feature configurations
7. **Declaring `u64`/`usize` under `swift` or `kotlin`** → Refused at expansion: neither target has a mapping for an unsigned 64-bit or pointer-sized integer. Use `i64`, or `u32` where the range allows. A member serde neither writes nor reads, and a member of a variant it neither writes nor reads, is not refused: serde writes none of it
8. **A member named after a word Dart, Swift or Kotlin reserves** → Written in the form the language accepts, the wire key unchanged: `dart_member` adds a trailing underscore (Dart has no escape), `swift_member` and `kotlin_property_name` write backticks. `swift_property` also declares a field named `validate` as `validate_`, the method every emitted Swift type has. Each list (`DART_RESERVED`, `SWIFT_RESERVED`, `KOTLIN_HARD_KEYWORDS`) holds only words the real compiler refused as a member name, so a name that compiled before is emitted as before. A longer identifier built from a member's name (`wireFor`, `forKeys`) reads it through `swift_bare`/`kotlin_bare`. The service clients name a method, a parameter and a request's member through the same seams (`dart_local`/`dart_field_member`, `swift_member`/`swift_field_member`, `kotlin_name`/`kotlin_property_name`), and every name the service code reads as text goes through `parse::written`, which drops a raw identifier's `r#`; `parse::field_ident` writes it back where a keyword-named field is read in Rust. The TypeScript emitters name a header or part argument through `message::parameter_name` and a placeholder's local through `message::local_name`, which add a trailing underscore to a word `tsc` refuses as a parameter or a local in a module (`REFUSED_AS_A_NAME`, each one measured)
9. **An argument or placeholder named after a local the generated code writes around it** (`path`, `headers`, `body`, `request`, `message`) → Never read as that local, and never a change to what a caller writes. The Rust transports hold a header or part argument in `parse::argument_local` (`argument_{name}`), bound there by a dispatcher and moved there by a client's method ahead of its own locals, and a path placeholder in `placeholder_{name}`. Each client emitter holds a `TAKEN_BY_A_METHOD` list of the locals and parameters its methods write: TypeScript (`message::parameter_name`) and Dart (`dart_parameter`) move a positional argument's name, Swift keeps the label and moves the name the method reads it by (`swift_parameter_declared`), Kotlin keeps the parameter and copies it first (`held_copies`, `kotlin_held`). A local added to an emitted method goes into that emitter's list

## Related Documentation

- [README.md](README.md) - User guide, examples, troubleshooting
- [justfile](justfile) - All available commands with descriptions
- [docs/architecture.md](docs/architecture.md) - How the macros turn a declaration into each surface, the data structures they share, the feature flags, and where each part of the source lives.
- [docs/type-mappings.md](docs/type-mappings.md) - How each Rust type, serde attribute and published name is written on every surface, and the form each client sends.
- [docs/field-validation.md](docs/field-validation.md) - `#[model_schema_prop(...)]`, the generated `validate()`, and the read that refuses what `validate()` refuses.
- [docs/kotlin-client.md](docs/kotlin-client.md) - `validate()`, the read and `preprocess` in the Kotlin client.
- [docs/dart-client.md](docs/dart-client.md) - `validate()`, the read and `preprocess` on the Dart types.
- [docs/swift-client.md](docs/swift-client.md) - `validate()`, the read and `preprocess` in the Swift client.
- [docs/generics-and-brands.md](docs/generics-and-brands.md) - Branded newtypes, generic types and Zod factories, declared defaults, and the string constraints of a generic brand.
- [docs/recovering-decode.md](docs/recovering-decode.md) - The recovering decode (`decode_with`): its entry points, walkers, support libraries and the MongoDB paths.
- [docs/testing.md](docs/testing.md) - How to add a type mapping, a feature, a test or an example, and what CI runs.
