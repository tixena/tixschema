//! Which `#[serde(untagged)]` enums Zod joins with `z.discriminatedUnion` in place of `z.union`:
//! the ones whose variants are told apart by a `literal` on one shared key, and the text of that
//! union.
//!
//! A discriminated union reads the one option its key names before it parses, so a rule that
//! fails inside a losing option never reaches the union's report, and an intersection around it
//! clears the keys of its other sources. The five conditions are in [`plan`].

use core::cell::RefCell;
use std::collections::HashMap;

use super::ts_member_key;
use crate::field_type::{FieldDef, FieldDefType, format_number_literal};
use crate::utils::ts_quoted;

/// The literal a member is held to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Literal {
    /// Whether the member is an `Option`, so that an absent key matches it.
    pub optional: bool,
    /// The literal in the text Zod writes it as, its kind in front.
    pub value: String,
}

/// A named member of a variant's object, under the key serde writes it by.
#[derive(Clone, Debug)]
pub struct Member {
    pub key: String,
    pub literal: Option<Literal>,
}

/// The key the enum is told apart by, and the variant that omits it.
#[derive(Debug, PartialEq, Eq)]
pub struct Plan {
    pub key: String,
    pub omitting: Option<usize>,
}

/// What a variant is written as in the union.
#[derive(Clone, Debug)]
pub enum Shape {
    /// One strict object of these members.
    Object(Vec<Member>),
    /// Anything else: a union option cannot be read by a key.
    Unreadable,
}

/// One variant: its shape, and the text it is an option of a discriminated union as.
pub struct Candidate {
    option: String,
    plain: String,
    shape: Shape,
}

/// The union an untagged enum's Zod schema is written as.
pub struct ZodUnion {
    pub code: String,
    pub discriminated: bool,
}

thread_local! {
    /// The members of each model `#[model_schema]` has been written on so far whose Zod binding is
    /// one strict object and nothing else: an empty entry for a model that is not.
    static MODELS: RefCell<HashMap<String, Option<Vec<Member>>>> = RefCell::new(HashMap::new());
}

/// Records what the model `rust_ident` publishes: its members where its `$RawSchema` is one strict
/// object, and nothing where it is anything else.
pub fn record_model(rust_ident: &str, members: Option<Vec<Member>>) {
    MODELS.with(|models| {
        models.borrow_mut().insert(rust_ident.to_owned(), members);
    });
}

/// The members of the model `rust_ident` where it was declared above and is one strict object.
fn model_members(rust_ident: &str) -> Option<Vec<Member>> {
    MODELS.with(|models| models.borrow().get(rust_ident).cloned().flatten())
}

/// The members of an object of `fields`.
pub fn members_of(fields: &[FieldDef]) -> Vec<Member> {
    fields
        .iter()
        .filter(|fld| !fld.absent_from_wire)
        .map(|fld| Member {
            key: fld.name.clone(),
            literal: literal_of(fld),
        })
        .collect()
}

/// The literal `fld` is held to, where it is held to one and nothing is written around it: a
/// function or a default would make the key something other than the literal.
fn literal_of(fld: &FieldDef) -> Option<Literal> {
    if fld.array_depth != 0 || fld.serde_default.is_some() {
        return None;
    }
    if fld.is_written_around() {
        return None;
    }
    Some(Literal {
        optional: fld.is_optional(),
        value: literal_text(&fld.field_type)?,
    })
}

/// The literal a type is, in the text Zod writes it as with its kind in front.
fn literal_text(field_type: &FieldDefType) -> Option<String> {
    if let FieldDefType::BooleanLiteral(value) = field_type {
        Some(format!("boolean {value}"))
    } else if let FieldDefType::StringLiteral(value) = field_type {
        Some(format!("string {}", ts_quoted(value)))
    } else if let FieldDefType::NumberLiteral(value) = field_type {
        Some(format!("number {}", format_number_literal(*value)))
    } else {
        None
    }
}

/// A variant written as an object of its named members.
pub fn named_candidate(zod: &str, fields: &[FieldDef], flattened: &[FieldDef]) -> Candidate {
    Candidate {
        option: zod.to_owned(),
        plain: zod.to_owned(),
        shape: if flattened.is_empty() {
            Shape::Object(members_of(fields))
        } else {
            Shape::Unreadable
        },
    }
}

