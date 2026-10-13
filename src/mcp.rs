use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

pub type ToolHandler = Arc<dyn Fn(Value) -> Result<Value, String> + Send + Sync>;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

pub struct RegisteredTool {
    pub definition: ToolDefinition,
    pub handler: ToolHandler,
}

pub struct MCPServer {
    pub name: String,
    pub version: String,
    pub protocol_version: String,
    tools: HashMap<String, RegisteredTool>,
}

impl MCPServer {
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            protocol_version: "2024-11-05".to_string(),
            tools: HashMap::new(),
        }
    }

    pub fn register_tool(
        &mut self,
        name: &str,
        description: &str,
        input_schema: Value,
        handler: ToolHandler,
    ) {
        self.tools.insert(
            name.to_string(),
            RegisteredTool {
                definition: ToolDefinition {
                    name: name.to_string(),
                    description: description.to_string(),
                    input_schema,
                },
                handler,
            },
        );
    }

    pub fn handle_request(&self, request: &Value) -> Value {
        let req_id = request.get("id").cloned();
        let method = match request.get("method").and_then(|m| m.as_str()) {
            Some(m) => m,
            None => {
                return json!({
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "error": { "code": -32600, "message": "Invalid Request: missing method" }
                });
            }
        };
        let params = request.get("params").cloned().unwrap_or(json!({}));

        match method {
            "initialize" => json!({
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {
                    "protocolVersion": self.protocol_version,
                    "serverInfo": { "name": self.name, "version": self.version },
                    "capabilities": {
                        "tools": { "listChanged": true }
                    }
                }
            }),
            "tools/list" => {
                let tool_list: Vec<&ToolDefinition> =
                    self.tools.values().map(|t| &t.definition).collect();
                json!({
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": { "tools": tool_list }
                })
            }
            "tools/call" => {
                let tool_name = match params.get("name").and_then(|n| n.as_str()) {
                    Some(n) => n,
                    None => {
                        return json!({
                            "jsonrpc": "2.0",
                            "id": req_id,
                            "error": { "code": -32602, "message": "Missing 'name' in tools/call params" }
                        });
                    }
                };
                let args = params.get("arguments").cloned().unwrap_or(json!({}));

                match self.tools.get(tool_name) {
                    Some(tool) => match (tool.handler)(args) {
                        Ok(res) => json!({
                            "jsonrpc": "2.0",
                            "id": req_id,
                            "result": {
                                "content": [{
                                    "type": "text",
                                    "text": if res.is_string() { res.as_str().unwrap().to_string() } else { res.to_string() }
                                }],
                                "isError": false
                            }
                        }),
                        Err(err) => json!({
                            "jsonrpc": "2.0",
                            "id": req_id,
                            "result": {
                                "content": [{ "type": "text", "text": format!("Tool error: {}", err) }],
                                "isError": true
                            }
                        }),
                    },
                    None => json!({
                        "jsonrpc": "2.0",
                        "id": req_id,
                        "error": { "code": -32601, "message": format!("Tool '{}' not found", tool_name) }
                    }),
                }
            }
            _ => json!({
                "jsonrpc": "2.0",
                "id": req_id,
                "error": { "code": -32601, "message": format!("Method '{}' not found", method) }
            }),
        }
    }
}

pub struct MCPClient<'a> {
    pub server: &'a MCPServer,
    msg_seq: usize,
}

impl<'a> MCPClient<'a> {
    pub fn new(server: &'a MCPServer) -> Self {
        Self { server, msg_seq: 0 }
    }

    fn next_id(&mut self) -> usize {
        self.msg_seq += 1;
        self.msg_seq
    }

    pub fn initialize(&mut self) -> Result<Value, String> {
        let req = json!({
            "jsonrpc": "2.0",
            "id": self.next_id(),
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "clientInfo": { "name": "ezagents-rust-client", "version": "0.1.0" }
            }
        });
        let res = self.server.handle_request(&req);
        if let Some(err) = res.get("error") {
            return Err(err.to_string());
        }
        Ok(res.get("result").cloned().unwrap_or(json!({})))
    }

    pub fn list_tools(&mut self) -> Result<Vec<ToolDefinition>, String> {
        let req = json!({
            "jsonrpc": "2.0",
            "id": self.next_id(),
            "method": "tools/list",
            "params": {}
        });
        let res = self.server.handle_request(&req);
        if let Some(err) = res.get("error") {
            return Err(err.to_string());
        }
        let tools_val = res
            .get("result")
            .and_then(|r| r.get("tools"))
            .cloned()
            .unwrap_or(json!([]));
        serde_json::from_value(tools_val).map_err(|e| e.to_string())
    }

    pub fn call_tool(&mut self, name: &str, arguments: Value) -> Result<String, String> {
        let req = json!({
            "jsonrpc": "2.0",
            "id": self.next_id(),
            "method": "tools/call",
            "params": {
                "name": name,
                "arguments": arguments
            }
        });
        let res = self.server.handle_request(&req);
        if let Some(err) = res.get("error") {
            return Err(err.to_string());
        }
        let result = res.get("result").ok_or("Missing result field")?;
        let is_error = result
            .get("isError")
            .and_then(|e| e.as_bool())
            .unwrap_or(false);
        let content_arr = result
            .get("content")
            .and_then(|c| c.as_array())
            .ok_or("Missing content array")?;
        let text = content_arr
            .first()
            .and_then(|item| item.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();

        if is_error {
            Err(text)
        } else {
            Ok(text)
        }
    }
}
