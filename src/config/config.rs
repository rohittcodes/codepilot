use std::env;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct Config {
    // LLM Configuration
    pub openai_base_url: Option<String>,
    pub openai_api_key: Option<String>,

    // Agent Configuration
    pub agent_name: String,
    pub user_name: String,
    pub system_prompt: String,
    pub max_retries: u32,
    pub max_loops: u32,
    pub save_state_dir: Option<String>,

    // Target repo for code edits
    pub target_repo_path: String,

    // Logging
    pub log_level: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenv::dotenv().ok();

        Ok(Self {
            // LLM Configuration
            openai_base_url: env::var("OPENAI_BASE_URL").ok(),
            openai_api_key: env::var("OPENAI_API_KEY").ok(),

            // Agent Configuration
            agent_name: env::var("AGENT_NAME")
                .unwrap_or_else(|_| "CodePilotAgent".to_string()),
            user_name: env::var("USER_NAME")
                .unwrap_or_else(|_| "User".to_string()),
            system_prompt: env::var("SYSTEM_PROMPT")
                .unwrap_or_else(|_| "You are a coding agent that edits JS/TS codebases.".to_string()),
            max_retries: env::var("MAX_RETRIES")
                .unwrap_or_else(|_| "3".to_string())
                .parse()
                .unwrap_or(3),
            max_loops: env::var("MAX_LOOPS")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .unwrap_or(10),
            save_state_dir: env::var("SAVE_STATE_DIR").ok(),

            target_repo_path: env::var("TARGET_REPO_PATH")
                .unwrap_or_else(|_| ".".to_string()),

            // Logging
            log_level: env::var("RUST_LOG")
                .unwrap_or_else(|_| "info".to_string()),
        })
    }

    pub fn validate(&self) -> Result<()> {
        if self.openai_api_key.as_ref().map_or(true, |s| s.is_empty()) {
            return Err(anyhow::anyhow!("OPENAI_API_KEY cannot be empty"));
        }

        Ok(())
    }
}

pub fn get_openai_api_key() -> Result<String> {
    std::env::var("OPENAI_API_KEY").map_err(|_| anyhow::anyhow!("OPENAI_API_KEY not found"))
}

pub fn get_openai_base_url() -> Result<String> {
    Ok(std::env::var("OPENAI_BASE_URL")
        .unwrap_or_else(|_| "https://api.openai.com/v1".to_string()))
}
