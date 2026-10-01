use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const REVIEW_OPERATION_ID: &str = "hathq://vocabulary/action/review-hat-composition/v1";
pub const REVIEW_INPUT_SCHEMA: &str =
    "hathq://hat-digital-twin-coordinator/review-composition-input/v1";
pub const REVIEW_OUTPUT_SCHEMA: &str =
    "hathq://hat-digital-twin-coordinator/review-composition-output/v1";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompositionMember {
    pub repository_id: String,
    pub binding_revision: u64,
    pub active: bool,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewCompositionInput {
    pub schema: String,
    pub scope_ref: String,
    pub revision: u64,
    pub members: Vec<CompositionMember>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnresolvedDependency {
    pub repository_id: String,
    pub dependency_repository_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReviewCompositionOutput {
    pub schema: &'static str,
    pub scope_ref: String,
    pub revision: u64,
    pub members: Vec<CompositionMember>,
    pub ordered_active_repository_ids: Vec<String>,
    pub unresolved_dependencies: Vec<UnresolvedDependency>,
    pub cycle_detected: bool,
}

/// Produces an exact, deterministic composition review without installing,
/// enabling, disabling, or guessing a HAT.
///
/// # Errors
///
/// Rejects malformed identifiers, duplicate members/dependencies, empty input,
/// and revision overflow.
pub fn review_composition(
    mut input: ReviewCompositionInput,
) -> Result<ReviewCompositionOutput, &'static str> {
    if input.schema != REVIEW_INPUT_SCHEMA
        || !reference(&input.scope_ref, 96)
        || input.members.is_empty()
        || input.members.len() > 256
    {
        return Err("composition-review-invalid");
    }
    input
        .members
        .sort_by(|left, right| left.repository_id.cmp(&right.repository_id));
    validate_members(&input.members)?;
    let active: BTreeSet<_> = input
        .members
        .iter()
        .filter(|item| item.active)
        .map(|item| item.repository_id.clone())
        .collect();
    let unresolved = unresolved(&input.members, &active);
    let ordered = ordered(&input.members, &active);
    let cycle_detected = ordered.len() != active.len();
    Ok(ReviewCompositionOutput {
        schema: REVIEW_OUTPUT_SCHEMA,
        scope_ref: input.scope_ref,
        revision: input.revision.checked_add(1).ok_or("revision-overflow")?,
        members: input.members,
        ordered_active_repository_ids: if unresolved.is_empty() && !cycle_detected {
            ordered
        } else {
            Vec::new()
        },
        unresolved_dependencies: unresolved,
        cycle_detected,
    })
}

fn validate_members(members: &[CompositionMember]) -> Result<(), &'static str> {
    let mut repositories = BTreeSet::new();
    for member in members {
        if !reference(&member.repository_id, 96)
            || member.binding_revision == 0
            || member.dependencies.len() > 64
            || !repositories.insert(member.repository_id.as_str())
        {
            return Err("composition-review-invalid");
        }
        let mut dependencies = BTreeSet::new();
        for dependency in &member.dependencies {
            if !reference(dependency, 96)
                || dependency == &member.repository_id
                || !dependencies.insert(dependency)
            {
                return Err("composition-review-invalid");
            }
        }
    }
    Ok(())
}

fn unresolved(
    members: &[CompositionMember],
    active: &BTreeSet<String>,
) -> Vec<UnresolvedDependency> {
    let mut values = Vec::new();
    for member in members.iter().filter(|item| item.active) {
        for dependency in &member.dependencies {
            if !active.contains(dependency) {
                values.push(UnresolvedDependency {
                    repository_id: member.repository_id.clone(),
                    dependency_repository_id: dependency.clone(),
                });
            }
        }
    }
    values
}

fn ordered(members: &[CompositionMember], active: &BTreeSet<String>) -> Vec<String> {
    let mut dependencies: BTreeMap<String, BTreeSet<String>> = members
        .iter()
        .filter(|item| item.active)
        .map(|item| {
            (
                item.repository_id.clone(),
                item.dependencies
                    .iter()
                    .filter(|value| active.contains(*value))
                    .cloned()
                    .collect(),
            )
        })
        .collect();
    let mut result = Vec::new();
    while let Some(next) = dependencies
        .iter()
        .find_map(|(key, values)| values.is_empty().then(|| key.clone()))
    {
        dependencies.remove(&next);
        for values in dependencies.values_mut() {
            values.remove(&next);
        }
        result.push(next);
    }
    result
}

fn reference(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value.trim() == value
        && !value.bytes().any(|byte| byte.is_ascii_control())
}
