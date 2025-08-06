// MCP client modules for different services
pub mod linear;
pub mod github;
pub mod supabase;

pub use linear::LinearMCPClient;
pub use github::GitHubMCPClient;
pub use supabase::SupabaseMCPClient; 