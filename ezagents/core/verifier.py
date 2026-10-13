"""Deterministic Verification Engine.
Enforces real code assertion and ground-truth testing rather than relying on LLM promises.
"""

from __future__ import annotations

import subprocess
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional


@dataclass
class VerificationResult:
    passed: bool
    name: str
    details: str
    execution_time_ms: float
    metadata: Dict[str, Any] = field(default_factory=dict)


class DeterministicVerifier:
    """Runs repeatable validation checks against artifacts produced by an agent."""

    def __init__(self, workspace_path: Path):
        self.workspace_path = Path(workspace_path).resolve()
        self.checks: List[tuple[str, Callable[[Path], tuple[bool, str]]]] = []

    def add_file_exists_check(self, rel_path: str) -> None:
        def check(ws: Path) -> tuple[bool, str]:
            target = ws / rel_path
            exists = target.exists()
            msg = f"File '{rel_path}' exists" if exists else f"File '{rel_path}' is missing"
            return exists, msg

        self.checks.append((f"FileExists:{rel_path}", check))

    def add_file_content_check(self, rel_path: str, substring: str) -> None:
        def check(ws: Path) -> tuple[bool, str]:
            target = ws / rel_path
            if not target.is_file():
                return False, f"File '{rel_path}' does not exist"
            content = target.read_text(encoding="utf-8", errors="replace")
            matched = substring in content
            msg = f"Substring found in '{rel_path}'" if matched else f"Substring '{substring}' not found in '{rel_path}'"
            return matched, msg

        self.checks.append((f"ContentMatch:{rel_path}", check))

    def add_command_check(self, command: List[str], expected_exit_code: int = 0) -> None:
        def check(ws: Path) -> tuple[bool, str]:
            try:
                proc = subprocess.run(
                    command,
                    cwd=str(ws),
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    timeout=30,
                    text=True,
                )
                passed = proc.returncode == expected_exit_code
                msg = f"Command {' '.join(command)} exit code: {proc.returncode} (expected {expected_exit_code})\nStdout: {proc.stdout[:200]}"
                return passed, msg
            except Exception as e:
                return False, f"Execution failed: {e}"

        self.checks.append((f"Command:{' '.join(command[:3])}", check))

    def run_all(self) -> List[VerificationResult]:
        results = []
        for name, check_fn in self.checks:
            start_t = time.perf_counter()
            try:
                passed, detail = check_fn(self.workspace_path)
            except Exception as ex:
                passed, detail = False, f"Check crashed: {ex}"
            elapsed = (time.perf_counter() - start_t) * 1000
            results.append(
                VerificationResult(
                    passed=passed,
                    name=name,
                    details=detail,
                    execution_time_ms=elapsed,
                )
            )
        return results
