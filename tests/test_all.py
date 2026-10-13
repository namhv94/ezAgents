import unittest
import tempfile
import shutil
from pathlib import Path

from ezagents.core.mcp import MCPServer, MCPClient
from ezagents.core.state import StateMachine, RunState
from ezagents.core.sandbox import ScopedSandbox, CapabilityLevel, CapabilityPolicy
from ezagents.core.verifier import DeterministicVerifier
from ezagents.core.orchestrator import AgentOrchestrator, OrchestratorConfig


class TestMCPLayer(unittest.TestCase):
    def setUp(self):
        self.server = MCPServer(name="test-server")
        self.server.register_tool(
            name="echo",
            description="Echo back input",
            input_schema={"type": "object", "properties": {"msg": {"type": "string"}}},
            handler=lambda args: f"echo: {args.get('msg')}",
        )
        self.client = MCPClient(self.server)

    def test_initialize_and_tool_discovery(self):
        info = self.client.initialize()
        self.assertEqual(info["protocolVersion"], "2024-11-05")
        tools = self.client.list_tools()
        self.assertEqual(len(tools), 1)
        self.assertEqual(tools[0]["name"], "echo")

    def test_tool_call_execution(self):
        res = self.client.call_tool("echo", {"msg": "hello-ezlabs"})
        self.assertEqual(res, "echo: hello-ezlabs")

    def test_missing_tool_call(self):
        with self.assertRaises(RuntimeError):
            self.client.call_tool("nonexistent", {})


class TestStateMachine(unittest.TestCase):
    def test_lifecycle_and_transition(self):
        sm = StateMachine("task_1")
        self.assertEqual(sm.current_state, RunState.DRAFT)
        sm.transition_to(RunState.PLANNING)
        self.assertEqual(sm.current_state, RunState.PLANNING)
        sm.transition_to(RunState.RUNNING)
        self.assertEqual(sm.current_state, RunState.RUNNING)

    def test_illegal_transition(self):
        sm = StateMachine("task_bad")
        with self.assertRaises(ValueError):
            sm.transition_to(RunState.COMPLETED)  # Illegal jump from DRAFT to COMPLETED

    def test_rollback(self):
        sm = StateMachine("task_rb", initial_data={"val": 1})
        sm.transition_to(RunState.PLANNING, {"val": 2})
        sm.transition_to(RunState.RUNNING, {"val": 3})
        self.assertEqual(sm.data["val"], 3)
        # Roll back to initial step 0
        sm.rollback_to_step(0)
        self.assertEqual(sm.current_state, RunState.ROLLED_BACK)
        self.assertEqual(sm.data["val"], 1)


class TestSandbox(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.mkdtemp()
        self.sandbox = ScopedSandbox(self.temp_dir)

    def tearDown(self):
        shutil.rmtree(self.temp_dir, ignore_errors=True)

    def test_safe_write_and_read(self):
        self.sandbox.write_file("sub/file.txt", "ezlabs-content")
        content = self.sandbox.read_file("sub/file.txt")
        self.assertEqual(content, "ezlabs-content")

    def test_path_traversal_prevention(self):
        with self.assertRaises(PermissionError):
            self.sandbox.read_file("../../etc/passwd")


class TestOrchestrator(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.mkdtemp()
        self.config = OrchestratorConfig(task_id="run_101", workspace_path=self.temp_dir)
        self.orchestrator = AgentOrchestrator(self.config)

    def tearDown(self):
        shutil.rmtree(self.temp_dir, ignore_errors=True)

    def test_full_pipeline_verification(self):
        self.orchestrator.plan_phase({"goal": "create file"})
        self.orchestrator.execute_step("write_file", {"path": "hello.py", "content": "print('ok')"})

        self.orchestrator.verifier.add_file_exists_check("hello.py")
        self.orchestrator.verifier.add_file_content_check("hello.py", "print('ok')")
        passed, results = self.orchestrator.verify_and_complete()

        self.assertTrue(passed)
        self.assertEqual(self.orchestrator.state_machine.current_state, RunState.COMPLETED)


if __name__ == "__main__":
    unittest.main()
