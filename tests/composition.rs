use hat_digital_twin_coordinator::{
    CompositionMember, REVIEW_INPUT_SCHEMA, ReviewCompositionInput, review_composition,
};

fn member(id: &str, dependencies: &[&str]) -> CompositionMember {
    CompositionMember {
        repository_id: id.into(),
        binding_revision: 1,
        active: true,
        dependencies: dependencies.iter().map(|value| (*value).into()).collect(),
    }
}

#[test]
fn dependencies_are_ordered_without_inferring_missing_members() {
    let output = review_composition(ReviewCompositionInput {
        schema: REVIEW_INPUT_SCHEMA.into(),
        scope_ref: "scope/person".into(),
        revision: 4,
        members: vec![
            member("hat-budget", &["hat-accountant"]),
            member("hat-accountant", &[]),
        ],
    })
    .expect("review");
    assert_eq!(output.revision, 5);
    assert_eq!(
        output.ordered_active_repository_ids,
        ["hat-accountant", "hat-budget"]
    );
    assert!(output.unresolved_dependencies.is_empty());
    assert!(!output.cycle_detected);

    let missing = review_composition(ReviewCompositionInput {
        schema: REVIEW_INPUT_SCHEMA.into(),
        scope_ref: "scope/person".into(),
        revision: 0,
        members: vec![member("hat-budget", &["hat-accountant"])],
    })
    .expect("unresolved review");
    assert!(missing.ordered_active_repository_ids.is_empty());
    assert_eq!(missing.unresolved_dependencies.len(), 1);
}

#[test]
fn cycles_and_duplicate_identity_fail_closed() {
    let cycle = review_composition(ReviewCompositionInput {
        schema: REVIEW_INPUT_SCHEMA.into(),
        scope_ref: "scope/person".into(),
        revision: 0,
        members: vec![member("a", &["b"]), member("b", &["a"])],
    })
    .expect("cycle report");
    assert!(cycle.cycle_detected);
    assert!(cycle.ordered_active_repository_ids.is_empty());

    let duplicate = review_composition(ReviewCompositionInput {
        schema: REVIEW_INPUT_SCHEMA.into(),
        scope_ref: "scope/person".into(),
        revision: 0,
        members: vec![member("a", &[]), member("a", &[])],
    });
    assert_eq!(duplicate, Err("composition-review-invalid"));
}
