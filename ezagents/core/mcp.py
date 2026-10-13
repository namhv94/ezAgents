"""Model Context Protocol (MCP) Client & Server implementation.
Adheres to Anthropic MCP Specification (2024-11-05).
Zero external dependencies, fast JSON-RPC 2.0 protocol layer.
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from typing import Any, Callable, Dict, List, Optional


@dataclass
class ToolDefinition:
    name: str
    description: str
    input_schema: Dict[str, Any]
    handler: Optional[Callable[[Dict[str, Any]], Any]] = None

    def to_dict(self) -> Dict[str, Any]:
        return {
            "name": self.name,
            "description": self.description,
            "inputSchema": self.input_schema,
        }


@dataclass
class ResourceDefinition:
    uri: str
    name: str
    description: str = ""
    mime_type: str = "text/plain"
    reader: Optional[Callable[[str], str]] = None

    def to_dict(self) -> Dict[str, Any]:
        return {
            "uri": self.uri,
            "name": self.name,
            "description": self.description,
            "mimeType": self.mime_type,
        }


class MCPServer:
    """In-process and protocol-compliant MCP Server."""

    def __init__(self, name: str, version: str = "0.1.0"):
        self.name = name
        self.version = version
        self.tools: Dict[str, ToolDefinition] = {}
        self.resources: Dict[str, ResourceDefinition] = {}
        self.protocol_version = "2024-11-05"

    def register_tool(
        self,
        name: str,
        description: str,
        input_schema: Dict[str, Any],
        handler: Optional[Callable[[Dict[str, Any]], Any]] = None,
    ) -> None:
        self.tools[name] = ToolDefinition(
            name=name,
            description=description,
            input_schema=input_schema,
            handler=handler,
        )

    def register_resource(
        self,
        uri: str,
        name: str,
        description: str = "",
        mime_type: str = "text/plain",
        reader: Optional[Callable[[str], str]] = None,
    ) -> None:
        self.resources[uri] = ResourceDefinition(
            uri=uri,
            name=name,
            description=description,
            mime_type=mime_type,
            reader=reader,
        )

    def handle_request(self, request_payload: str | bytes | dict) -> Dict[str, Any]:
        if isinstance(request_payload, (str, bytes)):
            try:
                msg = json.loads(request_payload)
            except Exception as e:
                return {
                    "jsonrpc": "2.0",
                    "id": None,
                    "error": {"code": -32700, "message": f"Parse error: {e}"},
                }
        else:
            msg = request_payload

        req_id = msg.get("id")
        method = msg.get("method")
        params = msg.get("params", {})

        if not method:
            return {
                "jsonrpc": "2.0",
                "id": req_id,
                "error": {"code": -32600, "message": "Invalid Request: method is required"},
            }

        # Dispatch standard MCP methods
        if method == "initialize":
            return {
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {
                    "protocolVersion": self.protocol_version,
                    "serverInfo": {"name": self.name, "version": self.version},
                    "capabilities": {
                        "tools": {"listChanged": True},
                        "resources": {"subscribe": True},
                    },
                },
            }

        elif method == "tools/list":
            return {
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {
                    "tools": [tool.to_dict() for tool in self.tools.values()]
                },
            }

        elif method == "tools/call":
            tool_name = params.get("name")
            arguments = params.get("arguments", {})
            if tool_name not in self.tools:
                return {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "error": {
                        "code": -32601,
                        "message": f"Tool '{tool_name}' not found",
                    },
                }
            tool = self.tools[tool_name]
            if not tool.handler:
                return {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "error": {
                        "code": -32603,
                        "message": f"Tool '{tool_name}' has no registered handler",
                    },
                }
            try:
                res = tool.handler(arguments)
                return {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": json.dumps(res) if not isinstance(res, str) else res,
                            }
                        ],
                        "isError": False,
                    },
                }
            except Exception as ex:
                return {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "content": [{"type": "text", "text": f"Error executing {tool_name}: {ex}"}],
                        "isError": True,
                    },
                }

        elif method == "resources/list":
            return {
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {
                    "resources": [r.to_dict() for r in self.resources.values()]
                },
            }

        elif method == "resources/read":
            uri = params.get("uri")
            if uri not in self.resources:
                return {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "error": {"code": -32602, "message": f"Resource '{uri}' not found"},
                }
            res_obj = self.resources[uri]
            try:
                content_str = res_obj.reader(uri) if res_obj.reader else ""
                return {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "contents": [
                            {
                                "uri": uri,
                                "mimeType": res_obj.mime_type,
                                "text": content_str,
                            }
                        ]
                    },
                }
            except Exception as ex:
                return {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "error": {"code": -32603, "message": str(ex)},
                }

        return {
            "jsonrpc": "2.0",
            "id": req_id,
            "error": {"code": -32601, "message": f"Method '{method}' not found"},
        }


class MCPClient:
    """Client for negotiating and communicating with an MCP Server."""

    def __init__(self, server: MCPServer):
        self.server = server
        self.msg_seq = 0
        self.initialized = False
        self.server_info: Dict[str, Any] = {}
        self.cached_tools: List[Dict[str, Any]] = []

    def _next_id(self) -> int:
        self.msg_seq += 1
        return self.msg_seq

    def initialize(self) -> Dict[str, Any]:
        req = {
            "jsonrpc": "2.0",
            "id": self._next_id(),
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "clientInfo": {"name": "ezAgents-client", "version": "0.1.0"},
                "capabilities": {},
            },
        }
        res = self.server.handle_request(req)
        if "error" in res:
            raise RuntimeError(f"MCP Initialize failed: {res['error']}")
        self.initialized = True
        self.server_info = res.get("result", {})
        return self.server_info

    def list_tools(self) -> List[Dict[str, Any]]:
        if not self.initialized:
            self.initialize()
        req = {
            "jsonrpc": "2.0",
            "id": self._next_id(),
            "method": "tools/list",
            "params": {},
        }
        res = self.server.handle_request(req)
        if "error" in res:
            raise RuntimeError(f"MCP list_tools failed: {res['error']}")
        self.cached_tools = res.get("result", {}).get("tools", [])
        return self.cached_tools

    def call_tool(self, name: str, arguments: Dict[str, Any]) -> Any:
        if not self.initialized:
            self.initialize()
        req = {
            "jsonrpc": "2.0",
            "id": self._next_id(),
            "method": "tools/call",
            "params": {"name": name, "arguments": arguments},
        }
        res = self.server.handle_request(req)
        if "error" in res:
            raise RuntimeError(f"MCP tool call error: {res['error']}")
        call_res = res.get("result", {})
        if call_res.get("isError"):
            err_msg = "".join(c.get("text", "") for c in call_res.get("content", []))
            raise RuntimeError(f"Tool '{name}' execution failed: {err_msg}")
        # Parse return text
        content_items = call_res.get("content", [])
        if content_items:
            text = content_items[0].get("text", "")
            try:
                return json.loads(text)
            except Exception:
                return text
        return None
