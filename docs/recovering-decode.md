# Recovering Decode

The recovering decode (`decode_with`): its entry points, walkers, support libraries and the MongoDB paths.

### Recovering Decode

`#[model_schema(enable = ["decode_with"])]` makes a type implement `DecodeWith`, the trait its
crate's library holds, whose methods are `from_value_with`, and `from_bson_with`
as well under `bson`: serde reads the value with the type's read alone, a walker lists every issue
in it, and a callback supplied with the call rejects the record or fixes it once.
`from_value_piped` and `from_bson_piped` are the same reads over resolvers run in order, each
answering one issue. `enable = ["decode_with", "mongodb"]` adds, under `mongodb`, the type's typed
MongoDB paths, the filters and updates they build, and the operations that read and write its
rows; without the `mongodb` feature the entry adds nothing. What every flagged type shares is a
support library its crate writes once. `README.md`'s "Recovering Decode" and "MongoDB Operations and Typed Filters"
sections are the consumer's view; this is what a change to the emitter has to keep. "The flag"
below is the entry `"decode_with"`, and a flagged type is one that lists it.

- **The `enable` list is read by the parser, in every build.** `enable_arg` in `src/model_schema.rs`
  answers `Enabled { decode_with, mongodb }` into `ModelSchemaArgs::enable`, and refuses, spanned on
  what is written: `"mongodb"` without `"decode_with"`, an entry it does not read, an entry written
  twice, an empty list, and any form but a list of string literals (`ENABLE_TAKES`). `apply_arg`
  refuses the bare `decode_with`, in each of its three spellings, with the `enable` form to write,
  and `KNOWN_ARGS` lists `enable` in its place. A refusal is `ModelSchemaArgs::arg_rejection`,
  raised at `exec_model_schema` ahead of every shape under the prefix ``model_schema: type
  `{name}`:``, and the item expands with nothing of the recovering decode.
  `the_bare_decode_with_flag_is_refused_with_the_enable_list_it_moved_to` and the `enable_refuses_*`
  tests in `src/model_schema/tests.rs` hold each message.
- **What each entry generates, and under which feature.** `"decode_with"` is the recovering
  decode: an `impl` of `DecodeWith` holding `from_value_with` and `from_value_piped`, their
  walkers, the type's own module items and the
  re-export of the `decode_with` library, and under `bson` the BSON source as well.
  `Source::read_by()` takes no argument and answers `Source::GENERATED`, which holds `Bson` under
  `bson` alone, so every flagged type is walked from BSON in a build with `bson`, and
  `bson_issue_method` writes its BSON check issues. `"mongodb"` adds, under `mongodb` and nothing
  without it, the typed paths (`struct_paths` and `enum_paths` are called only for such a type,
  `TypedPaths::default()` otherwise), the operations, which `bson_entry_methods` writes only where
  `Source::entry_methods` hands it `written.mongodb`, and the re-export of the MongoDB library and
  its probe. `Written::mongodb` carries the entry through the walk. The expansion test in
  `recovering_decode/tests.rs` holds a type that lists `"decode_with"` alone, under `mongodb`, to
  `from_bson_with`, `from_bson_piped` (in the `DecodeWith` impl) and the BSON walkers, and to no name of `OPERATIONS`, no
  `MONGO_FIELDS` and no `MongoFields`.
