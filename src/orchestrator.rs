use anyhow::Result;
use swarms_rs::{llm::provider::openai::OpenAI, structs::swarms_router::{SwarmRouter, SwarmRouterConfig, SwarmType}};
use std::env;

/// Multi-Agent Orchestrator that coordinates Linear, GitHub, and Supabase agents
pub struct MultiAgentOrchestrator {
    router: SwarmRouter,
}

impl MultiAgentOrchestrator {
    pub async fn new() -> Result<Self> {
        let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
        let client = OpenAI::new(api_key).set_model("gpt-4-turbo");

        // Create a routing agent that decides which service to use
        let agent = client
            .agent_builder()
            .system_prompt("You are a routing orchestrator that decides which service to use for user requests.

When users ask about:
- Linear: issues, projects, tasks, assignments, project management -> respond with 'USE_LINEAR_AGENT'
- GitHub: repositories, pull requests, code, commits, branches -> respond with 'USE_GITHUB_AGENT'  
- Supabase: databases, tables, records, queries, data storage -> respond with 'USE_SUPABASE_AGENT'
- General questions or greetings -> respond normally

For service-specific requests, start your response with the service directive (e.g., 'USE_LINEAR_AGENT: ') followed by the original query.")
            .agent_name("OrchestratorAgent")
            .user_name("User")
            .max_loops(3)
            .max_tokens(4096)
            .build();

        let config = SwarmRouterConfig {
            name: "OpenAI Orchestrator".to_string(),
            description: "Coordinates operations using OpenAI and MCP tools".to_string(),
            swarm_type: SwarmType::SequentialWorkflow,
            agents: vec![agent],
            rules: None,
            multi_agent_collab_prompt: false,
        };

        let router = SwarmRouter::new_with_config(config)?;
        Ok(Self { router })
    }

    /// Process a natural language query and orchestrate the appropriate agents
    pub async fn process_query(&self, query: &str) -> Result<String> {
        let result = self.router.run(query).await?;
        Ok(result.to_string())
    }
} 