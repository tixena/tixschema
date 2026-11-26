//! Python type generation: one `python_definition()` per `#[model_schema]` item, in a
//! `{snake}_python` module, dispatched from `exec_model_schema` the way the Dart, Swift and Kotlin
//! backends are.
//!
//! Each item's text is a frozen dataclass, or an `enum.Enum`, with full type hints and a JSON
//! codec written against the standard library alone: the read takes what serde reads and refuses
//! what serde refuses, in serde's words, and the write sends the form serde writes. A struct is a
//! draft, `{Name}Draft`, and a valid type, `{Name}`, which only `validate()`, `of(..)` and the
//! judged read make, as in the other clients.
//!
//! Every name the text binds at module scope carries the published name: the modules it imports
//! (`import json as _{Name}_json`) and the function that makes its valid type, so the texts of
//! every type of a crate join into one module with no name bound twice. The module's first line is
//! `from __future__ import annotations`, which the consumer writes.
//!
//! An item whose shape the emitter does not write yet earns no `{snake}_python` module: a bundle
//! that names its `python_definition()` stops at build time, rather than holding a class that
//! reads otherwise than serde. A holder of such a type, declared after it, earns none either.

extern crate alloc;

use alloc::collections::BTreeSet;
use core::cell::RefCell;
use core::fmt::Write as _;
use std::collections::{HashMap, HashSet};

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::ext::IdentExt as _;
use syn::token::Paren;
use syn::{
    Attribute, Field, Fields, GenericArgument, Ident, Item, ItemEnum, ItemStruct, PathArguments,
    Type,
};

use crate::features::model_schema_prop::{FunctionName, ModelSchemaPropMeta};
use crate::features::serde::{
    brand_field, parse_serde_field_attributes, parse_serde_key_omission,
    parse_serde_type_attributes, struct_tag,
};
use crate::field_type::{
    F32_READ_BOUND, FieldDef, FieldDefType, get_field_def, is_map_wrapper, is_plain_enum,
    is_sequence_wrapper,
};
use crate::model_schema::valid_type;
use crate::rename_rule::resolve_rename_rule;
use crate::utils::{compute_item_export_name, is_nonstrict, is_rule_less, to_snake_case};

thread_local! {
    /// The name each type a Python class has been written for publishes under, by Rust ident.
    static PYTHON_NAMES: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());

    /// Each type whose Python the emitter withheld, by Rust ident: a holder declared after it is
    /// withheld too.
    static PYTHON_WITHHELD: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

/// The words Python refuses as a name, `keyword.kwlist`: a member named after one is written with a
/// trailing underscore, as PEP 8 spells it. A soft keyword, `match`, `case`, `type` or `_`, names a
/// member as it is.
const PYTHON_RESERVED: [&str; 35] = [
    "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class", "continue",
    "def", "del", "elif", "else", "except", "finally", "for", "from", "global", "if", "import",
    "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while",
    "with", "yield",
];

/// The members a draft and a valid type write for themselves, which no field is named as: a field
/// of one of these names is written with a trailing underscore.
const GENERATED_MEMBERS: [&str; 11] = [
    "draft",
    "from_json",
    "from_json_value",
    "from_json_value_unjudged",
    "of",
    "to_draft",
    "to_json",
    "to_json_value",
    "to_valid",
    "validate",
    "validation_issues",
];

/// The names an `enum.Enum` reserves, which no variant is written as.
const ENUM_TAKEN: [&str; 2] = ["name", "value"];

/// The serde attributes a struct the emitter writes may carry on itself.
const STRUCT_SERDE_KEYS: [&str; 3] = ["deny_unknown_fields", "rename", "rename_all"];

/// The serde attributes a plain enum the emitter writes may carry on itself.
const ENUM_SERDE_KEYS: [&str; 3] = ["deny_unknown_fields", "rename", "rename_all"];

/// Reads JSON text as serde does where Python's `json` reads otherwise: `-0` is the float serde
/// reads it as, and an integer beyond `u64` and `i64` a float. A value serde's parser refuses, a
/// number beyond an `f64`, `NaN`, `Infinity` and `-Infinity`, is held as a `tix_refused` value of
/// serde's words, which the read of the value it stands for refuses at its place, as serde refuses
/// it where the parser reaches it. An object that writes a key twice keeps every pair in the order
/// written, in `tix_pairs`, for a read that walks them as serde does.
const PARSED: &str = r#"    @staticmethod
    def _tix_parsed(text: str | bytes | bytearray) -> object:
        class Refused:
            def __init__(self, said: str) -> None:
                self.tix_refused = said

        def fraction(digits: str) -> object:
            number = float(digits)
            if @M@math.isinf(number):
                return Refused("number out of range")
            return number

        def whole(digits: str) -> object:
            if digits == "-0":
                return -0.0
            if len(digits) > 21:
                return fraction(digits)
            number = int(digits)
            if -9223372036854775808 <= number <= 18446744073709551615:
                return number
            return fraction(digits)

        def constant(word: str) -> object:
            return Refused("invalid number" if word.startswith("-") else "expected value")

        class Repeated(dict[str, object]):
            tix_pairs: list[tuple[str, object]] = []

        def pairs(read: list[tuple[str, object]]) -> object:
            if len({key for key, _ in read}) == len(read):
                return dict(read)
            repeated = Repeated(read)
            repeated.tix_pairs = read
            return repeated

        return @M@json.loads(text, parse_int=whole, parse_float=fraction, parse_constant=constant, object_pairs_hook=pairs)
"#;

/// The refusal of `value` where a value of `expected` is read: the words serde's parser refused it
/// in, or serde's `invalid type`.
const INVALID: &str = r#"    @staticmethod
    def _tix_invalid(value: object, expected: str) -> ValueError:
        said: object = getattr(value, "tix_refused", None)
        if isinstance(said, str):
            return ValueError(said, [])
        return ValueError("invalid type: " + @C@._tix_unexpected(value) + ", expected " + expected, [])
"#;

/// The pairs of an object, in the order the text writes them, a key written twice included.
const PAIRS: &str = r#"    @staticmethod
    def _tix_pairs(value: object, expected: str) -> list[tuple[str, object]]:
        repeated: object = getattr(value, "tix_pairs", None)
        if isinstance(repeated, list):
            return @M@typing.cast("list[tuple[str, object]]", repeated)
        if not isinstance(value, dict):
            raise @C@._tix_invalid(value, expected)
        held = @M@typing.cast("dict[object, object]", value)
        return [(str(key), item) for key, item in held.items()]
"#;

/// The value of a key a nonstrict struct reads past, refused where serde's parser refuses it while
/// it skips the value: at `NaN`, `Infinity` and `-Infinity`, and not at a number beyond an `f64`,
/// which serde skips unread. serde places the refusal at the key alone.
const IGNORED: &str = r#"    @staticmethod
    def _tix_ignored(value: object) -> None:
        said: object = getattr(value, "tix_refused", None)
        if isinstance(said, str) and said != "number out of range":
            raise ValueError(said, [])
        repeated: object = getattr(value, "tix_pairs", None)
        if isinstance(repeated, list):
            for pair in @M@typing.cast("list[tuple[str, object]]", repeated):
                @C@._tix_ignored(pair[1])
        elif isinstance(value, list):
            for item in @M@typing.cast("list[object]", value):
                @C@._tix_ignored(item)
        elif isinstance(value, dict):
            for item in @M@typing.cast("dict[object, object]", value).values():
                @C@._tix_ignored(item)