- **The reads are methods of `DecodeWith` alone.** `type_impls` writes each source's helpers and
  walkers into the type's inherent `impl` and the four reads, which `Source::entry_methods`
  answers apart as `Entry::reads`, into one `impl` of `DecodeWith`, so no type has an inherent
  and a trait method of one name (clippy's `same_name_method`). A trait has one `impl` per type,
  so a generic type's is written under the bounds of the last source, which hold the others'
  (`Source::bounded`, `read_under`): in a build with `bson`, its JSON reads ask `Serialize` of the
  parameters too. The `impl` names the trait as `#module::DecodeWith`; `DecodeWith` is in
  `ADDED_TYPE_NAMES`, so a type of the author's named `DecodeWith` stays the author's. A crate without
  `decode_with_foundation!()` gets the unresolved import of its library and no more: a trait
  `impl` naming an unresolved trait stops rustc before the type check that holds the foundation
  probe's message.
- **The two initializers write the support libraries, once per crate.** `src/foundation.rs` refuses
  any argument (`refused_arguments`) and hands each macro to `decode_with_foundation` and
  `mongodb_foundation` in `recovering_decode.rs`. The first writes the module
  `__tixschema_decode_with` (`DECODE_WITH_LIBRARY`): `decode_with_library`, which is `trait_items` (the `DecodeWith` trait, with the BSON methods
  from `bson_trait_methods` under `bson`), `path_items`,
  `callback_items`, `handoff_items`, `taken_items` and `bound_items`, and under
  `#[cfg(feature = "bson")]` the three BSON pieces (`bson_bound_items`, `bson_leaf_items`, and
  `bson_stored_items`, the `impl Stored for bson::Document`). `path_items` writes `Segment`, `Path`,
  the `Stored` trait and its impl for `serde_json::Value`. It names no
  `bson` path in a build without `bson`. So a crate whose types list `"decode_with"` alone names
  `bson`, and lists it, wherever tixschema's `bson` feature is on, and never needs `mongodb`. The
  second writes `__tixschema_mongodb` (`MONGODB_LIBRARY`) under `mongodb`: `mongodb_library`, which
  is `query_items` and `operation_items`. It reads the first library through
  `use crate::__tixschema_decode_with::*;`, so it is written beside it. `MONGODB_LIBRARY`,
  `MONGODB_LIBRARY_NAMES`, `mongodb_reexport`, `mongodb_foundation_probe` and the
  `mongodb_foundation` that writes the module are under `#[cfg(feature = "mongodb")]`, and the
  empty `mongodb_foundation` answers every build without it. The first writes nothing without
  `serde`, so each call stays in a crate whatever it builds with, and no consumer writes a line
  for the BSON pieces. Each writes beside its module a private marker trait implemented for `()`,
  `__TixschemaDecodeWithFoundation` and `__TixschemaMongodbFoundation`, whose one method the probes
  look for. `each_initializer_writes_its_module_and_its_marker` and
  `the_two_libraries_share_no_name` hold them.
- **Each `{type}_schema` module re-exports the libraries, name by name.** `library_reexport`
  writes `pub use crate::__tixschema_decode_with::{...};` from `DECODE_WITH_LIBRARY_NAMES`, whose
  five BSON pieces (`bson_bound`, `bson_digits_settled`, `bson_leaf`, `bson_remaining`,
  `same_bracket`) are each under `#[cfg(feature = "bson")]`, and, for a type that lists
  `"mongodb"`, the same from
  `MONGODB_LIBRARY_NAMES` under `mongodb`. A glob does not
  serve: the module reads the author's scope through `use super::*`, and a second glob holding a
  name the first holds makes it ambiguous (E0659), where a name written out shadows the glob as an
  item the module declares does. So every `{type}_schema::Name` path a consumer writes resolves,
  and `reading_the_authors_scope` and `ADDED_TYPE_NAMES` read the module as before. The items
  written for one type alone (`paths.module_items`, `read_whole_items`, `Reach::module_items`)
  stay in its module and name the libraries' items through the re-export.
  `a_type_reexports_its_crates_libraries_by_name` holds it.
- **A crate without its initializer fails with the line to add.** `foundation_probes` writes, beside
  each flagged type, one `foundation_probe` for the `decode_with` library and, for a type that lists
  `"mongodb"` under `mongodb`, one for the second. A probe is an anonymous `const` that brings
  `crate::*` into scope and calls the marker's method on `&()`: with the initializer, the crate
  root's marker answers `()`, which the block-local `FoundationWritten` is implemented for; without
  it, the block-local `Fallback` on `&()` answers `Missing`, and the
  `#[diagnostic::on_unimplemented]` on `FoundationWritten` is the E0277, its message, label and note
  naming the line to write and where. The probe is order-independent: the initializer may be
  anywhere at the crate root, after the `mod` lines included.
  `a_type_probes_for_the_libraries_it_reexports` holds the text.
- **The flag is judged at `exec_model_schema`.** `decode_with_guard_errors` runs there, ungated, so
  the refusals -- a type alias, a type that declares a lifetime, a field that borrows -- are the
  same in every feature combination and are raised before any shape is split off. The emitter
  itself runs under `serde`, from `struct_decode_with` and `enum_decode_with`, and reads the item
  as it is emitted, so a read hook tixschema hangs on a field is one the walker reads through.
- **Every read the flag makes is the type's read alone.** A plain read of a type ends on its
  `validate()` (section 7, "A read refuses what `validate()` refuses"), and a refusal there would
  hide the list. `listed_read` in `judged_read.rs` writes `decode_with_read_unjudged` on every
  flagged type, once, after the first source's methods, and the entry points, `read_whole`,
  `lone_slot_bound` and the enum walkers read `Self` through it. Where a schema surface is on it
  calls the type's `model_schema_read_unjudged` behind the `ReadUnjudged` fallback. With none on,
  a holder that names no check publishes no `validate()` and its own read judges what it holds,
  so the function is serde's derive on a copy of the item, each model below read through its own
  `decode_with_read_unjudged`. A generic type's `impl` asks what that read asks beside
  `Deserialize` of a parameter (`read_under`), at every lifetime, and the function repeats none
  of it: a bound asked by the `impl` and by the function is one the compiler cannot choose
  between (E0283). A list held to a count of its own is read whole through `listed_reader`, with
  each model unjudged, as a type declared in the closure and generic over the parameters of the
  item that the model names (`HookScope`, the rules the holder's own hook is written by): the same
  read for `Vec<Tote<CodeType>>` and `Vec<Tote<String>>`, and for `Vec<Self>`. Where `HookScope`
  turns the member away, or the type names a lifetime, the list is read whole through
  `own_reader`, each model judged as it is read. A
  type that names a `preprocess` on itself is read through `decode_with_read_corrected`, from a
  value its functions corrected once (section 7, "A payload corrected before it is read").
- **A walker is generic over the issue type.** Each walker method takes a type parameter for the
  issue it builds and an `IssueFromParts` constructor, a function pointer over standard types, so
  one type's walker builds another type's issues at their full path. Within one crate the flagged
  types share one `Issue`, the library's, and every `{type}_schema::Issue` is that type. Across
  crates each crate holds its own library, and what crosses is standard types alone:
  `ExpectedToken` and `IssueFromParts` are aliases of standard types, and `issue_from_parts`
  turns the parts into the `Issue` of the crate whose walker called it. So a type of one crate
  holding a type of another builds and lists the held type's issues at their full path, as two
  types of one crate do.
- **A field's type is walked by what tixschema has seen of it.** `record_declaration` notes at
  `exec_model_schema`, in every build with `serde`, whether a name is a struct or an enum or an
  alias, and what a parameterless alias names. A model type seen above is called as it stands, so
  one with no flag fails the build (E0599), as before. Any other type goes through
  `Walker::asked`: the call is written in a block with `{type}_schema::ReadWhole` in scope, so a
  type with a walker of its own answers with it (an inherent function wins over a trait's) and
  one with none is read whole. `<()>::decode_with_read_whole()` beside the call keeps the import
  and the trait's methods in use where the type answers itself; without it a consumer that denies
  warnings fails on `unused_imports` and `dead_code`. The trait is added to the module only where
  a walker asks for it, and answers for the sources the type's own walkers read.
- **A type that lists `"mongodb"` may hold one that lists `"decode_with"` alone.** Under `bson`
  every flagged type has the BSON walker its holder calls, and the holder's typed paths name no
  held type's module, so nothing asks what a held type lists and nothing is refused for it.
- **What holds the libraries as a consumer meets them.**
  - `tests/foundation_refusals.rs` is the `trybuild` harness. Each file of
    `tests/foundation_refusals/` is a crate of its own, held to the `.stderr` beside it, written in
    the build with every plain feature on, the one the harness is gated on: a missing initializer,
    each refusal of the `enable` list, an initializer handed an argument, a type that lists
    `"mongodb"` with a map keyed by a number, a map keyed by a number under
    `#[serde(flatten)]`, an enum that declares no variant, under a tag or not, and a member that
    holds an `i128`. The initializer
    written twice, `tests/foundation_refusals/written_twice/main.rs`, is checked by cargo's JSON
    messages instead: an `E0428` names each item it writes, and no error but `E0428` and `E0119`
    is reported. rustc 1.99 reports the second marker's `impl` as `E0119` and 1.100 does not.
  - `tests/crates_apart/` holds two crates, path dev-dependencies of this package, each writing its
    own libraries: `stored_customers` declares a stored `Customer`, and `invoice_lookups`, whose
    types list `"decode_with"` alone, lists no `mongodb` in its manifest and builds under
    `deny(warnings)` where tixschema has `mongodb` on. `tests/crates_apart_tests.rs` reads a
    `Customer` inside a type of its own and lists an issue in it at its full path, from JSON and
    from BSON, and reads the types of `invoice_lookups` through that crate's library.
  - `tests/support_library_tests.rs` holds one library per crate: three types re-exporting it and
    one resolver serving the reads of all three, what a type that lists `"mongodb"` gains under each
    feature and a type that lists `"decode_with"` alone does not, the BSON decode of a type that
    lists `"decode_with"` alone (`a_type_that_lists_decode_with_alone_decodes_bson`), the issue of
    such a type held by one that lists `"mongodb"`, at its full path
    (`a_stored_holder_lists_the_issue_of_a_held_type_that_lists_decode_with_alone`), and the
    author's own `Path` and `Filter` beside the library's.
  - `examples/strict_consumer_models`, a library with models in a public and a private module, and
    `examples/strict_consumer_app`, a binary with models of its own, each write both initializers
    and are linted by this package's own clippy table, pedantic and restriction included. Both
    require the `mongodb` feature.
- **An alias seen above is walked as the type it names.** `aliases::Reach` writes the field's type
  out again in a clone of the item the walker is then run on: a standard type by its full path,
  and every other type under an alias in `{type}_schema::decode_with_at{N}` that reaches it from
  the field's own type (`<<Counts as IntoIterator>::Item as EntryOf>::Value`) and keeps its last
  name, which is what the walker reads a type by. Nothing the alias was written with has to
  resolve beside the field. A `const _: fn(Written) -> WrittenOut = |held| held;` beside the type
  holds the two to one type.
- **A field's bound is the walker's to list.** `generate_field_validation` records each validator
  it publishes under the `Validator` gate (`record_field_validator`, by schema module and helper
  stem, with the name the validator writes in front of a violation, an `Option`), `walked_field`
  reads it back (`recorded_validator`, into `FieldValidator::named`), and `Walker::bounded` writes
  one `{source}_bound` call beside the read of each value the bound reaches: the validator run on
  what serde read, one `Invalid` per value, its violations joined by `; `, that name taken off
  each sentence. The name is the steps serde writes on the way to the field, read from the
  registry and computed nowhere else, so the two cannot drift, and the validator of a value
  serde writes itself, the one slot of a struct and of a variant under a tag alone, writes none:
  its sentence is listed as it is.
  `recovering_bound_check` builds the closure from the walk's own type at
  that position. A list is that position: the arm
  that walks its items hands them no validator, and holds the list read whole (`read_as_a_list`)
  to its count, one `Invalid` at the list. The registry is cleared per item at `exec_model_schema`, and holds nothing in a
  build with no schema surface, where no validator is published.
- **A check is the read's to list, after serde.** `recovering_decode/checks.rs` emits
  `decode_with_check_messages(&self)` in an `impl` of its own beside the walkers, under the bounds
  the type's author wrote (`RecoveringDecode::check_messages`, `checks::type_impl`): the members'
  messages, the type's own, then those of every model it holds, each under the place serde writes it
  (`member_steps` and `variant_steps`, the two functions `validate()` names a member by). A held
  type is asked through the block-local `Unchecked` trait, so one with no such method answers
  nothing, and a map's key is written through `WrittenKey`. The checks are read back from the
  registry `record_item_checks` and `record_member_checks` write, by schema module and helper stem.
  `entry_methods` and `bson_entry_methods` splice the call after each read serde accepts, in the two
  shared fragments, and `decode_with_value_check_issues` and, under `bson`,
  `decode_with_bson_check_issues`, which `bson_issue_method` writes for every flagged type, turn
  each message into an `Invalid`, `expected: Unknown`, at the message's lead read against the raw
  value. The three names are in the list
  `every_added_method_but_the_entry_point_carries_the_flags_name` pins.
- **Every name the flag adds is its own.** Each walker method carries the flag's name
  (`decode_with_value_issues`, `decode_with_bson_fields`, `decode_with_value_named`), and a
  method's own type parameters take a name the item does not write (`unclaimed_parameter`): `D`,
  `F` and `I`, and under `mongodb` `U`, numbered where the item writes one. The parameters a struct
  of paths declares beside the type's own are named the same way: `Root`, and `Whole` and `Value`
  on a type serde writes as one value. An entry point and an operation are the name they are
  called by (`from_bson_piped`, `find_one`), and the typed paths carry the feature's
  (`MONGO_FIELDS`, `mongo_write_{field}`, `mongo_read_row`).
  `every_added_method_but_the_entry_point_carries_the_flags_name` pins the whole list in order:
  a function of the author's own under one of those names is a duplicate definition (E0592). The
  libraries' modules and markers are named for tixschema (`__tixschema_decode_with`,
  `__TixschemaDecodeWithFoundation`), and a probe's names are local to its anonymous `const`.
- **Both entry points of a source share its two reads.** `entry_methods` and `bson_entry_methods`
  build the first read and the second read once, as token fragments, and splice them into the
  callback entry point and into the piped one, so neither read can change for one and not the other.
  `resolver_items` writes `Resolution`, `Resolver` and `unsettled`, the pipe, beside `Verdict`, in
  the `decode_with` library. `Resolver` is `&dyn Fn + Sync`: a read that waits for MongoDB holds its
  resolvers, and without the bound its future is not `Send`, which clippy's `future_not_send`
  reports at every flagged type of a consumer that denies it.
- **What `mongodb` adds is emitted under `#[cfg(feature = "mongodb")]`, for a type that lists
  `"mongodb"`, and nothing of it in any other build or for any other type.** `query.rs` writes the
  query types into the MongoDB library (`query_items`, spliced by `mongodb_library`).
  `fields.rs` writes `MongoFields` into the type's own module with one function per field, and
  onto the type `MONGO_FIELDS` and one `mongo_write_{field}` per hooked field (`struct_paths`,
  `enum_paths`, called only where `Written::mongodb` holds). `operations.rs` writes
  `OperationError` and `Read` into the library (`error_items`, `read_items`, through
  `operation_items`) and the operations onto the type, beside the BSON entry points and under
  their bounds (`methods`, which `bson_entry_methods` calls only where it is handed the entry).
  Each is reached through a `cfg`'d pair in `recovering_decode.rs`, whose other half answers
  nothing, so a type without the flag expands as before, a type that lists `"decode_with"` alone
  gains no operation and no path under any feature, and a type that lists `"mongodb"` in a build
  without `mongodb` gains nothing more than the BSON read every flagged type has under `bson`.
  The query types' items the typed paths of a type call across modules are `pub(crate)`
  (`Field::held_path`, `Filter::held`).
- **A type's paths hold its own keys, and nothing of another type's.** `MongoFields` holds a
  marker and no path. Each named member is a `const fn` of it (`of_functions`) that answers one
  path kind at a key written as a literal, relative to the type: `"total"`, and `"list.0"` where
  the type's own shape puts two keys. A `MongoPath` is that one key. So the size of what is
  generated for a type does not follow what the type holds, and no function builds a tree:
  `tests/mongodb_query_tests/wide.rs` holds four levels of six fields each to clippy's
  `large_stack_frames`. Each binary declares that module outside `cfg(test)`, where the lint
  reports; inside it clippy reports nothing for the same types. A tuple of slots stays a tuple of
  paths, a position being a member access.
- **`nest` is the one way below a nested type, and two standard traits are what crosses.** A
  path kind carries the model `nest` reaches as its last parameter, `N`: the field's type read
  through `Option`, the list and the wrappers `through_wrappers` reads, which `reached_through`
  writes only where it is not the value an operator takes. `nest` takes a filter over the rows
  of `N` or an update of them by the bound `Nests<Root, N, Kind>`, which the MongoDB library
  implements twice over the two markers a filter and an update carry (`AsFilter`,
  `AsUpdate`), and writes
  each key under the path's own (`nested_filter`, `nested_update`, `nested_key`). No expansion
  names another type's module: a flattened model and a variant that holds one at the enum's own
  level are a `Nested<Root, N>` at the key they are written under.
- **Every `impl` of a struct of paths is written beside the type, and the schema module and
  `mongo_members` name no type, trait or bound of the author's.** A module written inside a
  function body reaches none of that function's items, so the two modules hold declarations
  only, under the parameters `Own::bare` writes: the row type, then the type's own with no bound
  but `?Sized`, no default and no `where` clause.
  - `of_functions` declares a struct of functions in its module and writes its one `impl` beside
    the type (`TypedPaths::beside`), under `Own::writing`: every member's function, the
    `is_{variant}` asks, and the arm functions of an enum under a tag.
  - A tuple's slots keep their positions. `of_slots` declares a trait beside the struct, with one
    associated type per slot (`Slot0`, `Slot1`, ..): `MongoSlots` for `MongoFields`, and for a
    struct of `mongo_members` `{Struct}Slots`, under a name the item does not write. The struct
    reads each slot as `<() as Trait<..>>::SlotN` and declares `where (): Trait<..>`, and the
    `impl` of the trait for `()` beside the type fills each slot under the author's bounds
    (`Own::declared`), so a slot whose type needs a bound to be well formed builds. The
    `MongoFields` `Own::keeping` writes repeats the predicate of the `Members` it holds.
  - `Own::keeping` and `Own::whole` keep `built`, `Deref` and `regex` in the module: they name
    only the module's own items, under the bare parameters.
  - A hooked path writes through `mongo_write_{field}`, a function of the type's own that calls
    the hook as the attribute writes it. In an `impl` beside the type `Self` is the struct of
    paths, so `Own::naming_itself` writes the type's own name wherever its author wrote `Self`:
    in the value of a path, `children: Vec<Self>` on `Node` being a `Field<Root, Vec<Node>>`,
    and in a bound of the type's own. `mongo_write_{field}` is in the type's own `impl` and
    keeps `Self` as written.
  - `tests/mongodb_query_tests/function_body.rs` declares each shape inside a test function, a
    holder of a type declared there under `not(feature = "jsonschema")`, whose JSON Schema
    names the held type's module.
- **A type serde writes as one value is one path.** `written_as_one_value`, recorded as
  `Declared::OneValue`: its `MongoFields<Root, .., Whole>` holds the path at the empty key and
  dereferences to it, the path type being a parameter the type's own `impl` writes, so its module
  names no type of its author's and builds inside a function body. Held by another type it is a
  plain kind over the value, a `Field` or an `OptionalField`.
- **A member is a model only where its walker is called.** `Own::seen_model` reaches a struct or
  an enum `#[model_schema]` was written on above (`declared`), never the type itself, one
  declared below, an alias or another crate's type: each of those is one whole value, and so is a
  field under a hook or one serde never reads back, where no walker is called either. `nest`
  reaches below a whole value as it does below a model.
- **An `is_{variant}` keeps its name.** `enum_paths` claims the names of the asks first, and the
  function of a variant that meets one is numbered by `unclaimed`. A struct of paths carries no
  inherent function but the members' own, the asks and, on an enum under a tag key, the arm
  functions, so a field named `built`, `plain`, `key` or `nest` builds. `MongoFields::built` is
  on the struct of a one-value type, and of a tuple that keeps a marker beside its slots
  (`Own::keeping`), neither of which has a field function.
- **An enum under a tag key has one filter per arm, and a name two functions would take is
  refused.** `tag_arms` in `recovering_decode/enums.rs` is the one seam: for an enum
  `Tagging::of` reads as `Internal` or `Adjacent` it answers the tag key and one `TagArm` per
  variant serde writes, with the value serde writes for it and the name of its function,
  `<tag>__<value>`, each side through `snake_form`. `written_variants` beside it is where a
  variant's written name and the stem of its `is_{stem}` are read, for every enum: `fields.rs`
  computes neither.
  - `Emitter::arms` writes the functions into the `impl` of the struct of paths **beside the
    type**, with its other functions, so the model an arm holds is named as its author wrote it
    and an enum declared inside a function builds. `Own::naming_itself` writes `Self`
    as the enum there. Each is a `const fn` that answers `ArmFilter<Root, M>`.
  - `ArmFilter` (`query.rs`, in the MongoDB library) holds the tag, the value, the arm's
    place and the conditions, and no document until it is turned into one. The place is
    `MongoPath::ROOT` under a tag written beside the fields and the content key under
    `content`. Its `and` takes a filter over the rows of `M` by the two standard traits and
    writes it at the place through `nested_filter`. It carries `Into<bson::Document>` and
    `AsRef<PhantomData<Root>>` itself, and `Filter` converts from it.
  - `ArmFilter::update` and `ArmFilter::tag_on_insert` answer `ArmUpdate<Root, M>` (`query.rs`),
    an update of the rows of the enum. `update` and `ArmUpdate::and` take an update of the rows
    of `M` by the two standard traits and write it at the place through `nested_update`, and
    `update` leaves the arm filter's conditions behind. `tag_on_insert`, on both, adds the tag
    to `$setOnInsert` under the tag key, at the enum's own level and never at the place, and
    only where it is called. An arm update carries `Into<bson::Document>` and
    `AsRef<PhantomData<fn(Root) -> Root>>`, that `AsRef` alone, and `Update` converts from it.
    Every merge is `Update::merged_update`, a private function of `Update` that `Update::and`
    calls too: `an_update_and_an_arm_update_merge_through_one_function` holds the loop to being
    written once.
  - `M` is what `nest` reaches below a path over the variant's one slot (`arm_model`, through
    `taken` and `through_wrappers`), and `()` for any other variant and for a map. No registry
    is asked, so a model declared above, one declared below and one of another crate are one
    case.
  - `Arm`, in the enum's own module, has one unit arm per variant that has a function, under
    the variant's ident, with no serde attribute. Nothing emitted reads it.
  - `tag_arm_refusals` refuses what would let two functions of the struct take one name: `__`
    in a variant's Rust name, two values with one snake case, and an arm function that meets an
    `is_{variant}`. `decode_with_guard_errors` raises them under `serde` for every enum that
    lists `"decode_with"`, so the verdict is the same with the `mongodb` feature on and off and
    with the entry `"mongodb"` listed or not, and an enum without `"decode_with"` is refused
    nothing.
  - A name that is no identifier, under a tag key whose snake case starts with a digit, is
    written as no function and no arm (`function_ident`).
- **A type that lists `"mongodb"` holds no map whose key the `bson` library refuses.** serde
  writes a key of a number or a `bool` as text into JSON only, and the `bson` library refuses to
  write one and reads a stored one back as text, which the key's type refuses. Measured under
  both majors: a `char`, a `chrono` date or time, a plain enum and a brand over a `String` are
  stored as their text and read back, and are not refused. `mongodb_map_key_refusals`, which
  `decode_with_guard_errors` chains where `args.enable.mongodb` holds, raises one error per such
  map, in every build, spanned on the member's ident (a slot's type) and naming the member, behind
  its variant in an enum, and the key as written. A type that lists `"mongodb"` and keys a map by
  its own type parameter is not refused, by decision: no expansion sees what fills the parameter,
  and the README states the error a write and a read of such a row answer at run time.
  - `bson_refused_keys` finds each map (`is_map_wrapper`) a member serde writes or reads holds,
    behind `Option`, a list, a tuple, a transparent wrapper, a map's value, both arms of a std
    `Result` (`written_std_result`, the `Ok` first) and the bound of a std range
    (`written_std_range`), and each model declared above that holds one (`RefusedKey::Held`).
    The two seams leave a type of the crate's own declared above under a std name to
    `held_model_maps`, the model it is. A member or a variant under `skip`
    (`absent_from_wire`) is read past, and so is a member or a variant written and read by its
    author's functions (`written_and_read_by_the_author`, in every build: `with`, or
    `serialize_with` with `deserialize_with`), for a map and a held model alike, so an enum whose
    only such map is in such a variant is not recorded as a holder. One under either of the two
    alone is not.
  - A held model is looked into through `BSON_REFUSED_HOLDERS`, each struct, enum and alias that
    holds such a map at any depth to a description of the first one. It is written in every build
    at `exec_model_schema` (`record_bson_refused_holder_item`), whatever the model's `enable`
    list: for a struct and an enum from the walk `bson_refused_maps` makes, the one
    `mongodb_map_key_refusals` makes, and for an alias from `bson_refused_keys` over the type it
    names, described by `RefusedKey::described`. The refusal names the holder's member, the model
    or the alias, and that map. A member that holds the item itself is read past, its maps being
    the item's own. A model or an alias declared below, or of another crate, is not looked into.
  - `is_bson_refused_key` answers for a key, read under `Box`, `Rc`, `Arc` and `Cow`: one of
    `BSON_REFUSED_KEY_TYPES`, or a name `BSON_REFUSED_KEYS` holds. That registry is written in
    every build at `exec_model_schema` (`record_bson_refused_key_item`), for a brand or an alias
    over such a key, so a chain forwards its target's answer. A brand or an alias whose value
    reaches one of its own type parameters is written instead into `FORWARDED_KEY_PARAMETERS`,
    with that parameter's position among its type parameters (`forwarded_parameter_position`),
    and a key written with its name is answered for the argument at that position: `Ordinal<u32>`
    is refused, `Ordinal<String>` and a bare `Ordinal` are not. The value reaches the parameter
    written bare, under `Box`, `Rc`, `Arc` and `Cow` (`Boxed<T>(pub Box<T>)`,
    `type Held<T> = Box<T>`), and through a brand or an alias recorded there, at the position it
    forwards (`Rank<T>(pub Ordinal<T>)`). A name declared below, or of another crate, is not
    refused, and a brand reached through one declared below forwards nothing.
  - A held model or alias is read at the arguments it is written with (`held_model_maps`). A walk
    asks one thing of each key (`KeyAsked`): `Refused`, or `Parameter(at)`, the holder's own type
    parameter at that position. `FORWARDED_KEY_PARAMETERS` also holds, for each struct, enum and
    alias that keys a map by its own type parameters, the position of each such parameter and the
    first map it keys, written in every build at `exec_model_schema`
    (`record_forwarded_key_maps_item`): `first_held_map` asks `Parameter(at)` of a copy of the
    item whose members, or whose alias's value, spell each parameter as its
    `parameter_placeholder` (`with_parameters_placed`), through `bson_refused_maps` for a struct
    and an enum and `bson_refused_keys` for an alias, the walks `record_bson_refused_holder_item`
    makes. `filled_in` writes a holder's arguments in their place in one pass, so the map is named
    at what fills each parameter on the way: `ByKey<u32>` over
    `type ByKey<KeyType> = HashMap<KeyType, String>`, and `Rekeyed<u32>` over
    `type Rekeyed<K> = Keyed<K>`. One name holds one record, so an item whose value reaches one of
    its parameters (`forwarded_parameter_position`) keeps the position
    `record_bson_refused_key_item` writes and records no map. A held model whose argument at such
    a position is the key a walk asks for, and that holds no such map of its own
    (`BSON_REFUSED_HOLDERS`), is one `RefusedKey::Held` named as written (`Keyed<u32>`). A model
    `is_declared_ident` answers for has each type argument walked as the member's own type, and
    each map found there is a `RefusedKey::Held` named as written. Every argument is read as one
    the model writes into its row: a map an argument holds is refused even where the model holds
    that argument only in a `PhantomData` or in a member serde never writes, an assumption the
    owner reviews. A model or an alias declared below, or of another crate, is not read at its
    arguments.
  - `tests/foundation_refusals/number_keyed_map_with_mongodb.rs` holds the README's `Roster` to
    the error, `held_number_keyed_map_with_mongodb.rs` beside it a held model's, the unit tests
    in `src/model_schema/tests.rs` each position, each key, each held model, each held alias,
    each generic model and alias read at its arguments and each member and variant read past, and
    `tests/mongodb_operation_tests/readme/operations.rs` the README's text: the declaration and
    the error, the keys stored as their text, a type that lists `"decode_with"` alone, and a map
    keyed by the type's own parameter, with the error each major answers at run time.
