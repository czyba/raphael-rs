use raphael_sim::*;

use rustc_hash::FxHashMap as HashMap;

use crate::{
    SolverSettings,
    actions::{FULL_SEARCH_ACTIONS, use_action_combo_with_condition},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ReducedState {
    durability: i16,
    cp: i16,
    effects: Effects,
}

impl ReducedState {
    fn from_state(state: &SimulationState) -> Self {
        Self {
            durability: state.durability,
            cp: state.cp,
            effects: state
                .effects
                .with_inner_quiet(0)
                .with_innovation(0)
                .with_great_strides(0)
                .with_guard(0)
                .with_quick_innovation_available(false),
        }
    }

    fn to_state(self) -> SimulationState {
        SimulationState {
            durability: self.durability,
            cp: self.cp,
            progress: 0,
            quality: 0,
            unreliable_quality: 0,
            effects: self.effects,
        }
    }
}

pub struct FinishSolver {
    settings: SolverSettings,
    // maximum attainable progress for each state
    max_progress: HashMap<ReducedState, u32>,
}

impl FinishSolver {
    pub fn new(settings: SolverSettings) -> Self {
        Self {
            settings,
            max_progress: HashMap::default(),
        }
    }

    pub fn can_finish(&mut self, state: &SimulationState, condition: Condition) -> bool {
        let max_progress = self.solve_max_progress(ReducedState::from_state(state), condition);
        state.progress + max_progress >= self.settings.max_progress()
    }

    fn solve_max_progress(&mut self, state: ReducedState, condition: Condition) -> u32 {
        match self.max_progress.get(&state) {
            Some(max_progress) => *max_progress,
            None => {
                let mut max_progress = 0;
                for action in FULL_SEARCH_ACTIONS {
                    if let Ok(new_state) = use_action_combo_with_condition(
                        &self.settings,
                        state.to_state(),
                        *action,
                        condition,
                    ) {
                        if new_state.is_final(&self.settings.simulator_settings) {
                            max_progress = std::cmp::max(max_progress, new_state.progress);
                        } else {
                            let next_condition = condition
                                .follow_up_condition_after_steps(action.steps())
                                .unwrap_or(Condition::Normal);
                            let child_progress = self.solve_max_progress(
                                ReducedState::from_state(&new_state),
                                next_condition,
                            );
                            max_progress =
                                std::cmp::max(max_progress, child_progress + new_state.progress);
                        }
                    }
                    if max_progress >= self.settings.max_progress() {
                        // stop early if progress is already maxed out
                        // this optimization would work better with a better action ordering
                        max_progress = self.settings.max_progress();
                        break;
                    }
                }
                self.max_progress.insert(state, max_progress);
                max_progress
            }
        }
    }
}

impl Drop for FinishSolver {
    fn drop(&mut self) {
        log::debug!("FinishSolver - states: {}", self.max_progress.len());
    }
}
