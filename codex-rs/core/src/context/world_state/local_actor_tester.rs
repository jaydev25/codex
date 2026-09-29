use super::PreviousSectionState;
use super::WorldStateSection;
use crate::context::ContextualUserFragment;
use crate::context::LocalActorTester;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct LocalActorTesterState {
    enabled: bool,
}

impl LocalActorTesterState {
    pub(crate) fn new(enabled: bool) -> Self {
        Self { enabled }
    }
}

impl WorldStateSection for LocalActorTesterState {
    const ID: &'static str = "local_actor_tester";
    type Snapshot = bool;

    fn snapshot(&self) -> Self::Snapshot {
        self.enabled
    }

    fn matches_legacy_fragment(role: &str, text: &str) -> bool {
        role == "developer" && LocalActorTester::matches_text(text)
    }

    fn has_retained_fragment_matcher() -> bool {
        true
    }

    fn matches_retained_fragment(role: &str, text: &str) -> bool {
        Self::matches_legacy_fragment(role, text)
    }

    fn render_diff(
        &self,
        previous: PreviousSectionState<'_, Self::Snapshot>,
    ) -> Option<Box<dyn ContextualUserFragment>> {
        if !self.enabled || matches!(previous, PreviousSectionState::Known(previous) if *previous) {
            return None;
        }
        Some(Box::new(LocalActorTester))
    }
}

#[cfg(test)]
#[path = "local_actor_tester_tests.rs"]
mod tests;