- **A path is under the name serde writes.** `parse_written_renames` reads `rename`, `rename_all`
  and `rename_all_fields` for their `serialize` side in every build. A list whose two sides differ
  is refused by every build that describes the type, a mobile client included:
  `rename_direction_guard_errors` runs under `serde` with any of `typescript`, `zod`,
  `jsonschema`, `dart`, `swift` and `kotlin`, and a build with none of the six takes the list.
  `the_rename_guard_is_compiled_in_every_build_that_describes_a_type` reads those gates off the
  sources, no local build being of the clients alone.
- **Two standard traits join filters and updates across modules.** A `Filter<Root>` is
  `Into<bson::Document>` and `AsRef<PhantomData<Root>>`, and an `Update<Root>` is
  `Into<bson::Document>` and `AsRef<PhantomData<fn(Root) -> Root>>`. `Filter::and`, `Filter::or`,
  `ArmFilter::and`, `ArmFilter::update`, `Update::and`, `ArmUpdate::and`, `ModelList::elem_match`,
  `ModelList::pull`, `nest` and every operation take a filter or an update by those bounds, never
  as the module's own type: that is what lets a filter of another module over the same rows join
  and be taken, a filter over a nested model be nested, and what refuses an update where a filter
  is asked. `a_filter_and_an_update_each_carry_a_marker_of_their_own` pins each bound.
