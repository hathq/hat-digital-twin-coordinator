use hat_digital_twin_coordinator::{
    LINK_OPERATION_ID, LinkCheck, PACKAGE_ID, PACKAGE_JSON, REPOSITORY_ID, information_link_check,
};

#[test]
fn package_is_a_separate_bounded_hat() {
    let package: serde_json::Value = serde_json::from_str(PACKAGE_JSON).expect("package JSON");
    assert_eq!(package["package_id"], PACKAGE_ID);
    assert_eq!(package["repository_id"], REPOSITORY_ID);
    assert!(
        package["operations"]
            .as_array()
            .expect("operations")
            .iter()
            .any(|value| value["id"] == LINK_OPERATION_ID)
    );
    assert_eq!(package["operations"][1]["handler"]["kind"], "hat-service");
}

#[test]
fn tutorial_projects_explicit_values_and_never_invents_a_place() {
    let value = information_link_check(LinkCheck {
        observed_at_unix_ms: 1_700_000_000_000,
        time_zone: "Asia/Tokyo".into(),
        revision: 1,
    })
    .expect("contribution");
    assert_eq!(value.source.repository_id, REPOSITORY_ID);
    assert_eq!(value.time.time_zone, "Asia/Tokyo");
    assert_eq!(value.place, None);
    assert_eq!(
        information_link_check(LinkCheck {
            observed_at_unix_ms: 0,
            time_zone: "Asia/Tokyo".into(),
            revision: 1
        }),
        Err("information-link-check-invalid")
    );
}
