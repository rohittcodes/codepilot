use std::env;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct Config {
    // LLM Configuration
    pub openai_base_url: Option<String>,
    pub openai_api_key: Option<String>,
    
    // Composio Configuration
    pub composio_base_url: String,
    pub composio_api_key: String,
    
    // MCP Server URLs
    pub supabase_mcp_url: String,
    pub linear_mcp_url: String,
    pub github_mcp_url: String,
    
    // Agent Configuration
    pub agent_name: String,
    pub user_name: String,
    pub system_prompt: String,
    pub max_retries: u32,
    pub max_loops: u32,
    pub save_state_dir: Option<String>,
    
    // Logging
    pub log_level: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenv::dotenv().ok();
        
        Ok(Self {
            // LLM Configuration
            openai_base_url: env::var("OPENAI_BASE_URL").ok(),
            openai_api_key: env::var("OPENAI_API_KEY")
                .ok(),
            
            // Composio Configuration
            composio_base_url: env::var("COMPOSIO_BASE_URL")
                .unwrap_or_else(|_| "https://backend.composio.dev/unify".to_string()),
            composio_api_key: env::var("COMPOSIO_API_KEY")
                .expect("COMPOSIO_API_KEY must be set"),
            
            // MCP Server URLs
            supabase_mcp_url: env::var("SUPABASE_MCP_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8001/sse".to_string()),
            linear_mcp_url: env::var("LINEAR_MCP_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8002/sse".to_string()),
            github_mcp_url: env::var("GITHUB_MCP_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8003/sse".to_string()),
            
            // Agent Configuration
            agent_name: env::var("AGENT_NAME")
                .unwrap_or_else(|_| "MultiAgentOrchestrator".to_string()),
            user_name: env::var("USER_NAME")
                .unwrap_or_else(|_| "User".to_string()),
            system_prompt: env::var("SYSTEM_PROMPT")
                .unwrap_or_else(|_| "You are a helpful AI assistant.".to_string()),
            max_retries: env::var("MAX_RETRIES")
                .unwrap_or_else(|_| "3".to_string())
                .parse()
                .unwrap_or(3),
            max_loops: env::var("MAX_LOOPS")
                .unwrap_or_else(|_| "50".to_string())
                .parse()
                .unwrap_or(50),
            save_state_dir: env::var("SAVE_STATE_DIR").ok(),
            
            // Logging
            log_level: env::var("RUST_LOG")
                .unwrap_or_else(|_| "info".to_string()),
        })
    }
    
    pub fn validate(&self) -> Result<()> {
        if self.openai_api_key.as_ref().map_or(true, |s| s.is_empty()) {
            return Err(anyhow::anyhow!("OPENAI_API_KEY cannot be empty"));
        }
        
        if self.composio_api_key.is_empty() {
            return Err(anyhow::anyhow!("COMPOSIO_API_KEY cannot be empty"));
        }
        
        Ok(())
    }
    
    pub fn get_mcp_url(&self, service: &str) -> &str {
        match service {
            "supabase" => &self.supabase_mcp_url,
            "linear" => &self.linear_mcp_url,
            "github" => &self.github_mcp_url,
            _ => &self.supabase_mcp_url, // default
        }
    }
}

// Helper functions for getting API keys
pub fn get_composio_api_key() -> Result<String> {
    std::env::var("COMPOSIO_API_KEY").map_err(|_| anyhow::anyhow!("COMPOSIO_API_KEY not found"))
}

pub fn get_openai_api_key() -> Result<String> {
    std::env::var("OPENAI_API_KEY").map_err(|_| anyhow::anyhow!("OPENAI_API_KEY not found"))
}

pub fn get_openai_base_url() -> Result<String> {
    Ok(std::env::var("OPENAI_BASE_URL")
        .unwrap_or_else(|_| "https://api.openai.com/v1".to_string()))
} 