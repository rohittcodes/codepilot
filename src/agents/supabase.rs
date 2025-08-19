use anyhow::Result;
use serde_json::Value;
use swarms_rs::{llm::provider::openai::OpenAI, structs::agent::Agent};
use crate::clients::SupabaseMCPClient;
use crate::config::Config;

#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

pub struct SupabaseAgent {
    agent: Box<dyn Agent>,
    supabase_client: SupabaseMCPClient,
    available_tools: Vec<ToolInfo>,
}

impl SupabaseAgent {
    pub async fn new(api_key: String, config: &Config) -> Result<Self> {
        let supabase_client = SupabaseMCPClient::new(config);
        
        // Get dynamic tools from MCP server
        let tools_response = supabase_client.get_tools().await?;
        let available_tools: Vec<ToolInfo> = tools_response
            .iter()
            .map(|tool| ToolInfo {
                name: tool["name"].as_str().unwrap_or("unknown").to_string(),
                description: tool["description"].as_str().unwrap_or("No description").to_string(),
                input_schema: tool["inputSchema"].clone(),
            })
            .collect();
        
        let client = OpenAI::new(api_key).set_model("gpt-4-turbo");
        
        // Create dynamic system prompt with actual tool descriptions
        let tools_description = available_tools
            .iter()
            .map(|tool| format!("- {}: {}", tool.name, tool.description))
            .collect::<Vec<_>>()
            .join("\n");
        
        let system_prompt = format!(
            "You are a Supabase agent. You can ONLY use these Supabase MCP tools:

{}

CRITICAL: You are NOT allowed to use any other tools. You can ONLY mention and use the tools listed above.

When a user asks you something:
1. Look at the list of tools above
2. Find the most appropriate tool for their request
3. Mention the exact tool name you would use
4. Explain why you chose that tool

Example responses:
- 'I would use SUPABASE_SELECT_RECORDS to fetch your data'
- 'I would use SUPABASE_INSERT_RECORD to create a new record'
- 'I would use SUPABASE_UPDATE_RECORD to modify existing data'

If no tool matches the request, say: 'I don't have a tool for that request. Available tools are: [list tools]'

Remember: ONLY use tools from the list above. Never use any other tools.",
            tools_description
        );
        
        let agent = client
            .agent_builder()
            .agent_name("SupabaseAgent")
            .system_prompt(system_prompt)
            .user_name("User")
            .max_loops(1)  // Reduce loops to prevent tool calling
            .temperature(0.1)  // Lower temperature for more focused responses
            .max_tokens(2048)  // Reduce tokens to prevent long responses
            .build();

        Ok(Self {
            agent: Box::new(agent),
            supabase_client,
            available_tools,
        })
    }

    /// Process a natural language query and execute Supabase operations
    pub async fn process_query(&mut self, query: &str) -> Result<String> {
        // Use LLM for true tool selection and execution
        let llm_response = tokio::time::timeout(
            std::time::Duration::from_secs(60),
            self.agent.run(query.to_string())
        ).await;
        
        match llm_response {
            Ok(Ok(response)) => {
                if response.trim().is_empty() {
                    // LLM returned empty - tell user we can't help
                    Ok("I couldn't understand your request. Please try asking about Supabase data operations like select, insert, update, or delete records.".to_string())
                } else {
                    // LLM provided reasoning - use it to guide tool selection
                    let result = self.try_execute_supabase_operation_with_llm_guidance(query, &response).await?;
                    Ok(format!("LLM Analysis: {}\n\nSupabase Operation: {}", response, result))
                }
            }
            Ok(Err(_)) => {
                // LLM failed - tell user we can't help
                Ok("I encountered an error processing your request. Please try again or rephrase your question.".to_string())
            }
            Err(_) => {
                // LLM timed out - tell user we can't help
                Ok("I took too long to process your request. Please try again with a simpler query.".to_string())
            }
        }
    }

    /// Use LLM guidance to intelligently select and execute tools
    async fn try_execute_supabase_operation_with_llm_guidance(&self, query: &str, llm_guidance: &str) -> Result<String> {
        // Parse LLM guidance to extract tool selection
        let query_lower = query.to_lowercase();
        let guidance_lower = llm_guidance.to_lowercase();
        
        // Look for tool mentions in LLM response
        let mut best_tool: Option<&ToolInfo> = None;
        let mut best_score = 0.0;
        
        for tool in &self.available_tools {
            let tool_name_lower = tool.name.to_lowercase();
            let tool_desc_lower = tool.description.to_lowercase();
            
            // Check if LLM mentioned this tool
            if guidance_lower.contains(&tool_name_lower) || guidance_lower.contains(&tool_desc_lower) {
                let score = self.calculate_tool_relevance_score(&query_lower, tool);
                if score > best_score {
                    best_score = score;
                    best_tool = Some(tool);
                }
            }
        }
        
        // If LLM didn't mention any specific tool, use pattern matching
        if best_tool.is_none() {
            return self.try_execute_supabase_operation(query).await;
        }
        
        // Execute the LLM-selected tool
        let tool = best_tool.unwrap();
        
        // Create arguments dynamically based on the tool's schema
        let arguments = self.create_dynamic_arguments(&tool.name, &tool.input_schema, query);
        
        // Execute the operation
        match self.supabase_client.execute_tool(&tool.name, arguments).await {
            Ok(result) => {
                Ok(format!("Successfully executed {}: {:?}", tool.name, result))
            }
            Err(e) => {
                Ok(format!("Failed to execute {}: {}", tool.name, e))
            }
        }
    }