- **The operations hand the driver documents, and read each row themselves.** The collection is
  `mongodb::Collection<bson::Document>`: a row is read through `from_bson_piped`
  (`mongo_read_row`) and written through `bson::Serializer::new` (`mongo_written_row`), never by
  the driver's serde, and many rows are read with the cursor's own `advance` and
  `deserialize_current`, so a consumer lists no `futures`. `WriteError` is
  `<bson::Serializer as serde::Serializer>::Error`, one path under both majors. `OperationError`
  converts from neither error, and the generated operations and every example build each member
  by name.
- **A read answers `Read`, never a future of its own.** `find_one`, `find`, `count` and the two
  that take resolvers answer `{type}_schema::Read<'c, T, O>`, which holds the function that reads
  one row, so the module names no type of its author's. It takes the five options, and
  `IntoFuture` is implemented once per answer (`Vec<T>`, `Option<T>`, `u64`), each handing the
  driver's method the options that method has. What is awaited is a boxed `Send` future.
  `insert_one` writes the row when it is called and answers a boxed `Send` future too: an
  `async fn` over `&self` is `Send` only where the type is `Sync`.
- **The lists of what the flag adds are pinned.** `DECODE_WITH_LIBRARY_NAMES` is every name the
  `decode_with` library makes public in the build, the five BSON pieces under their own `cfg`, and
  `MONGODB_LIBRARY_NAMES`, under `mongodb`, every name the MongoDB library makes public: each is
  what a `{type}_schema` module re-exports, and
  `the_decode_with_library_names_are_every_name_it_makes_public` and
  `the_mongodb_library_names_are_every_name_it_makes_public` hold each list to the items its library
  declares. `ADDED_TYPE_NAMES`, with `ADDED_TYPE_COUNT`, is every type name the schema module of a
  type that lists both entries holds, the ones it re-exports and the ones written for it alone, the
  ones `mongodb` adds under their own `cfg`: `reading_the_authors_scope` reads past those names, and
  `every_type_the_flag_adds_is_a_name_read_past` holds the list to the libraries and to what the
  module of a struct, that of a tuple struct and that of an enum under a tag declare.
  `every_added_method_but_the_entry_point_carries_the_flags_name` is every method the flag adds to
  the type, and `OPERATIONS` in `recovering_decode/tests.rs` is what
  `mongodb` adds to it. A name a library, a module or a type gains joins its list in the same
  change.
