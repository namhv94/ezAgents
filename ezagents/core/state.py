"""Durable State Machine and Checkpointing Engine.
Enforces strict lifecycle transitions and rollback capabilities for agent steps.
"""

from __future__ import annotations

import copy
import hashlib
import json
import time
from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Dict, List, Optional


class RunState(str, Enum):
    DRAFT = "draft"
    PLANNING = "planning"
    AWAITING_APPROVAL = "awaiting_approval"
    RUNNING = "running"
    VERIFYING = "verifying"
    COMPLETED = "completed"
    FAILED = "failed"
    CANCELLED = "cancelled"
    ROLLED_BACK = "rolled_back"


VALID_TRANSITIONS = {
    RunState.DRAFT: {RunState.PLANNING, RunState.CANCELLED},
    RunState.PLANNING: {RunState.AWAITING_APPROVAL, RunState.RUNNING, RunState.FAILED, RunState.CANCELLED},
    RunState.AWAITING_APPROVAL: {RunState.RUNNING, RunState.CANCELLED, RunState.FAILED},
    RunState.RUNNING: {RunState.RUNNING, RunState.VERIFYING, RunState.FAILED, RunState.CANCELLED, RunState.ROLLED_BACK},
    RunState.VERIFYING: {RunState.COMPLETED, RunState.RUNNING, RunState.FAILED, RunState.ROLLED_BACK},
    RunState.ROLLED_BACK: {RunState.RUNNING, RunState.FAILED, RunState.CANCELLED},
    RunState.COMPLETED: set(),
    RunState.FAILED: set(),
    RunState.CANCELLED: set(),
}


@dataclass
class StateCheckpoint:
    step_id: int
    state: RunState
    timestamp: float
    data: Dict[str, Any]
    artifacts: Dict[str, str] = field(default_factory=dict)
    state_hash: str = ""

    @classmethod
    def create(cls, step_id: int, state: RunState, data: Dict[str, Any], artifacts: Optional[Dict[str, str]] = None) -> StateCheckpoint:
        ts = time.time()
        art = artifacts or {}
        raw = f"{step_id}:{state.value}:{ts}:{json.dumps(data, sort_keys=True)}"
        shash = hashlib.sha256(raw.encode("utf-8")).hexdigest()[:16]
        return cls(
            step_id=step_id,
            state=state,
            timestamp=ts,
            data=copy.deepcopy(data),
            artifacts=copy.deepcopy(art),
            state_hash=shash,
        )


class StateMachine:
    """Deterministic finite state machine with checkpoint rollback."""

    def __init__(self, run_id: str, initial_data: Optional[Dict[str, Any]] = None):
        self.run_id = run_id
        self.current_state: RunState = RunState.DRAFT
        self.data: Dict[str, Any] = initial_data or {}
        self.artifacts: Dict[str, str] = {}
        self.step_counter: int = 0
        self.history: List[StateCheckpoint] = []
        self._record_checkpoint()

    def _record_checkpoint(self) -> StateCheckpoint:
        cp = StateCheckpoint.create(
            step_id=self.step_counter,
            state=self.current_state,
            data=self.data,
            artifacts=self.artifacts,
        )
        self.history.append(cp)
        return cp

    def transition_to(self, target_state: RunState, updates: Optional[Dict[str, Any]] = None) -> StateCheckpoint:
        allowed = VALID_TRANSITIONS.get(self.current_state, set())
        if target_state not in allowed:
            raise ValueError(
                f"Illegal state transition from '{self.current_state.value}' to '{target_state.value}'. "
                f"Allowed target states: {[s.value for s in allowed]}"
            )
        self.current_state = target_state
        if updates:
            self.data.update(updates)
        self.step_counter += 1
        return self._record_checkpoint()

    def rollback_to_step(self, step_id: int) -> StateCheckpoint:
        """Roll back system data and artifacts to an earlier verified checkpoint."""
        target_cp = None
        for cp in self.history:
            if cp.step_id == step_id:
                target_cp = cp
                break

        if not target_cp:
            raise ValueError(f"Checkpoint for step_id {step_id} not found in run history")

        self.current_state = RunState.ROLLED_BACK
        self.data = copy.deepcopy(target_cp.data)
        self.artifacts = copy.deepcopy(target_cp.artifacts)
        self.step_counter += 1
        return self._record_checkpoint()

    def get_latest_checkpoint(self) -> StateCheckpoint:
        return self.history[-1]