    /// Try to find and execute a relevant Supabase operation based on the query using dynamic tools
    async fn try_execute_supabase_operation(&self, query: &str) -> Result<String> {
        let query_lower = query.to_lowercase();
        
        // Score each tool based on how well it matches the query
        let mut scored_tools: Vec<(f32, &ToolInfo)> = self.available_tools
            .iter()
            .map(|tool| {
                let score = self.calculate_tool_relevance_score(&query_lower, tool);
                (score, tool)
            })
            .collect();
        
        // Sort by relevance score (highest first)
        scored_tools.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        
        // Select the single best tool (AI agent behavior)
        if let Some((score, tool)) = scored_tools.first() {
            if *score > 0.0 {
                // Create arguments dynamically based on the tool's schema
                let arguments = self.create_dynamic_arguments(&tool.name, &tool.input_schema, query);
                
                // Execute the operation
                match self.supabase_client.execute_tool(&tool.name, arguments).await {
                    Ok(result) => {
                        return Ok(format!("Successfully executed {}: {:?}", tool.name, result));
                    }
                    Err(e) => {
                        return Ok(format!("Failed to execute {}: {}", tool.name, e));
                    }
                }
            }
        }
        
        // If no tools matched well, provide helpful suggestions
        let tool_names: Vec<String> = self.available_tools
            .iter()
            .take(5)
            .map(|tool| format!("- {}: {}", tool.name, tool.description.chars().take(80).collect::<String>()))
            .collect();
        
        Ok(format!(
            "No relevant Supabase tool found for your query: '{}'\n\nAvailable tools (showing top 5):\n{}",
            query,
            tool_names.join("\n")
        ))
    }


    
    /// Calculate how relevant a tool is to the user's query
    fn calculate_tool_relevance_score(&self, query_lower: &str, tool: &ToolInfo) -> f32 {
        let mut score = 0.0;
        let tool_name_lower = tool.name.to_lowercase();
        let tool_desc_lower = tool.description.to_lowercase();
        
        // Common query patterns and their weights
        let patterns = vec![
            ("list", vec!["list", "show", "get", "fetch", "display"], 1.0),
            ("create", vec!["create", "new", "add", "make"], 1.0), 
            ("update", vec!["update", "modify", "change", "edit"], 1.0),
            ("delete", vec!["delete", "remove", "destroy"], 1.0),
            ("table", vec!["table", "schema", "database"], 0.8),
            ("record", vec!["record", "row", "data", "entry"], 0.8),
            ("select", vec!["select", "query", "read", "get"], 0.8),
            ("insert", vec!["insert", "add", "create", "new"], 0.8),
        ];
        
        for (concept, keywords, weight) in patterns.iter() {
            for keyword in keywords {
                if query_lower.contains(keyword) {
                    // Check if tool name or description contains the concept
                    if tool_name_lower.contains(concept) || tool_desc_lower.contains(concept) {
                        score += weight;
                    }
                    // Also check if tool contains the keyword directly
                    if tool_name_lower.contains(keyword) || tool_desc_lower.contains(keyword) {
                        score += weight * 0.7;
                    }
                }
            }
        }
        
        // Boost score for exact keyword matches in tool name
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        for word in query_words {
            if tool_name_lower.contains(word) {
                score += 0.5;
            }
            if tool_desc_lower.contains(word) {
                score += 0.2;
            }
        }
        
        score
    }

    /// Create arguments dynamically based on the tool's input schema
    fn create_dynamic_arguments(&self, tool_name: &str, _input_schema: &Value, query: &str) -> Value {
        // For now, use simple heuristics. In the future, this could be more sophisticated
        // by parsing the input_schema and generating appropriate arguments
        
        let tool_name_lower = tool_name.to_lowercase();
        
        if tool_name_lower.contains("list") && tool_name_lower.contains("table") {
            // List tables
            serde_json::json!({})
        } else if tool_name_lower.contains("select") || tool_name_lower.contains("get") {
            // Select records
            serde_json::json!({
                "table": "ai_generated_records",
                "columns": "*",
                "limit": 10
            })
        } else if tool_name_lower.contains("insert") || tool_name_lower.contains("create") {
            // Insert record
                serde_json::json!({
                    "table": "ai_generated_records",
                    "data": {
                        "query": query,
                        "created_at": chrono::Utc::now().to_rfc3339()
                    }
                })
        } else if tool_name_lower.contains("update") || tool_name_lower.contains("modify") {
            // Update record
                serde_json::json!({
                    "table": "ai_generated_records",
                    "data": {
                        "updated_at": chrono::Utc::now().to_rfc3339()
                    },
                    "filter": {
                        "id": "eq.1"
                    }
                })
        } else if tool_name_lower.contains("delete") || tool_name_lower.contains("remove") {
            // Delete record
                serde_json::json!({
                    "table": "ai_generated_records",
                    "filter": {
                        "id": "eq.1"
                    }
                })
        } else {
            // Default: try to extract any obvious parameters from the query
                serde_json::json!({})
        }
    }

    /// Get available Supabase tools
    pub fn get_available_operations(&self) -> Vec<String> {
        self.available_tools
            .iter()
            .map(|tool| tool.name.clone())
            .collect()
    }

    pub async fn test_connection(&self) -> Result<String> {
        match self.supabase_client.get_tools().await {
            Ok(tools) => {
                Ok(format!("Supabase MCP connection successful! Found {} tools", tools.len()))
            }
            Err(e) => {
                Ok(format!("Supabase MCP connection failed: {}", e))
            }
        }
    }
} 