"#;

/// Runs `read`, placing what it refuses under `step`.
const AT: &str = "    @staticmethod
    def _tix_at[R](step: str | int, read: @M@typing.Callable[[], R]) -> R:
        try:
            return read()
        except ValueError as refused:
            said: str = str(refused.args[0]) if refused.args else str(refused)
            steps: list[str | int] = refused.args[1] if len(refused.args) == 2 else []
            raise ValueError(said, [step, *steps]) from None
";

/// What serde's `invalid type` names a value read as.
const UNEXPECTED: &str = r#"    @staticmethod
    def _tix_unexpected(value: object) -> str:
        if value is None:
            return "null"
        if isinstance(value, bool):
            return "boolean `" + ("true" if value else "false") + "`"
        if isinstance(value, int):
            return "integer `" + str(value) + "`"
        if isinstance(value, float):
            return "floating point `" + @C@._tix_spelled(value) + "`"
        if isinstance(value, str):
            return "string " + @M@json.dumps(value, ensure_ascii=False)
        if isinstance(value, list):
            return "sequence"
        return "map"
"#;

/// A float as serde spells it in a refusal.
const SPELLED: &str = r#"    @staticmethod
    def _tix_spelled(number: float) -> str:
        if number == 0:
            return "-0.0" if @M@math.copysign(1.0, number) < 0 else "0.0"
        shown = repr(abs(number))
        mark = shown.find("e")
        mantissa = shown if mark < 0 else shown[:mark]
        every = mantissa.replace(".", "", 1)
        lead = len(every) - len(every.lstrip("0"))
        digits = every[lead:].rstrip("0")
        point = mantissa.index(".") if "." in mantissa else len(mantissa)
        at = (0 if mark < 0 else int(shown[mark + 1:])) + point - 1 - lead
        sign = "-" if number < 0 else ""
        if 0 <= at <= 15:
            padded = digits.ljust(at + 1, "0")
            return sign + padded[:at + 1] + "." + (digits[at + 1:] or "0")
        if -5 <= at < 0:
            return sign + "0." + "0" * (-at - 1) + digits
        rest = "." + digits[1:] if len(digits) > 1 else ""
        return sign + digits[0] + rest + "e" + ("+" if at > 0 else "") + str(at)
"#;

const LIST: &str = r#"    @staticmethod
    def _tix_list(value: object) -> list[object]:
        if not isinstance(value, list):
            raise @C@._tix_invalid(value, "a sequence")
        return list(@M@typing.cast("list[object]", value))
"#;

const STR: &str = r#"    @staticmethod
    def _tix_str(value: object) -> str:
        if not isinstance(value, str):
            raise @C@._tix_invalid(value, "a string")
        return value
"#;

const BOOL: &str = r#"    @staticmethod
    def _tix_bool(value: object) -> bool:
        if not isinstance(value, bool):
            raise @C@._tix_invalid(value, "a boolean")
        return value
"#;

const INT: &str = r#"    @staticmethod
    def _tix_int(value: object, low: int, high: int, named: str) -> int:
        if isinstance(value, bool) or not isinstance(value, int):
            raise @C@._tix_invalid(value, named)
        if not low <= value <= high:
            raise ValueError("invalid value: integer `" + str(value) + "`, expected " + named, [])
        return value
"#;

const FLOAT: &str = "    @staticmethod
    def _tix_float(value: object, named: str) -> float:
        if isinstance(value, bool) or not isinstance(value, (int, float)):
            raise @C@._tix_invalid(value, named)
        return float(value)
";

/// An `f32` as serde reads it: the number rounded to an `f32`, and one of `F32_READ_BOUND` or more
/// in size an infinity of its sign.
const F32: &str = r#"    @staticmethod
    def _tix_f32(value: object) -> float:
        number = @C@._tix_float(value, "f32")
        if abs(number) >= @F32@:
            return @M@math.copysign(@M@math.inf, number)
        return @C@._tix_single(number)
"#;

const SENT_INT: &str = r#"    @staticmethod
    def _tix_sent_int(value: int, low: int, high: int) -> int:
        if isinstance(value, bool) or not low <= value <= high:
            raise ValueError("expected an integer from " + str(low) + " to " + str(high) + ", found " + str(value), [])
        return value
"#;

const SENT_FLOAT: &str = r#"    @staticmethod
    def _tix_sent_float(value: float) -> float:
        if isinstance(value, bool) or not @M@math.isfinite(value):
            raise ValueError("expected a finite number, found " + str(value), [])
        return value
"#;

const SENT_F32: &str = r#"    @staticmethod
    def _tix_sent_f32(value: float) -> float:
        if isinstance(value, bool) or not @M@math.isfinite(value) or abs(value) >= @F32@:
            raise ValueError("expected a finite number an f32 holds, found " + str(value), [])
        return @C@._tix_single(value)
"#;

/// An `f32` as the shortest digits that read back as the same `f32`, as serde writes one, held as a
/// float: what Rust displays of the value. A number past `f32::MAX` and short of the bound an `f32`
/// overflows at rounds to `f32::MAX`, which `struct` would refuse to pack, and one at the bound or
/// past it to an infinity.
const SINGLE: &str = r#"    @staticmethod
    def _tix_single(number: float) -> float:
        def rounded(wide: float) -> float:
            if abs(wide) >= @F32@:
                return @M@math.copysign(@M@math.inf, wide)
            if abs(wide) > 3.4028234663852886e38:
                wide = @M@math.copysign(3.4028234663852886e38, wide)
            narrow: float = @M@struct.unpack("<f", @M@struct.pack("<f", wide))[0]
            return narrow

        held = rounded(number)
        for digits in range(1, 10):
            shown = float(format(held, "." + str(digits) + "g"))
            if rounded(shown) == held:
                return shown
        return held
"#;

/// A JSON value as serde writes it: no space, each text as serde escapes it, and each float spelled
/// as serde spells it, where Python's `json` writes `1e-05` for serde's `0.00001`.
const DUMPS: &str = r#"    @staticmethod
    def _tix_dumps(value: object) -> str:
        if value is None:
            return "null"
        if isinstance(value, bool):
            return "true" if value else "false"
        if isinstance(value, int):
            return str(value)
        if isinstance(value, float):
            return @C@._tix_spelled(value)
        if isinstance(value, str):
            return @M@json.dumps(value, ensure_ascii=False)
        if isinstance(value, list):
            items = @M@typing.cast("list[object]", value)
            return "[" + ",".join(@C@._tix_dumps(item) for item in items) + "]"
        pairs = @M@typing.cast("dict[object, object]", value)
        return "{" + ",".join(@M@json.dumps(str(key), ensure_ascii=False) + ":" + @C@._tix_dumps(item) for key, item in pairs.items()) + "}"
"#;

