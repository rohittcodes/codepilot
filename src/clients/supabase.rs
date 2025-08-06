use anyhow::Result;
use serde_json::Value;
use reqwest::Client;
use crate::config::Config;

pub struct SupabaseMCPClient {
    client: Client,
    mcp_url: String,
}

impl SupabaseMCPClient {
    pub fn new(config: &Config) -> Self {
        Self {
            client: Client::new(),
            mcp_url: config.get_mcp_url("supabase").to_string(),
        }
    }

    pub async fn get_tools(&self) -> Result<Vec<Value>> {
        // Add retry logic for rate limiting
        for attempt in 1..=3 {
            let request_body = serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/list",
                "params": {}
            });

            let response = self
                .client
                .post(&self.mcp_url)
                .header("Content-Type", "application/json")
                .header("Accept", "application/json, text/event-stream")
                .json(&request_body)
                .send()
                .await?;

            if response.status().is_success() {
                // Handle SSE response
                let response_text = response.text().await?;
                
                // Parse SSE format: "event: message\ndata: {json}\n\n"
                for line in response_text.lines() {
                    if line.starts_with("data: ") {
                        let json_str = &line[6..]; // Remove "data: " prefix
                        if let Ok(response_data) = serde_json::from_str::<Value>(json_str) {
                            // Extract tools from the MCP response
                            if let Some(tools) = response_data["result"]["tools"].as_array() {
                                return Ok(tools.clone());
                            }
                        }
                    }
                }
                return Err(anyhow::anyhow!("No valid tools found in SSE response"));
            } else if response.status().as_u16() == 429 {
                // Rate limited - wait and retry
                if attempt < 3 {
                    tokio::time::sleep(tokio::time::Duration::from_secs(attempt * 2)).await;
                    continue;
                }
            }
            
            // If we get here, it's not a rate limit issue
            return Err(anyhow::anyhow!(
                "Failed to fetch tools from Supabase MCP: {}",
                response.status()
            ));
        }
        
        // If we get here, all attempts failed
        Err(anyhow::anyhow!("Failed to fetch tools from Supabase MCP after 3 attempts"))
    }

    pub async fn execute_tool(&self, tool_name: &str, arguments: Value) -> Result<Value> {
        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": arguments
            }
        });

        let response = self
            .client
            .post(&self.mcp_url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .json(&request_body)
            .send()
            .await?;

        if response.status().is_success() {
            let response_text = response.text().await?;
            
            // Parse SSE response
            for line in response_text.lines() {
                if line.starts_with("data: ") {
                    let json_str = &line[6..];
                    if let Ok(response_data) = serde_json::from_str::<Value>(json_str) {
                        if let Some(result) = response_data["result"].as_object() {
                            return Ok(serde_json::json!(result));
                        }
                    }
                }
            }
            
            // If no SSE response, try parsing as regular JSON
            if let Ok(response_data) = serde_json::from_str::<Value>(&response_text) {
                if let Some(result) = response_data["result"].as_object() {
                    return Ok(serde_json::json!(result));
                }
            }
            
            return Err(anyhow::anyhow!("No valid result found in response"));
        }
        
        Err(anyhow::anyhow!("Failed to execute tool on Supabase MCP: {}", response.status()))
    }

    pub async fn get_tool_schema(&self, tool_name: &str) -> Result<Value> {
        // This would typically call a specific method to get tool schema
        // For now, we'll return a placeholder
        Ok(serde_json::json!({
            "name": tool_name,
            "description": "Supabase tool schema"
        }))
    }

    pub async fn list_operations(&self) -> Result<Vec<String>> {
        let tools = self.get_tools().await?;
        let operations: Vec<String> = tools
            .iter()
            .filter_map(|tool| tool["name"].as_str().map(|s| s.to_string()))
            .collect();
        Ok(operations)
    }
} 