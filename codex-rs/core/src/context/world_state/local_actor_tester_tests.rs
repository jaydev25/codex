use super::*;

#[test]
fn legacy_planner_fragment_is_recognized_for_replacement() {
    assert!(LocalActorTesterState::matches_legacy_fragment(
        "developer",
        "<local_actor_planner>old contract</local_actor_planner>",
    ));
}

#[test]
fn unknown_prior_state_emits_current_tester_contract() {
    let state = LocalActorTesterState::new(true);

    assert!(state.render_diff(PreviousSectionState::Unknown).is_some());
    assert!(state.render_diff(PreviousSectionState::Absent).is_some());
    assert!(
        state
            .render_diff(PreviousSectionState::Known(&true))
            .is_none()
    );
}