/// Places what a held value's `validation_issues()` says under the member that holds it.
const UNDER: &str = r#"    @staticmethod
    def _tix_under(place: str, said: str) -> str:
        if said.startswith("'"):
            end = said.find("'", 1)
            if end > 0:
                return "'" + place + "." + said[1:end] + "'" + said[end + 1:]
        return "'" + place + "': " + said
"#;

/// The text of a struct's refusal, its draft, its valid type and the function that makes one.
/// `@NAME@` is the name the struct publishes, `@DRAFT@` its draft's, and each other marker a part
/// `struct_source` writes: each line of `@FIELDS@`, `@READS@`, `@RETURNED@`, `@WRITES@` and
/// `@WALKS@` ends with a line break.
const STRUCT_TEXT: &str = r#"

class @REFUSAL@(Exception):
    def __init__(self, issues: list[str]) -> None:
        super().__init__(issues)
        self.issues: list[str] = issues


@@DATACLASSES@.dataclass(frozen=True, slots=True, kw_only=True)
class @DRAFT@:
@FIELDS@
    @classmethod
    def from_json(cls, text: str | bytes | bytearray) -> @DRAFT@:
        return cls.from_json_value(cls._tix_parsed(text))

    @classmethod
    def from_json_value(cls, value: object) -> @DRAFT@:
@READS@        return cls(
@RETURNED@        )

    def to_json(self) -> str:
        return self._tix_dumps(self.to_json_value())

    def to_json_value(self) -> dict[str, object]:
        written: dict[str, object] = {}
@WRITES@        return written

    def validation_issues(self) -> list[str]:
@WALKS@
    def validate(self) -> @NAME@:
        issues = self.validation_issues()
        if issues:
            raise @REFUSAL@(issues)
        return @VALID@(self)

    def to_valid(self) -> @NAME@:
        return @VALID@(self)

@HELPERS@

@@TYPING@.final
@@DATACLASSES@.dataclass(frozen=True, slots=True, init=False)
class @NAME@:
    draft: @DRAFT@

    def __init__(self) -> None:
        raise TypeError(@MADE_BY@)
@PROPERTIES@
    @staticmethod
    def of(*, @PARAMETERS@) -> @NAME@:
        return @DRAFT@(@ARGUMENTS@).validate()

    @staticmethod
    def from_json(text: str | bytes | bytearray) -> @NAME@:
        return @DRAFT@.from_json(text).validate()

    @staticmethod
    def from_json_value(value: object) -> @NAME@:
        return @DRAFT@.from_json_value(value).validate()

    @staticmethod
    def from_json_value_unjudged(value: object) -> @NAME@:
        return @VALID@(@DRAFT@.from_json_value(value))

    def to_draft(self) -> @DRAFT@:
        return self.draft

    def to_json(self) -> str:
        return self.draft.to_json()

    def to_json_value(self) -> dict[str, object]:
        return self.draft.to_json_value()

    def validation_issues(self) -> list[str]:
        return self.draft.validation_issues()


def @VALID@(draft: @DRAFT@) -> @NAME@:
    made = object.__new__(@NAME@)
    object.__setattr__(made, "draft", draft)
    return made"#;

/// The text of a plain enum: an `enum.Enum` whose every member is a variant's name, read as serde
/// reads a unit variant, its bare name or an object of the name over `null`. Each line of
/// `@VARIANTS@` ends with a line break.
const ENUM_TEXT: &str = r#"

class @NAME@(@ENUM@.Enum):
@VARIANTS@
    @classmethod
    def from_json(cls, text: str | bytes | bytearray) -> @NAME@:
        return cls.from_json_value(cls._tix_parsed(text))

    @classmethod
    def from_json_value(cls, value: object) -> @NAME@:
        said: object = getattr(value, "tix_refused", None)
        if isinstance(value, str):
            named: str = value
        elif isinstance(value, dict) and len(pairs := @TYPING@.cast("dict[object, object]", value)) == 1:
            ((key, held),) = pairs.items()
            named = str(key)
            if held is not None:
                raise ValueError(cls._tix_invalid(held, "unit").args[0], [named])
        else:
            raise ValueError(said if isinstance(said, str) else "expected value", [])
        for variant in cls:
            if variant.value == named:
                return variant
        raise ValueError("unknown variant `" + named + @EXPECTED@, [])

    @classmethod
    def from_json_value_unjudged(cls, value: object) -> @NAME@:
        return cls.from_json_value(value)

    def to_json(self) -> str:
        return @JSON@.dumps(self.to_json_value(), ensure_ascii=False)

    def to_json_value(self) -> str:
        return self.value

    def validation_issues(self) -> list[str]:
        return []

@HELPERS@"#;

/// What `#[model_schema(...)]` wrote on an item, as this module reads it.
pub struct PythonArgs<'args> {
    /// The functions `check` names, in the order written.
    pub check: &'args [FunctionName],
    /// A member's `model_schema_prop`, held to its enum's string check where that reaches it.
    pub member_meta: &'args dyn Fn(&Field) -> ModelSchemaPropMeta,
    pub name_override: Option<&'args str>,
    /// The functions `preprocess` names on the type, in the order written.
    pub preprocess: &'args [FunctionName],
    /// The Rust name of the item, which a member written `Self` is named as.
    pub type_name: &'args str,
}

/// The helpers a class calls, each a static method written once into the class that calls it.
#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
enum Helper {
    At,
    Bool,
    Dumps,
    F32,
    Float,
    Ignored,
    Int,
    Invalid,
    List,
    Pairs,
    Parsed,
    SentF32,
    SentFloat,
    SentInt,
    Single,
    Spelled,
    Str,
    Under,
    Unexpected,
}

impl Helper {
    /// The helpers this one calls.
    const fn calls(self) -> &'static [Self] {
        match self {
            Self::Bool | Self::Float | Self::Int | Self::List | Self::Pairs | Self::Str => {
                &[Self::Invalid]
            }
            Self::Invalid => &[Self::Unexpected],
            Self::F32 => &[Self::Float, Self::Single],
            Self::Dumps | Self::Unexpected => &[Self::Spelled],
            Self::SentF32 => &[Self::Single],
            Self::At
            | Self::Ignored
            | Self::Parsed
            | Self::SentFloat
            | Self::SentInt
            | Self::Single
            | Self::Spelled
            | Self::Under => &[],
        }
    }

    /// The modules this helper names, each imported under the class's own prefix.
    const fn modules(self) -> &'static [&'static str] {
        match self {
            Self::At | Self::Ignored | Self::List | Self::Pairs => &["typing"],
            Self::Dumps => &["json", "typing"],
            Self::Parsed => &["json", "math"],
            Self::Single => &["math", "struct"],
            Self::F32 | Self::SentF32 | Self::SentFloat | Self::Spelled => &["math"],
            Self::Unexpected => &["json"],
            Self::Bool
            | Self::Float
            | Self::Int
            | Self::Invalid
            | Self::SentInt
            | Self::Str
            | Self::Under => &[],
        }
    }

    /// The method's text, indented for a class body: `@C@` is the class that holds it, `@M@` the
    /// prefix of the modules the text imports.
    const fn text(self) -> &'static str {
        match self {
            Self::At => AT,
            Self::Bool => BOOL,
            Self::Dumps => DUMPS,
            Self::F32 => F32,
            Self::Float => FLOAT,
            Self::Ignored => IGNORED,
            Self::Int => INT,
            Self::Invalid => INVALID,
            Self::List => LIST,
            Self::Pairs => PAIRS,
            Self::Parsed => PARSED,
            Self::SentF32 => SENT_F32,
            Self::SentFloat => SENT_FLOAT,
            Self::SentInt => SENT_INT,
            Self::Single => SINGLE,
            Self::Spelled => SPELLED,
            Self::Str => STR,
            Self::Under => UNDER,
            Self::Unexpected => UNEXPECTED,
        }
    }
}