/// A variant that holds one value, `slot`, written as `zod`: a model declared above that is one
/// strict object is read by its `$RawSchema`, which keeps the shape its key is read from where its
/// `$Schema` is annotated `ZodType`.
pub fn newtype_candidate(zod: &str, slot: &FieldDef, enum_name: &str) -> Candidate {
    let unreadable = unreadable_candidate(zod);
    let FieldDefType::SiblingType(name, arguments) = &slot.field_type else {
        return unreadable;
    };
    if !arguments.is_empty()
        || slot.array_depth != 0
        || slot.is_optional()
        || name == enum_name
        || slot.is_written_around()
    {
        return unreadable;
    }
    let (Some(members), Some(stem)) = (model_members(name), zod.strip_suffix("$Schema")) else {
        return unreadable;
    };
    Candidate {
        option: format!("{stem}$RawSchema"),
        plain: zod.to_owned(),
        shape: Shape::Object(members),
    }
}

/// A variant `zod` writes that no key can tell apart.
pub fn unreadable_candidate(zod: &str) -> Candidate {
    Candidate {
        option: zod.to_owned(),
        plain: zod.to_owned(),
        shape: Shape::Unreadable,
    }
}

/// The [`Plan`] of the variants `shapes`, in the order declared, where they meet all five conditions
/// on one key:
///
/// 1. Every variant is one strict object whose members are known.
/// 2. Every variant but at most one holds a literal on the key, and holds nothing else there.
/// 3. No two variants hold the same literal.
/// 4. At most one variant holds its literal in an `Option`.
/// 5. The one variant that omits the key is declared last, and no variant holds it in an `Option`:
///    serde reads past a key a variant does not declare, so only the last variant is reached by
///    the payloads the others refuse.
pub fn plan(shapes: &[&Shape]) -> Option<Plan> {
    let mut objects = Vec::new();
    for shape in shapes {
        let Shape::Object(members) = shape else {
            return None;
        };
        objects.push(members);
    }
    let mut keys: Vec<&str> = Vec::new();
    for member in objects.iter().flat_map(|members| members.iter()) {
        if member.literal.is_some() && !keys.contains(&member.key.as_str()) {
            keys.push(&member.key);
        }
    }
    keys.into_iter().find_map(|key| plan_for(key, &objects))
}

/// The [`Plan`] on `key`, where the variants meet the five conditions on it.
fn plan_for(key: &str, objects: &[&Vec<Member>]) -> Option<Plan> {
    let mut literals: Vec<&Literal> = Vec::new();
    let mut omitting = Vec::new();
    for (at, members) in objects.iter().enumerate() {
        match members.iter().find(|member| member.key == key) {
            Some(member) => literals.push(member.literal.as_ref()?),
            None => omitting.push(at),
        }
    }
    let optionals = literals.iter().filter(|literal| literal.optional).count();
    let mut values: Vec<&str> = literals
        .iter()
        .map(|literal| literal.value.as_str())
        .collect();
    values.sort_unstable();
    values.dedup();
    let omits_last = match omitting.as_slice() {
        [] => None,
        [last] if *last + 1 == objects.len() && optionals == 0 => Some(*last),
        _ => return None,
    };
    (optionals <= 1 && values.len() == literals.len()).then(|| Plan {
        key: key.to_owned(),
        omitting: omits_last,
    })
}

/// The Zod union of the variants `candidates`.
pub fn union_of(candidates: &[Candidate]) -> ZodUnion {
    let shapes: Vec<&Shape> = candidates
        .iter()
        .map(|candidate| &candidate.shape)
        .collect();
    let Some(plan) = plan(&shapes) else {
        return ZodUnion {
            code: format!(
                "z.union([{}])",
                candidates
                    .iter()
                    .map(|candidate| candidate.plain.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            discriminated: false,
        };
    };
    ZodUnion {
        code: format!(
            "z.discriminatedUnion({}, [{}])",
            ts_quoted(&plan.key),
            options(candidates, &plan)
        ),
        discriminated: true,
    }
}

/// The options of a union, joined: each variant as it is written, and the one a [`Plan`] says omits
/// the key taking it as an `undefined` that may be absent.
fn options(candidates: &[Candidate], joined_by: &Plan) -> String {
    candidates
        .iter()
        .enumerate()
        .map(|(at, candidate)| {
            if joined_by.omitting == Some(at) {
                with_absent_key(&candidate.option, &joined_by.key)
            } else {
                candidate.option.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// `option`, an object, with `key` among its members read as an `undefined` that may be absent.
fn with_absent_key(option: &str, key: &str) -> String {
    let absent = format!("{}: z.undefined().optional()", ts_member_key(key));
    option.strip_prefix("z.strictObject({ ").map_or_else(
        || format!("{option}.safeExtend({{ {absent} }})"),
        |rest| format!("z.strictObject({{ {absent}, {rest}"),
    )
}

#[cfg(test)]
mod tests;