- **The emitted code names only what both major versions of the `bson` library have.** Every BSON
  value is read through `bson::Deserializer::new` and written through `bson::Serializer::new`.
  `tests/recovering_decode_bson_tests/` is compiled once per major, each time beside a MongoDB
  driver built for that major: by `tests/recovering_decode_bson2_tests.rs` here, which binds the
  name `bson` to version 2 with an `extern crate`, and by
  `bson3/tests/recovering_decode_bson3_tests.rs` in `bson3/`, a package of its own whose `bson` is
  version 3.
- **`bson3/` mirrors this package's manifest.** One package holds one build of the driver, and one
  build serves one major, so the version 3 tests cannot sit in `tests/`. The shared sources read
  `cfg(feature = "...")` and are linted in whichever package compiles them, so `bson3/Cargo.toml`
  declares every feature under the same name, each turning on `tixschema/<name>`, and carries the
  lint tables entry for entry. It resolves into a lock file of its own, so it also pins every
  advisory floor this manifest pins, the dependencies under the comment `# Transitive dep floors`,
  at that version or above. `bson3/tests/mirrored_manifest_tests.rs` fails when any of the three
  drifts. Every `just` recipe that tests, lints, checks, formats or audits this package runs the
  same line over `bson3/Cargo.toml`.