/// What one class's text asks for while it is written: the helpers it calls, the modules it
/// imports, and a counter that keeps the names of nested comprehensions apart.
struct Writing {
    class: String,
    counter: usize,
    helpers: BTreeSet<Helper>,
    modules: BTreeSet<&'static str>,
    prefix: String,
}

impl Writing {
    /// Asks for `helper`, and for every helper it calls.
    fn ask(&mut self, helper: Helper) -> String {
        if self.helpers.insert(helper) {
            self.modules.extend(helper.modules());
            for called in helper.calls() {
                self.ask(*called);
            }
        }
        self.class.clone()
    }

    /// A fresh suffix for the names a comprehension binds.
    const fn fresh(&mut self) -> usize {
        self.counter += 1;
        self.counter
    }

    /// The helpers asked for, as the methods of the class.
    fn helper_methods(&self) -> String {
        self.helpers
            .iter()
            .map(|helper| {
                filled(
                    helper.text(),
                    &[
                        ("C", &self.class),
                        ("F32", F32_READ_BOUND),
                        ("M", &self.prefix),
                    ],
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The imports the class's text names, each under its prefix.
    fn imports(&self) -> String {
        self.modules
            .iter()
            .fold(String::new(), |mut written, module| {
                let _ = writeln!(written, "import {module} as {}{module}", self.prefix);
                written
            })
    }

    /// The module `module`, imported under the class's prefix.
    fn module(&mut self, module: &'static str) -> String {
        self.modules.insert(module);
        format!("{}{module}", self.prefix)
    }

    fn new(class: &str, export_name: &str) -> Self {
        Self {
            class: class.to_owned(),
            counter: 0,
            helpers: BTreeSet::new(),
            modules: BTreeSet::new(),
            prefix: format!("_{export_name}_"),
        }
    }
}

/// One member of a struct, as its draft declares, reads and writes it.
struct Member {
    field: FieldDef,
    hint: String,
    name: String,
    /// Whether serde writes the member as `null` where it holds `None`, rather than leaving its key
    /// out.
    nullable: bool,
    /// Whether each map the member holds writes its keys in order: see [`sorted_maps`].
    sorted_maps: Vec<bool>,
    wire: String,
}

/// The text each member adds to a struct's two classes: its field, its property, its parameter of
/// `of(..)` and the argument `of(..)` hands the draft.
struct Declarations {
    arguments: String,
    fields: String,
    parameters: String,
    properties: String,
}

impl Declarations {
    /// The declarations of `members`, in the order declared.
    fn of(members: &[Member]) -> Self {
        let mut fields = String::new();
        let mut properties = String::new();
        let mut parameters = Vec::new();
        let mut arguments = Vec::new();
        for member in members {
            let default = if member.field.is_optional() {
                " = None"
            } else {
                ""
            };
            let (name, hint) = (&member.name, &member.hint);
            let _ = writeln!(fields, "    {name}: {hint}{default}");
            let _ = write!(
                properties,
                "\n    @property\n    def {name}(self) -> {hint}:\n        return self.draft.{name}\n"
            );
            parameters.push(format!("{name}: {hint}{default}"));
            arguments.push(format!("{name}={name}"));
        }
        Self {
            arguments: arguments.join(", "),
            fields,
            parameters: parameters.join(", "),
            properties,
        }
    }
}

/// The Python tokens `item` earns: the `{snake}_python` module of a shape the emitter writes, and
/// nothing for any other.
pub fn python_schema_dispatch(item: &Item, args: &PythonArgs<'_>) -> TokenStream {
    let written = if let Item::Struct(item_struct) = item {
        struct_source(item_struct, args).map(|source| (&item_struct.ident, source))
    } else if let Item::Enum(item_enum) = item {
        enum_source(item_enum, args).map(|source| (&item_enum.ident, source))
    } else {
        // An alias is written by no Python yet.
        None
    };
    let Some((ident, source)) = written else {
        withhold(args.type_name);
        return TokenStream::new();
    };
    let module_ident = python_module_ident(&ident.to_string(), ident.span());
    quote! {
        pub mod #module_ident {
            pub fn python_definition() -> ::std::string::String {
                ::std::borrow::ToOwned::to_owned(#source)
            }
        }
    }
}

/// The `{ident}_python` module an item's own `python_definition()` publishes from.
pub fn python_module_ident(rust_ident: &str, span: Span) -> Ident {
    Ident::new(&format!("{}_python", to_snake_case(rust_ident)), span)
}

/// The refusal class of the type published as `export_name`, named as every client names it
/// (`refusals::refusal_name`).
fn refusal_name(export_name: &str) -> String {
    format!("{export_name}Refusal")
}

/// Records that the type `rust_ident` earns no Python.
fn withhold(rust_ident: &str) {
    PYTHON_NAMES.with(|names| names.borrow_mut().remove(rust_ident));
    PYTHON_WITHHELD.with(|withheld| withheld.borrow_mut().insert(rust_ident.to_owned()));
}

/// Records the name the type `rust_ident` publishes its Python under.
fn register(rust_ident: &str, export_name: &str) {
    PYTHON_WITHHELD.with(|withheld| withheld.borrow_mut().remove(rust_ident));
    PYTHON_NAMES.with(|names| {
        names
            .borrow_mut()
            .insert(rust_ident.to_owned(), export_name.to_owned());
    });
}

/// The Python name of the type a member names as `rust_ident`, or `None` where its Python is
/// withheld. A type no expansion has met yet is named as written.
fn named_type(rust_ident: &str) -> Option<String> {
    if PYTHON_WITHHELD.with(|withheld| withheld.borrow().contains(rust_ident)) {
        return None;
    }
    Some(
        PYTHON_NAMES
            .with(|names| names.borrow().get(rust_ident).cloned())
            .unwrap_or_else(|| rust_ident.to_owned()),
    )
}

/// The key of every serde attribute in `attrs`, beside the text of the string it is set to where it
/// is set to one, or `None` where one is written in a form this module does not read: a key with a
/// parenthesized list, such as `rename(serialize = "..")`.
fn serde_keys(attrs: &[Attribute]) -> Option<Vec<(String, Option<String>)>> {
    let mut keys = Vec::new();
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("serde")) {
        attr.parse_nested_meta(|nested| {
            if nested.input.peek(Paren) {
                return Err(nested.error("a parenthesized serde attribute"));
            }
            let value = if nested.input.peek(syn::Token![=]) {
                let set_to = nested.value()?.parse::<syn::Expr>()?;
                if let syn::Expr::Lit(literal) = set_to
                    && let syn::Lit::Str(text) = literal.lit
                {
                    Some(text.value())
                } else {
                    None
                }
            } else {
                None
            };
            let key = nested
                .path
                .get_ident()
                .map_or_else(String::new, ToString::to_string);
            keys.push((key, value));
            Ok(())
        })
        .ok()?;
    }
    Some(keys)
}

/// Whether every serde attribute of `attrs` is one of `allowed`.
fn only_serde_keys(attrs: &[Attribute], allowed: &[&str]) -> bool {
    serde_keys(attrs)
        .is_some_and(|keys| keys.iter().all(|(key, _)| allowed.contains(&key.as_str())))
}

/// A Python string literal holding `text`.
fn quoted(text: &str) -> String {
    let mut written = String::from("\"");
    for character in text.chars() {
        match character {
            '\\' => written.push_str("\\\\"),
            '"' => written.push_str("\\\""),
            '\n' => written.push_str("\\n"),
            '\r' => written.push_str("\\r"),
            '\t' => written.push_str("\\t"),
            control if control.is_control() => {
                let _ = write!(written, "\\U{:08x}", u32::from(control));
            }
            other => written.push(other),
        }
    }
    written.push('"');
    written
}

/// What serde writes after `expected` naming `names`: one name, two joined by `or`, or a list.
fn one_of(names: &[String]) -> String {
    let ticked: Vec<String> = names.iter().map(|name| format!("`{name}`")).collect();
    match ticked.as_slice() {
        [] => String::new(),
        [only] => only.clone(),
        [first, second] => format!("{first} or {second}"),
        [..] => format!("one of {}", ticked.join(", ")),
    }
}

/// The name a member written `rust_name` is declared under: the name itself, or with a trailing
/// underscore where Python or a generated member takes it, then until no other member holds it.
fn python_member(rust_name: &str, taken: &[String]) -> String {
    let mut member = rust_name.to_owned();
    if PYTHON_RESERVED.contains(&member.as_str()) || GENERATED_MEMBERS.contains(&member.as_str()) {
        member.push('_');
    }
    while taken.contains(&member) {
        member.push('_');
    }
    member
}

/// The range, and the name serde says, of each integer type.
const fn integer_bounds(
    field_type: &FieldDefType,
) -> Option<(&'static str, &'static str, &'static str)> {
    if matches!(field_type, FieldDefType::U8) {
        Some(("0", "255", "u8"))
    } else if matches!(field_type, FieldDefType::U16) {
        Some(("0", "65535", "u16"))
    } else if matches!(field_type, FieldDefType::U32) {
        Some(("0", "4294967295", "u32"))
    } else if matches!(field_type, FieldDefType::U64) {
        Some(("0", "18446744073709551615", "u64"))
    } else if matches!(field_type, FieldDefType::Usize) {
        Some(("0", "18446744073709551615", "usize"))
    } else if matches!(field_type, FieldDefType::I8) {
        Some(("-128", "127", "i8"))
    } else if matches!(field_type, FieldDefType::I16) {
        Some(("-32768", "32767", "i16"))
    } else if matches!(field_type, FieldDefType::I32) {
        Some(("-2147483648", "2147483647", "i32"))
    } else if matches!(field_type, FieldDefType::I64) {
        Some(("-9223372036854775808", "9223372036854775807", "i64"))
    } else if matches!(field_type, FieldDefType::Isize) {
        Some(("-9223372036854775808", "9223372036854775807", "isize"))
    } else {
        None
    }
}

/// Whether `field` is held at every level in a form this module writes: no set, no array of a
/// fixed length, and no list of bytes.
const fn levels_written(field: &FieldDef) -> bool {
    field.set_levels.is_empty() && field.array_lengths.is_empty() && !field.bytes
}

/// The model type `field` names directly, a type with no type parameter written by its name.
fn model_named(field: &FieldDef) -> Option<&str> {
    if let FieldDefType::SiblingType(name, generics) = &field.field_type
        && generics.is_empty()
        && !is_sequence_wrapper(name)
    {
        Some(name.as_str())
    } else {
        None
    }
}

/// The value of a map keyed by a plain `String`, which is the one map this module writes.
fn string_keyed_value(field: &FieldDef) -> Option<&FieldDef> {
    let FieldDefType::Map(key, value) = &field.field_type else {
        return None;
    };
    let plain_key = matches!(key.field_type, FieldDefType::String)
        && key.array_depth == 0
        && key.nullable_levels.is_empty();
    plain_key.then_some(value.as_ref())
}

/// The type hint of `field`'s innermost value, or `None` for a type this module does not write.
fn base_hint(field: &FieldDef) -> Option<String> {
    if let Some(name) = model_named(field) {
        return named_type(name);
    }
    if let Some(value) = string_keyed_value(field) {
        return Some(format!("dict[str, {}]", hint(value)?));
    }
    let scalar = &field.field_type;
    if matches!(scalar, FieldDefType::String) {
        Some("str".to_owned())
    } else if matches!(scalar, FieldDefType::Boolean) {
        Some("bool".to_owned())
    } else if matches!(scalar, FieldDefType::F32 | FieldDefType::F64) {
        Some("float".to_owned())
    } else {
        integer_bounds(scalar).map(|_| "int".to_owned())
    }
}

/// The type hint of `field` at `level`, `level` 0 being its innermost value.
fn hint_at(field: &FieldDef, level: u8) -> Option<String> {
    let inner = if level == 0 {
        base_hint(field)?
    } else {
        format!("list[{}]", hint_at(field, level - 1)?)
    };
    Some(if field.is_nullable_at(level) {
        format!("{inner} | None")
    } else {
        inner
    })
}

/// The type hint of `field`, or `None` for a type this module does not write.
fn hint(field: &FieldDef) -> Option<String> {
    if !levels_written(field) {
        return None;
    }
    hint_at(field, field.array_depth)
}

/// The expression that reads `held`, a value JSON parsed, as `field`'s innermost value.
fn base_read(field: &FieldDef, held: &str, writing: &mut Writing) -> Option<String> {
    if let Some(name) = model_named(field) {
        return Some(format!(
            "{}.from_json_value_unjudged({held})",
            named_type(name)?
        ));
    }
    if let Some(value) = string_keyed_value(field) {
        let suffix = writing.fresh();
        let (key, item) = (format!("key{suffix}"), format!("item{suffix}"));
        let inner = read(value, &item, writing)?;
        let class = writing.ask(Helper::At);
        writing.ask(Helper::Pairs);
        // Each pair in the order written, a key written twice included: serde reads every value
        // and keeps the last.
        return Some(format!(
            "{{{key}: {class}._tix_at({key}, lambda: {inner}) for {key}, {item} in \
             {class}._tix_pairs({held}, \"a map\")}}"
        ));
    }
    let scalar = &field.field_type;
    if matches!(scalar, FieldDefType::String) {
        Some(format!("{}._tix_str({held})", writing.ask(Helper::Str)))
    } else if matches!(scalar, FieldDefType::Boolean) {
        Some(format!("{}._tix_bool({held})", writing.ask(Helper::Bool)))
    } else if matches!(scalar, FieldDefType::F64) {
        Some(format!(
            "{}._tix_float({held}, \"f64\")",
            writing.ask(Helper::Float)
        ))
    } else if matches!(scalar, FieldDefType::F32) {
        Some(format!("{}._tix_f32({held})", writing.ask(Helper::F32)))
    } else {
        let (low, high, named) = integer_bounds(scalar)?;
        Some(format!(
            "{}._tix_int({held}, {low}, {high}, \"{named}\")",
            writing.ask(Helper::Int)
        ))
    }
}

/// The expression that reads `held` as `field` at `level`, past the level's own `null`.
fn read_present(field: &FieldDef, level: u8, held: &str, writing: &mut Writing) -> Option<String> {
    if level == 0 {
        return base_read(field, held, writing);
    }
    let suffix = writing.fresh();
    let (at, item) = (format!("at{suffix}"), format!("item{suffix}"));
    let inner = read_at(field, level - 1, &item, writing)?;
    let class = writing.ask(Helper::At);
    writing.ask(Helper::List);
    Some(format!(
        "[{class}._tix_at({at}, lambda: {inner}) for {at}, {item} in \
         enumerate({class}._tix_list({held}))]"
    ))
}

/// The expression that reads `held` as `field` at `level`.
fn read_at(field: &FieldDef, level: u8, held: &str, writing: &mut Writing) -> Option<String> {
    let present = read_present(field, level, held, writing)?;
    Some(if field.is_nullable_at(level) {
        format!("(None if {held} is None else {present})")
    } else {
        present
    })
}

/// The expression that reads `held` as `field`.
fn read(field: &FieldDef, held: &str, writing: &mut Writing) -> Option<String> {
    read_at(field, field.array_depth, held, writing)
}

/// The expression that writes `held`, a value of `field`'s innermost type, in the form serde
/// writes it. `maps` says, of each map `field` holds from here in, whether it writes its keys in
/// order.
fn base_write(
    field: &FieldDef,
    held: &str,
    maps: &[bool],
    writing: &mut Writing,
) -> Option<String> {
    if model_named(field).is_some() {
        return Some(format!("{held}.to_json_value()"));
    }
    if let Some(value) = string_keyed_value(field) {
        let (sorted, within) = maps
            .split_first()
            .map_or((false, maps), |(first, rest)| (*first, rest));
        let suffix = writing.fresh();
        let (key, item) = (format!("key{suffix}"), format!("item{suffix}"));
        let inner = write(value, &item, within, writing)?;
        let pairs = if sorted {
            format!("sorted({held}.items())")
        } else {
            format!("{held}.items()")
        };
        return Some(match (inner == item, sorted) {
            (true, false) => held.to_owned(),
            (true, true) => format!("dict({pairs})"),
            (false, _) => format!("{{{key}: {inner} for {key}, {item} in {pairs}}}"),
        });
    }
    let scalar = &field.field_type;
    if matches!(scalar, FieldDefType::String | FieldDefType::Boolean) {
        Some(held.to_owned())
    } else if matches!(scalar, FieldDefType::F64) {
        Some(format!(
            "{}._tix_sent_float({held})",
            writing.ask(Helper::SentFloat)
        ))
    } else if matches!(scalar, FieldDefType::F32) {
        Some(format!(
            "{}._tix_sent_f32({held})",
            writing.ask(Helper::SentF32)
        ))
    } else {
        let (low, high, _) = integer_bounds(scalar)?;
        Some(format!(
            "{}._tix_sent_int({held}, {low}, {high})",
            writing.ask(Helper::SentInt)
        ))
    }
}

/// The expression that writes `held` as `field` at `level`.
fn write_at(
    field: &FieldDef,
    level: u8,
    held: &str,
    maps: &[bool],
    writing: &mut Writing,
) -> Option<String> {
    let present = if level == 0 {
        base_write(field, held, maps, writing)?
    } else {
        let suffix = writing.fresh();
        let item = format!("item{suffix}");
        let inner = write_at(field, level - 1, &item, maps, writing)?;
        if inner == item {
            held.to_owned()
        } else {
            format!("[{inner} for {item} in {held}]")
        }
    };
    Some(if field.is_nullable_at(level) && present != held {
        format!("(None if {held} is None else {present})")
    } else {
        present
    })
}

/// The expression that writes `held` as `field`.
fn write(field: &FieldDef, held: &str, maps: &[bool], writing: &mut Writing) -> Option<String> {
    write_at(field, field.array_depth, held, maps, writing)
}

/// Whether each map `ty` holds is a `BTreeMap`, which serde writes in the order of its keys, where
/// a `HashMap` or an `IndexMap` writes them in the order it holds them: the map the member's levels
/// hold first, then the map that map's values hold, and so on, the chain `FieldDef` nests them in.
fn sorted_maps(ty: &Type) -> Vec<bool> {
    let mut sorted = Vec::new();
    let mut at = ty;
    while let Type::Path(path) = at
        && let Some(last) = path.path.segments.last()
        && let PathArguments::AngleBracketed(written) = &last.arguments
    {
        let held: Vec<&Type> = written
            .args
            .iter()
            .filter_map(|argument| {
                if let GenericArgument::Type(inner) = argument {
                    Some(inner)
                } else {
                    None
                }
            })
            .collect();
        let named = last.ident.to_string();
        if let [_, value] = held.as_slice()
            && is_map_wrapper(&named)
        {
            sorted.push(named == "BTreeMap");
            at = value;
        } else if let [inner] = held.as_slice() {
            at = inner;
        } else {
            break;
        }
    }
    sorted
}

/// Whether a value of `field` holds a model, whose `validation_issues()` its holder runs.
fn holds_a_model(field: &FieldDef) -> bool {
    model_named(field).is_some() || string_keyed_value(field).is_some_and(holds_a_model)
}

/// The statements, at `indent`, that run `validation_issues()` of each model `held`, a value of
/// `field` at `level`, holds, each placed under `place`, a Python expression of the place.
fn walk_at(
    field: &FieldDef,
    level: u8,
    walked: (&str, &str),
    indent: usize,
    writing: &mut Writing,
    lines: &mut Vec<String>,
) {
    let (held, place) = walked;
    let pad = "    ".repeat(indent);
    let inner_indent = if field.is_nullable_at(level) {
        lines.push(format!("{pad}if {held} is not None:"));
        indent + 1
    } else {
        indent
    };
    let inner_pad = "    ".repeat(inner_indent);
    if level > 0 {
        let suffix = writing.fresh();
        let (at, item) = (format!("at{suffix}"), format!("item{suffix}"));
        lines.push(format!("{inner_pad}for {at}, {item} in enumerate({held}):"));
        let under = format!("{place} + \".\" + str({at})");
        walk_at(
            field,
            level - 1,
            (&item, &under),
            inner_indent + 1,
            writing,
            lines,
        );
    } else if let Some(value) = string_keyed_value(field) {
        let suffix = writing.fresh();
        let (key, item) = (format!("key{suffix}"), format!("item{suffix}"));
        lines.push(format!("{inner_pad}for {key}, {item} in {held}.items():"));
        let under = format!("{place} + \".\" + {key}");
        walk_at(
            value,
            value.array_depth,
            (&item, &under),
            inner_indent + 1,
            writing,
            lines,
        );
    } else {
        let class = writing.ask(Helper::Under);
        lines.push(format!(
            "{inner_pad}issues.extend({class}._tix_under({place}, said) for said in \
             {held}.validation_issues())"
        ));
    }
}

/// Whether `meta` asks nothing of a member that this module does not write: a bound, a check, a
/// `preprocess`, a literal or a type written over the member's own.
const fn member_meta_written(meta: &ModelSchemaPropMeta) -> bool {
    meta.as_type.is_none()
        && !meta.as_number
        && meta.check.is_empty()
        && meta.literal.is_none()
        && meta.preprocess.is_empty()
        && meta.enum_pattern.is_none()
        && meta.pattern.is_none()
        && meta.min_length.is_none()
        && meta.max_length.is_none()
        && meta.minimum.is_none()
        && meta.maximum.is_none()
        && meta.min_items.is_none()
        && meta.max_items.is_none()
}

/// The members of `item_struct` this module writes, or `None` where one is written in a form it
/// does not write yet.
fn struct_members(item_struct: &ItemStruct, args: &PythonArgs<'_>) -> Option<Vec<Member>> {
    let Fields::Named(named) = &item_struct.fields else {
        return None;
    };
    let rule = resolve_rename_rule(
        parse_serde_type_attributes(&item_struct.attrs)
            .rename_all
            .as_deref(),
    );
    let mut members: Vec<Member> = Vec::new();
    for field in &named.named {
        let rust_name = field.ident.as_ref()?.unraw().to_string();
        let omission = parse_serde_key_omission(&field.attrs);
        if omission.absent_from_wire() {
            continue;
        }
        if rust_name.starts_with("__") || parse_serde_field_attributes(&field.attrs).flatten {
            return None;
        }
        let meta = (args.member_meta)(field);
        if !member_meta_written(&meta) {
            return None;
        }
        let mut field_def = get_field_def(&rust_name, &field.ty, "");
        field_def.named_within(args.type_name, &[]);
        let keys = serde_keys(&field.attrs)?;
        let optional = field_def.is_optional();
        let keys_written = keys.iter().all(|(key, value)| match key.as_str() {
            "rename" => true,
            "default" => optional,
            // The write leaves the key out where the member holds `None`, and writes it otherwise.
            "skip_serializing_if" => {
                optional
                    && value
                        .as_deref()
                        .is_some_and(|path| path.ends_with("is_none"))
            }
            _ => false,
        });
        if !keys_written || omission.skips_deserializing {
            return None;
        }
        let taken: Vec<String> = members.iter().map(|member| member.name.clone()).collect();
        members.push(Member {
            hint: hint(&field_def)?,
            name: python_member(&rust_name, &taken),
            nullable: meta.nullable,
            sorted_maps: sorted_maps(&field.ty),
            wire: parse_serde_field_attributes(&field.attrs)
                .rename
                .unwrap_or_else(|| rule.apply_to_field(&rust_name)),
            field: field_def,
        });
    }
    (!members.is_empty()).then_some(members)
}

/// The Python text of `item_struct`, its draft and its valid type, or `None` for a struct whose
/// shape this module does not write yet.
fn struct_source(item_struct: &ItemStruct, args: &PythonArgs<'_>) -> Option<String> {
    let writes_its_shape = valid_type::has_draft()
        && is_rule_less()
        && args.check.is_empty()
        && args.preprocess.is_empty()
        && item_struct.generics.params.is_empty()
        && brand_field(item_struct).is_none()
        && struct_tag(item_struct).is_none()
        && only_serde_keys(&item_struct.attrs, &STRUCT_SERDE_KEYS);
    if !writes_its_shape {
        return None;
    }
    let members = struct_members(item_struct, args)?;
    let rust_ident = item_struct.ident.unraw().to_string();
    let export_name = compute_item_export_name(&rust_ident, args.name_override);
    let draft = format!("{export_name}Draft");
    let mut writing = Writing::new(&draft, &export_name);
    let described = parse_serde_field_attributes(&item_struct.attrs)
        .rename
        .unwrap_or_else(|| rust_ident.clone());
    let (reads, returned) = member_reads(&members, &described, &mut writing)?;
    let writes = member_writes(&members, &mut writing)?;
    let walks = member_walks(&members, &mut writing);
    writing.ask(Helper::Parsed);
    writing.ask(Helper::Dumps);
    let dataclasses = writing.module("dataclasses");
    let typing = writing.module("typing");
    let declared = Declarations::of(&members);
    let made_by = quoted(&format!(
        "a {export_name} is made by {draft}.validate(), {export_name}.of() or \
         {export_name}.from_json()"
    ));
    let text = filled(
        STRUCT_TEXT,
        &[
            ("ARGUMENTS", &declared.arguments),
            ("DATACLASSES", &dataclasses),
            ("DRAFT", &draft),
            ("FIELDS", &declared.fields),
            ("HELPERS", &writing.helper_methods()),
            ("MADE_BY", &made_by),
            ("NAME", &export_name),
            ("PARAMETERS", &declared.parameters),
            ("PROPERTIES", &declared.properties),
            ("READS", &reads),
            ("REFUSAL", &refusal_name(&export_name)),
            ("RETURNED", &returned),
            ("TYPING", &typing),
            ("VALID", &format!("_{export_name}_valid")),
            ("WALKS", &walks),
            ("WRITES", &writes),
        ],
    );
    register(&rust_ident, &export_name);
    Some(with_ident_aliases(
        format!("{}{text}", writing.imports()),
        &rust_ident,
        &export_name,
        true,
    ))
}

/// The draft's read of the struct `described`, and the keyword arguments it builds the draft with,
/// one line per member. The read walks the pairs in the order the text writes them, as serde's
/// derive does, so of two faults the one the text writes first is refused: a key no member is read
/// under is refused at the key where the struct is strict, and its value skipped as serde skips it
/// otherwise; a member's key written twice is refused before its second value is read; and each
/// member's value is read at its key. A member left out is refused once every pair is read, the
/// first in the order declared, at its key, and an `Option` left out reads `None`.
fn member_reads(
    members: &[Member],
    described: &str,
    writing: &mut Writing,
) -> Option<(String, String)> {
    let class = writing.ask(Helper::At);
    writing.ask(Helper::Pairs);
    let mut held = String::from("        seen: set[str] = set()\n");
    let mut branches = String::new();
    let mut missing = String::new();
    let mut returned = String::new();
    for (nth, member) in members.iter().enumerate() {
        let wire = quoted(&member.wire);
        let local = format!("member_{}", member.name);
        let optional = member.field.is_optional();
        let read_member = if optional {
            let inner = read(&present(&member.field), "item", writing)?;
            let _ = writeln!(held, "        {local}: {} = None", member.hint);
            format!("None if item is None else {inner}")
        } else {
            let _ = writeln!(held, "        {local}: {} | None = None", member.hint);
            let _ = write!(
                missing,
                "        if {local} is None:\n            raise ValueError({}, [{wire}])\n",
                quoted(&format!("missing field `{}`", member.wire))
            );
            read(&member.field, "item", writing)?
        };
        let keyword = if nth == 0 { "if" } else { "elif" };
        let _ = write!(
            branches,
            "            {keyword} key == {wire}:\n                {local} = \
             {class}._tix_at({wire}, lambda: {read_member})\n"
        );
        let _ = writeln!(returned, "            {}={local},", member.name);
    }
    let wires: Vec<String> = members.iter().map(|member| member.wire.clone()).collect();
    let unknown = if is_nonstrict() {
        writing.ask(Helper::Ignored);
        format!(
            "                {class}._tix_at(key, lambda: {class}._tix_ignored(item))\n                \
             continue\n"
        )
    } else {
        format!(
            "                raise ValueError(\"unknown field `\" + key + {}, [key])\n",
            quoted(&format!("`, expected {}", one_of(&wires)))
        )
    };
    let _ = write!(
        held,
        "        for key, item in {class}._tix_pairs(value, {}):\n            if key in seen:\n                \
         raise ValueError(\"duplicate field `\" + key + \"`\", [])\n{branches}            else:\n\
         {unknown}            seen.add(key)\n{missing}",
        quoted(&format!("struct {described}"))
    );
    Some((held, returned))
}

/// `field` past the `None` of its outermost level, which its holder reads and writes for itself.
fn present(field: &FieldDef) -> FieldDef {
    let mut held = field.clone();
    held.nullable_levels
        .retain(|level| *level != field.array_depth);
    held
}

/// The statements of the draft's write, one or two lines per member.
fn member_writes(members: &[Member], writing: &mut Writing) -> Option<String> {
    let mut lines = String::new();
    for member in members {
        let wire = quoted(&member.wire);
        let held = format!("self.{}", member.name);
        let maps = member.sorted_maps.as_slice();
        if !member.field.is_optional() {
            let written = write(&member.field, &held, maps, writing)?;
            let _ = writeln!(lines, "        written[{wire}] = {written}");
        } else if member.nullable {
            let written = write(&present(&member.field), &held, maps, writing)?;
            let _ = writeln!(
                lines,
                "        written[{wire}] = None if {held} is None else {written}"
            );
        } else {
            let written = write(&present(&member.field), &held, maps, writing)?;
            let _ = writeln!(lines, "        if {held} is not None:");
            let _ = writeln!(lines, "            written[{wire}] = {written}");
        }
    }
    Some(lines)
}

/// The body of the draft's `validation_issues()`, which runs each held model's.
fn member_walks(members: &[Member], writing: &mut Writing) -> String {
    let mut lines = Vec::new();
    for member in members.iter().filter(|member| holds_a_model(&member.field)) {
        let held = format!("self.{}", member.name);
        walk_at(
            &member.field,
            member.field.array_depth,
            (&held, &quoted(&member.wire)),
            2,
            writing,
            &mut lines,
        );
    }
    if lines.is_empty() {
        return "        return []\n".to_owned();
    }
    format!(
        "        issues: list[str] = []\n{}\n        return issues\n",
        lines.join("\n")
    )
}

/// `template` with each `@MARKER@` that `filling` names replaced by its text, in one pass: a text
/// put in place is never read for a marker again. An `@` that opens no marker of `filling` is kept.
fn filled(template: &str, filling: &[(&str, &str)]) -> String {
    let mut written = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(at) = rest.find('@') {
        written.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        let marked = after.find('@').and_then(|end| {
            let name = &after[..end];
            filling
                .iter()
                .find(|(marker, _)| *marker == name)
                .map(|(_, text)| (end, *text))
        });
        if let Some((end, text)) = marked {
            written.push_str(text);
            rest = &after[end + 1..];
        } else {
            written.push('@');
            rest = after;
        }
    }
    written.push_str(rest);
    written
}

/// `source`, then the names a renamed item re-publishes under its own Rust ident: the alias a
/// reference written above the rename still resolves through.
fn with_ident_aliases(
    source: String,
    rust_ident: &str,
    export_name: &str,
    drafted: bool,
) -> String {
    if rust_ident == export_name {
        return source;
    }
    let mut written = source;
    let _ = write!(written, "\n\n\n{rust_ident} = {export_name}");
    if drafted {
        let _ = write!(written, "\n{rust_ident}Draft = {export_name}Draft");
    }
    written
}

/// The variants of `item_enum`, each as the member its `enum.Enum` declares and the name serde
/// writes it under, or `None` where one is written in a form this module does not write yet.
fn enum_variants(item_enum: &ItemEnum) -> Option<Vec<(String, String)>> {
    let rule = resolve_rename_rule(
        parse_serde_type_attributes(&item_enum.attrs)
            .rename_all
            .as_deref(),
    );
    let mut variants: Vec<(String, String)> = Vec::new();
    for variant in &item_enum.variants {
        let rust_name = variant.ident.unraw().to_string();
        if !only_serde_keys(&variant.attrs, &["rename"])
            || rust_name.starts_with('_')
            || ENUM_TAKEN.contains(&rust_name.as_str())
        {
            return None;
        }
        let wire = parse_serde_field_attributes(&variant.attrs)
            .rename
            .unwrap_or_else(|| rule.apply_to_variant(&rust_name));
        let taken: Vec<String> = variants.iter().map(|(member, _)| member.clone()).collect();
        variants.push((python_member(&rust_name, &taken), wire));
    }
    let distinct: HashSet<&String> = variants.iter().map(|(_, wire)| wire).collect();
    (!variants.is_empty() && distinct.len() == variants.len()).then_some(variants)
}

/// The Python text of `item_enum`, an `enum.Enum` of its variants, or `None` for an enum whose
/// shape this module does not write yet: every enum but one serde writes as a variant's name alone.
fn enum_source(item_enum: &ItemEnum, args: &PythonArgs<'_>) -> Option<String> {
    let writes_its_shape = valid_type::is_undrafted()
        && is_plain_enum(item_enum)
        && args.check.is_empty()
        && args.preprocess.is_empty()
        && item_enum.generics.params.is_empty()
        && only_serde_keys(&item_enum.attrs, &ENUM_SERDE_KEYS);
    if !writes_its_shape {
        return None;
    }
    let variants = enum_variants(item_enum)?;
    let wires: Vec<String> = variants.iter().map(|(_, wire)| wire.clone()).collect();
    let declared = variants
        .iter()
        .fold(String::new(), |mut written, (member, wire)| {
            let _ = writeln!(written, "    {member} = {}", quoted(wire));
            written
        });
    let rust_ident = item_enum.ident.unraw().to_string();
    let export_name = compute_item_export_name(&rust_ident, args.name_override);
    let mut writing = Writing::new(&export_name, &export_name);
    writing.ask(Helper::Parsed);
    writing.ask(Helper::Invalid);
    let enum_module = writing.module("enum");
    let json = writing.module("json");
    let typing = writing.module("typing");
    let text = filled(
        ENUM_TEXT,
        &[
            ("ENUM", &enum_module),
            (
                "EXPECTED",
                &quoted(&format!("`, expected {}", one_of(&wires))),
            ),
            ("HELPERS", writing.helper_methods().trim_end()),
            ("JSON", &json),
            ("NAME", &export_name),
            ("TYPING", &typing),
            ("VARIANTS", &declared),
        ],
    );
    register(&rust_ident, &export_name);
    Some(with_ident_aliases(
        format!("{}{text}", writing.imports()),
        &rust_ident,
        &export_name,
        false,
    ))
}
