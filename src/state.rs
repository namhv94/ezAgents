use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    Draft,
    Planning,
    AwaitingApproval,
    Running,
    Verifying,
    Completed,
    Failed,
    Cancelled,
    RolledBack,
}

#[derive(Error, Debug)]
pub enum StateError {
    #[error("Illegal state transition from {from:?} to {to:?}")]
    IllegalTransition { from: RunState, to: RunState },
    #[error("Checkpoint not found for step {0}")]
    CheckpointNotFound(usize),
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct StateCheckpoint {
    pub step_id: usize,
    pub state: RunState,
    pub timestamp_ms: u64,
    pub data: Value,
    pub state_hash: String,
}

impl StateCheckpoint {
    pub fn create(step_id: usize, state: RunState, data: &Value) -> Self {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let raw = format!(
            "{}:{}:{}:{}",
            step_id,
            serde_json::to_string(&state).unwrap_or_default(),
            ts,
            data
        );
        let mut hasher = Sha256::new();
        hasher.update(raw.as_bytes());
        let hash_str = hex::encode(hasher.finalize());
        let state_hash = hash_str[..16].to_string();

        Self {
            step_id,
            state,
            timestamp_ms: ts,
            data: data.clone(),
            state_hash,
        }
    }
}

pub struct StateMachine {
    pub run_id: String,
    pub current_state: RunState,
    pub data: Value,
    pub step_counter: usize,
    pub history: Vec<StateCheckpoint>,
}

impl StateMachine {
    pub fn new(run_id: &str, initial_data: Value) -> Self {
        let initial_checkpoint = StateCheckpoint::create(0, RunState::Draft, &initial_data);
        Self {
            run_id: run_id.to_string(),
            current_state: RunState::Draft,
            data: initial_data,
            step_counter: 0,
            history: vec![initial_checkpoint],
        }
    }

    pub fn is_allowed_transition(&self, target: RunState) -> bool {
        match self.current_state {
            RunState::Draft => matches!(target, RunState::Planning | RunState::Cancelled),
            RunState::Planning => matches!(
                target,
                RunState::AwaitingApproval
                    | RunState::Running
                    | RunState::Failed
                    | RunState::Cancelled
            ),
            RunState::AwaitingApproval => {
                matches!(
                    target,
                    RunState::Running | RunState::Cancelled | RunState::Failed
                )
            }
            RunState::Running => matches!(
                target,
                RunState::Running
                    | RunState::Verifying
                    | RunState::Failed
                    | RunState::Cancelled
                    | RunState::RolledBack
            ),
            RunState::Verifying => matches!(
                target,
                RunState::Completed | RunState::Running | RunState::Failed | RunState::RolledBack
            ),
            RunState::RolledBack => {
                matches!(
                    target,
                    RunState::Running | RunState::Failed | RunState::Cancelled
                )
            }
            RunState::Completed | RunState::Failed | RunState::Cancelled => false,
        }
    }

    pub fn transition_to(
        &mut self,
        target_state: RunState,
        updates: Option<Value>,
    ) -> Result<&StateCheckpoint, StateError> {
        if !self.is_allowed_transition(target_state) {
            return Err(StateError::IllegalTransition {
                from: self.current_state,
                to: target_state,
            });
        }
        self.current_state = target_state;
        if let Some(mut upd) = updates {
            if let Some(obj) = upd.as_object_mut() {
                if let Some(current_obj) = self.data.as_object_mut() {
                    for (k, v) in std::mem::take(obj) {
                        current_obj.insert(k, v);
                    }
                }
            }
        }
        self.step_counter += 1;
        let cp = StateCheckpoint::create(self.step_counter, self.current_state, &self.data);
        self.history.push(cp);
        Ok(self.history.last().unwrap())
    }

    pub fn rollback_to_step(&mut self, step_id: usize) -> Result<&StateCheckpoint, StateError> {
        let target_cp = self
            .history
            .iter()
            .find(|cp| cp.step_id == step_id)
            .cloned()
            .ok_or(StateError::CheckpointNotFound(step_id))?;

        self.current_state = RunState::RolledBack;
        self.data = target_cp.data;
        self.step_counter += 1;
        let cp = StateCheckpoint::create(self.step_counter, self.current_state, &self.data);
        self.history.push(cp);
        Ok(self.history.last().unwrap())
    }
}
