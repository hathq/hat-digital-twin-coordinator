#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

mod composition;

pub use composition::{
    CompositionMember, REVIEW_INPUT_SCHEMA, REVIEW_OPERATION_ID, REVIEW_OUTPUT_SCHEMA,
    ReviewCompositionInput, ReviewCompositionOutput, UnresolvedDependency, review_composition,
};

pub const PACKAGE_JSON: &str = include_str!("../hat.package.json");
pub const PACKAGE_ID: &str = "hat/digital-twin-coordinator";
pub const REPOSITORY_ID: &str = "hat-digital-twin-coordinator";
pub const LINK_OPERATION_ID: &str = "hathq://vocabulary/action/verify-information-link/v1";
pub const PACKAGE_SHA256: &str = "ddcd256eb588af599bc61358b1352abdd218bb4cc20c4d1458b202bf3d9fc7ab";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkCheck {
    pub observed_at_unix_ms: u64,
    pub time_zone: String,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SituationContribution {
    pub schema: &'static str,
    pub contribution_id: &'static str,
    pub category_term_id: &'static str,
    pub kind: &'static str,
    pub title: &'static str,
    pub status: &'static str,
    pub time: SituationTime,
    pub place: Option<SituationPlace>,
    pub actors: [&'static str; 1],
    pub object_label: &'static str,
    pub detail_handle: &'static str,
    pub source: SituationSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SituationTime {
    pub start_at_unix_ms: u64,
    pub end_at_unix_ms: Option<u64>,
    pub time_zone: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SituationPlace {
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SituationSource {
    pub repository_id: &'static str,
    pub digest_sha256: &'static str,
    pub revision: u64,
}

/// Produces one explicit diagnostic contribution without inferring place or person facts.
///
/// # Errors
///
/// Returns `information-link-check-invalid` when the observation has no time or
/// revision, or when its time-zone identifier is empty or exceeds the bound.
pub fn information_link_check(input: LinkCheck) -> Result<SituationContribution, &'static str> {
    if input.observed_at_unix_ms == 0 || input.revision == 0 {
        return Err("information-link-check-invalid");
    }
    if input.time_zone.is_empty() || input.time_zone.len() > 80 {
        return Err("information-link-check-invalid");
    }
    Ok(SituationContribution {
        schema: "hathq://hat/situation-contribution/v1",
        contribution_id: "built-in-information-link-check",
        category_term_id: "hathq://vocabulary/data-domain/ecosystem-operations/v1",
        kind: "observation",
        title: "HATから「いま」への情報連携を確認",
        status: "active",
        time: SituationTime {
            start_at_unix_ms: input.observed_at_unix_ms,
            end_at_unix_ms: None,
            time_zone: input.time_zone,
        },
        place: None,
        actors: ["わたし"],
        object_label: "共通Situation projection",
        detail_handle: "built-in-information-link-check",
        source: SituationSource {
            repository_id: REPOSITORY_ID,
            digest_sha256: PACKAGE_SHA256,
            revision: input.revision,
        },
    })
}
