"""Scoped Sandbox and Capability Boundaries.
Enforces workspace filesystem scoping, resource limits, and execution barriers.
"""

from __future__ import annotations

import os
from dataclasses import dataclass
from enum import Enum
from pathlib import Path
from typing import Optional


class CapabilityLevel(str, Enum):
    READ_ONLY = "read_only"
    WORKSPACE_WRITE = "workspace_write"
    FULL_UNRESTRICTED = "full_unrestricted"


@dataclass
class CapabilityPolicy:
    level: CapabilityLevel = CapabilityLevel.WORKSPACE_WRITE
    allow_subprocesses: bool = True
    max_steps: int = 30
    timeout_seconds: float = 60.0
    max_output_bytes: int = 1_000_000


class ScopedSandbox:
    """Isolates agent file operations within an explicitly allowed directory."""

    def __init__(self, workspace_path: str | Path, policy: Optional[CapabilityPolicy] = None):
        self.workspace_root = Path(workspace_path).resolve()
        self.policy = policy or CapabilityPolicy()
        if not self.workspace_root.exists():
            self.workspace_root.mkdir(parents=True, exist_ok=True)

    def _resolve_safe_path(self, relative_or_abs_path: str | Path) -> Path:
        target = Path(relative_or_abs_path)
        if not target.is_absolute():
            resolved = (self.workspace_root / target).resolve()
        else:
            resolved = target.resolve()

        # Enforce boundary containment
        try:
            resolved.relative_to(self.workspace_root)
        except ValueError:
            raise PermissionError(
                f"Path traversal detected: '{relative_or_abs_path}' escapes workspace sandbox '{self.workspace_root}'"
            )
        return resolved

    def read_file(self, rel_path: str) -> str:
        safe_path = self._resolve_safe_path(rel_path)
        if not safe_path.is_file():
            raise FileNotFoundError(f"File '{rel_path}' does not exist inside sandbox")
        return safe_path.read_text(encoding="utf-8", errors="replace")

    def write_file(self, rel_path: str, content: str) -> int:
        if self.policy.level == CapabilityLevel.READ_ONLY:
            raise PermissionError("Write access denied: Current sandbox capability is READ_ONLY")

        safe_path = self._resolve_safe_path(rel_path)
        safe_path.parent.mkdir(parents=True, exist_ok=True)
        encoded = content.encode("utf-8")
        if len(encoded) > self.policy.max_output_bytes:
            raise ValueError(f"Content size {len(encoded)} bytes exceeds max limit {self.policy.max_output_bytes}")
        safe_path.write_bytes(encoded)
        return len(encoded)

    def list_files(self, rel_path: str = ".") -> list[str]:
        safe_path = self._resolve_safe_path(rel_path)
        if not safe_path.is_dir():
            return []
        items = []
        for p in safe_path.glob("**/*"):
            if p.is_file():
                rel = p.relative_to(self.workspace_root)
                items.append(str(rel))
        return sorted(items)
