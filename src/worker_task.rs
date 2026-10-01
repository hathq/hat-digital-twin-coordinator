use hat_digital_twin_coordinator::{
    LINK_OPERATION_ID, LinkCheck, REPOSITORY_ID, REVIEW_INPUT_SCHEMA, REVIEW_OPERATION_ID,
    REVIEW_OUTPUT_SCHEMA, ReviewCompositionInput, information_link_check, review_composition,
};
use hat_specifications::{
    ACTION_RESULT_SCHEMA, ActionReference, HatActionResult, HatInvocationOutcome,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::worker::Lease;
use crate::worker_io::{canonical_directory, message, path, required, run_hatter, write_document};

const LINK_INPUT_SCHEMA: &str =
    "hathq://hat-digital-twin-coordinator/information-link-check-input/v1";
const LINK_OUTPUT_SCHEMA: &str = "hathq://hat/situation-contribution/v1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LinkInput {
    schema: String,
    observed_at_unix_ms: u64,
    time_zone: String,
    revision: u64,
}

pub(super) fn process(
    args: &BTreeMap<String, String>,
    common: &[String],
    control: &Path,
    lease: &Lease,
) -> Result<(), String> {
    let invocation = &lease.record.invocation;
    if invocation.input.owner_id != "zixcel-graph" {
        return Err("invocation is outside the coordinator worker contract".into());
    }
    let input_store = canonical_directory(required(args, "input-store")?)?;
    let bytes = read_digest_document(&input_store, &invocation.input)?;
    let (output, schema, revision) = match invocation.operation_id.as_str() {
        REVIEW_OPERATION_ID => {
            let input: ReviewCompositionInput = serde_json::from_slice(&bytes).map_err(message)?;
            if input.revision != invocation.expected_projection_revision {
                return Err("composition revision differs".into());
            }
            let output = review_composition(input).map_err(str::to_owned)?;
            let revision = output.revision;
            (
                serde_json::to_vec(&output).map_err(message)?,
                REVIEW_OUTPUT_SCHEMA,
                revision,
            )
        }
        LINK_OPERATION_ID => {
            let input: LinkInput = serde_json::from_slice(&bytes).map_err(message)?;
            if input.schema != LINK_INPUT_SCHEMA
                || input.revision != invocation.expected_projection_revision.saturating_add(1)
            {
                return Err("information-link revision differs".into());
            }
            let revision = input.revision;
            let output = information_link_check(LinkCheck {
                observed_at_unix_ms: input.observed_at_unix_ms,
                time_zone: input.time_zone,
                revision,
            })
            .map_err(str::to_owned)?;
            (
                serde_json::to_vec(&output).map_err(message)?,
                LINK_OUTPUT_SCHEMA,
                revision,
            )
        }
        _ => return Err("coordinator operation is not supported".into()),
    };
    let digest = hex::encode(Sha256::digest(&output));
    let output_store = canonical_directory(required(args, "output-store")?)?;
    write_document(&output_store.join(format!("{digest}.json")), &output)?;
    complete(args, common, control, lease, schema, revision, &digest)?;
    println!("{{\"processed\":true,\"outputDigestSha256\":\"{digest}\"}}");
    Ok(())
}

fn complete(
    args: &BTreeMap<String, String>,
    common: &[String],
    control: &Path,
    lease: &Lease,
    schema: &str,
    revision: u64,
    digest: &str,
) -> Result<(), String> {
    let invocation = &lease.record.invocation;
    let result = HatActionResult {
        schema: ACTION_RESULT_SCHEMA.into(),
        invocation_id: invocation.invocation_id.clone(),
        operation_id: invocation.operation_id.clone(),
        state_revision: lease.record.status.state_revision.saturating_add(1),
        projection_revision: revision,
        outcome: HatInvocationOutcome::Completed,
        output: Some(ActionReference {
            owner_id: REPOSITORY_ID.into(),
            reference: digest.into(),
            schema_id: schema.into(),
            digest_sha256: digest.into(),
        }),
        reason_id: None,
        evidence_refs: Vec::new(),
    };
    let result_path = control.join(format!("result-{}.json", result.invocation_id));
    write_document(&result_path, &serde_json::to_vec(&result).map_err(message)?)?;
    run_hatter(
        args,
        "complete",
        common,
        &[
            "--worker-id",
            required(args, "worker-id")?,
            "--result-json",
            path(&result_path)?,
        ],
    )?;
    Ok(())
}

fn read_digest_document(root: &Path, reference: &ActionReference) -> Result<Vec<u8>, String> {
    let expected = match reference.schema_id.as_str() {
        REVIEW_INPUT_SCHEMA => REVIEW_INPUT_SCHEMA,
        LINK_INPUT_SCHEMA => LINK_INPUT_SCHEMA,
        _ => return Err("coordinator input schema differs".into()),
    };
    if reference.schema_id != expected || reference.reference != reference.digest_sha256 {
        return Err("input reference is not content-addressed".into());
    }
    let bytes = fs::read(root.join(format!("{}.json", reference.reference))).map_err(message)?;
    if bytes.len() > 1_048_576 || hex::encode(Sha256::digest(&bytes)) != reference.digest_sha256 {
        return Err("input digest differs".into());
    }
    Ok(bytes)
}