- **Nothing emitted carries `#[allow]`, `#[expect]` or `#[doc(hidden)]`, with two exceptions.** A
  consumer's lint levels reach what the macro emits into their crate: every added type is
  `#[non_exhaustive]`, no added function takes `impl Trait` as a parameter, and a `Result` is
  written `core::result::Result`. The two exceptions, both on the support libraries, and why:
  - Each `pub use` of a library in a `{type}_schema` module carries
    `#[expect(clippy::pub_use, reason = REEXPORT_REASON)]` (`reexported`). This repository and
    `bson3/` deny clippy's whole restriction group, `pub_use` among it, and so may a consumer; the
    re-export is what keeps every `{type}_schema::Name` path reachable from the crate and from
    another, and an alias per type or a `pub(crate) use` in its place breaks a path written today
    or another crate's callback. A consumer crate built with `cargo build`, with clippy at the
    lint's default level and with clippy's restriction group denied reports neither `pub_use` nor
    an unfulfilled expectation.
  - Each library module, `__tixschema_decode_with` and `__tixschema_mongodb`, carries
    `#[doc(hidden)]`, so the library is not published in the consumer's documentation as a module
    of its own; its items are documented under each `{type}_schema` that re-exports them.
  `nothing_decode_with_emits_suppresses_a_lint_but_the_two_exceptions` in
  `src/model_schema/tests.rs` holds every expansion and both libraries to these two and no other:
  one `#[expect(clippy::pub_use, reason = ...)]` per re-export, and one `#[doc(hidden)]` per
  library module.
- **The MongoDB suites are compiled once per `bson` major, and the live checks stand down.**
  `tests/mongodb_query_tests/` asserts the document each path writes, and
  `tests/mongodb_operation_tests/` the operations: `offline.rs` what needs no server, `live.rs`
  against a real collection. Each is compiled by a binary here, bound to version 2, and by one in
  `bson3/`. A live check reads `TIXSCHEMA_MONGODB_URI`; without it the check passes having run
  nothing, and says so once on the process's own stderr. `just test-mongodb` refuses to stand
  down, and sits outside `all` and `ci`: no MongoDB server comes with a clone or with the CI
  runner.
- **The README's examples are pinned.** Each is held as one text with the code that compiles, and
  what it prints is run: `tests/recovering_decode_bson_tests/readme.rs` (the callback) and
  `readme_resolvers.rs` (the resolvers), `tests/mongodb_operation_tests/readme.rs` with
  `readme/paths.rs`, `readme/operations.rs` and `readme/stored_forms.rs` (the model, every table
  of documents, the operations, what each failure is told as), and
  `tests/mongodb_query_tests/readme.rs` (a hooked date). A table row of a document is read off
  the call by `stringify!`, so the README's cell is the code. A line that ends in the `bson`
  library's own wording is shown under each major, and each binary holds the run to its own
  through its `BSON_MAJOR`. The README's operations run against a collection in
  `live_the_readme_examples_run_against_a_collection`, which stands down like every live check.
