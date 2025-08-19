# CodePilot Architecture

## System Overview

```mermaid
graph TB
    %% User Interface Layer
    subgraph "User Interface Layer"
        UI[Terminal UI<br/>ratatui + crossterm]
        CLI[CLI Application<br/>Rust Binary]
    end

    %% Application Layer
    subgraph "Application Layer"
        App[App State Management<br/>AppState + Config]
        Orchestrator[Multi-Agent Orchestrator<br/>MultiAgentOrchestrator]
        Formatter[Response Formatter<br/>ResponseFormatter]
    end

    %% Agent Layer
    subgraph "Agent Layer"
        LinearAgent[Linear Agent<br/>LinearAgent]
        GitHubAgent[GitHub Agent<br/>GitHubAgent]
        SupabaseAgent[Supabase Agent<br/>SupabaseAgent]
    end

    %% Client Layer
    subgraph "Client Layer"
        LinearClient[Linear MCP Client<br/>LinearMCPClient]
        GitHubClient[GitHub MCP Client<br/>GitHubMCPClient]
        SupabaseClient[Supabase MCP Client<br/>SupabaseMCPClient]
    end

    %% LLM Layer
    subgraph "LLM Layer"
        OpenAI[OpenAI API<br/>GPT-4 Turbo]
        Swarms[swarms-rs<br/>Agent Framework]
    end

    %% MCP Layer
    subgraph "MCP Layer"
        LinearMCP[Linear MCP Server<br/>Composio]
        GitHubMCP[GitHub MCP Server<br/>Composio]
        SupabaseMCP[Supabase MCP Server<br/>Composio]
    end

    %% External Services
    subgraph "External Services"
        LinearAPI[Linear API<br/>Project Management]
        GitHubAPI[GitHub API<br/>Repository Management]
        SupabaseAPI[Supabase API<br/>Database Operations]
    end

    %% Configuration & Environment
    subgraph "Configuration"
        Env[Environment Variables<br/>.env file]
        Config[Config Management<br/>Config struct]
    end

    %% Data Flow
    UI --> CLI
    CLI --> App
    App --> Orchestrator
    Orchestrator --> LinearAgent
    Orchestrator --> GitHubAgent
    Orchestrator --> SupabaseAgent

    LinearAgent --> LinearClient
    GitHubAgent --> GitHubClient
    SupabaseAgent --> SupabaseClient

    LinearAgent --> Swarms
    GitHubAgent --> Swarms
    SupabaseAgent --> Swarms

    Swarms --> OpenAI

    LinearClient --> LinearMCP
    GitHubClient --> GitHubMCP
    SupabaseClient --> SupabaseMCP

    LinearMCP --> LinearAPI
    GitHubMCP --> GitHubAPI
    SupabaseMCP --> SupabaseAPI

    App --> Config
    Config --> Env

    %% Styling
    classDef rustComponent fill:#ff6b6b,stroke:#333,stroke-width:2px,color:#fff
    classDef externalService fill:#4ecdc4,stroke:#333,stroke-width:2px,color:#fff
    classDef mcpLayer fill:#45b7d1,stroke:#333,stroke-width:2px,color:#fff
    classDef llmLayer fill:#96ceb4,stroke:#333,stroke-width:2px,color:#fff
    classDef configLayer fill:#feca57,stroke:#333,stroke-width:2px,color:#333

    class UI,CLI,App,Orchestrator,Formatter,LinearAgent,GitHubAgent,SupabaseAgent,LinearClient,GitHubClient,SupabaseClient rustComponent
    class LinearAPI,GitHubAPI,SupabaseAPI externalService
    class LinearMCP,GitHubMCP,SupabaseMCP mcpLayer
    class OpenAI,Swarms llmLayer
    class Env,Config configLayer
```

## Detailed Component Architecture

```mermaid
graph LR
    %% User Input Flow
    subgraph "User Input"
        User[User Query]
        InputMode[Input Mode<br/>crossterm events]
    end

    %% Processing Flow
    subgraph "Processing"
        State[App State<br/>AppState]
        Router[Query Router<br/>MultiAgentOrchestrator]
        Agent[Agent Selection<br/>Linear/GitHub/Supabase]
    end

    %% Tool Execution Flow
    subgraph "Tool Execution"
        ToolDiscovery[Tool Discovery<br/>MCP get_tools]
        ToolScoring[Tool Scoring<br/>calculate_tool_relevance_score]
        ToolExecution[Tool Execution<br/>execute_tool]
    end

    %% Response Flow
    subgraph "Response"
        Formatter[Response Formatter<br/>ResponseFormatter]
        UI[Terminal UI<br/>ratatui]
    end

    %% Data Storage
    subgraph "Data"
        Messages[Message History<br/>Vec of String]
        Tools[Tool Cache<br/>Vec of ToolInfo]
        Config[Configuration<br/>Config struct]
    end

    %% External APIs
    subgraph "External APIs"
        OpenAI[OpenAI API<br/>GPT-4 Turbo]
        LinearAPI[Linear API<br/>Project Management]
        GitHubAPI[GitHub API<br/>Repository Management]
        SupabaseAPI[Supabase API<br/>Database Operations]
    end

    %% MCP Servers
    subgraph "MCP Servers"
        LinearMCP[Linear MCP<br/>Composio Server]
        GitHubMCP[GitHub MCP<br/>Composio Server]
        SupabaseMCP[Supabase MCP<br/>Composio Server]
    end

    %% Flow Connections
    User --> InputMode
    InputMode --> State
    State --> Router
    Router --> Agent
    Agent --> ToolDiscovery
    ToolDiscovery --> ToolScoring
    ToolScoring --> ToolExecution
    ToolExecution --> Formatter
    Formatter --> UI
    UI --> User

    %% Data Connections
    State --> Messages
    Agent --> Tools
    Router --> Config

    %% API Connections
    ToolExecution --> LinearMCP
    ToolExecution --> GitHubMCP
    ToolExecution --> SupabaseMCP
    Agent --> OpenAI

    LinearMCP --> LinearAPI
    GitHubMCP --> GitHubAPI
    SupabaseMCP --> SupabaseAPI

    %% Styling
    classDef userLayer fill:#ff9ff3,stroke:#333,stroke-width:2px,color:#fff
    classDef processingLayer fill:#54a0ff,stroke:#333,stroke-width:2px,color:#fff
    classDef executionLayer fill:#5f27cd,stroke:#333,stroke-width:2px,color:#fff
    classDef responseLayer fill:#00d2d3,stroke:#333,stroke-width:2px,color:#fff
    classDef dataLayer fill:#ff9f43,stroke:#333,stroke-width:2px,color:#fff
    classDef apiLayer fill:#10ac84,stroke:#333,stroke-width:2px,color:#fff
    classDef mcpLayer fill:#ff6b6b,stroke:#333,stroke-width:2px,color:#fff

    class User,InputMode userLayer
    class State,Router,Agent processingLayer
    class ToolDiscovery,ToolScoring,ToolExecution executionLayer
    class Formatter,UI responseLayer
    class Messages,Tools,Config dataLayer
    class OpenAI,LinearAPI,GitHubAPI,SupabaseAPI apiLayer
    class LinearMCP,GitHubMCP,SupabaseMCP mcpLayer
```

## Technology Stack

```mermaid
graph TB
    %% Core Technologies
    subgraph "Core Technologies"
        Rust[Rust<br/>Programming Language]
        Tokio[Tokio<br/>Async Runtime]
        Serde[Serde<br/>Serialization]
        Anyhow[Anyhow<br/>Error Handling]
    end

    %% UI Technologies
    subgraph "UI Technologies"
        Ratatui[Ratatui<br/>Terminal UI]
        Crossterm[Crossterm<br/>Terminal Control]
    end

    %% AI/ML Technologies
    subgraph "AI/ML Technologies"
        Swarms[swarms-rs<br/>Agent Framework]
        OpenAI[OpenAI API<br/>GPT-4 Turbo]
    end

    %% Network Technologies
    subgraph "Network Technologies"
        Reqwest[Reqwest<br/>HTTP Client]
        SerdeJson[Serde JSON<br/>JSON Handling]
    end

    %% Configuration Technologies
    subgraph "Configuration"
        Dotenv[Dotenv<br/>Environment Variables]
        Chrono[Chrono<br/>Date/Time]
    end

    %% External Services
    subgraph "External Services"
        Linear[Linear<br/>Project Management]
        GitHub[GitHub<br/>Repository Management]
        Supabase[Supabase<br/>Database]
        Composio[Composio<br/>MCP Servers]
    end

    %% Dependencies
    Rust --> Tokio
    Rust --> Serde
    Rust --> Anyhow
    Rust --> Ratatui
    Rust --> Crossterm
    Rust --> Swarms
    Rust --> Reqwest
    Rust --> SerdeJson
    Rust --> Dotenv
    Rust --> Chrono

    %% Service Connections
    Swarms --> OpenAI
    Reqwest --> Linear
    Reqwest --> GitHub
    Reqwest --> Supabase
    Reqwest --> Composio

    %% Styling
    classDef coreTech fill:#ff6b6b,stroke:#333,stroke-width:2px,color:#fff
    classDef uiTech fill:#4ecdc4,stroke:#333,stroke-width:2px,color:#fff
    classDef aiTech fill:#45b7d1,stroke:#333,stroke-width:2px,color:#fff
    classDef networkTech fill:#96ceb4,stroke:#333,stroke-width:2px,color:#fff
    classDef configTech fill:#feca57,stroke:#333,stroke-width:2px,color:#333
    classDef externalService fill:#ff9ff3,stroke:#333,stroke-width:2px,color:#fff

    class Rust,Tokio,Serde,Anyhow coreTech
    class Ratatui,Crossterm uiTech
    class Swarms,OpenAI aiTech
    class Reqwest,SerdeJson networkTech
    class Dotenv,Chrono configTech
    class Linear,GitHub,Supabase,Composio externalService
```

## Data Flow Architecture

```mermaid
sequenceDiagram
    participant User
    participant UI as Terminal UI
    participant App as App State
    participant Orchestrator as Multi-Agent Orchestrator
    participant Agent as Specialized Agent
    participant MCP as MCP Client
    participant MCP_Server as MCP Server
    participant External as External API
    participant LLM as OpenAI

    User->>UI: Enter query
    UI->>App: Update input state
    App->>Orchestrator: Route query
    Orchestrator->>Agent: Select appropriate agent
    
    Agent->>LLM: Process with LLM
    LLM-->>Agent: LLM response
    
    Agent->>MCP: Discover tools
    MCP->>MCP_Server: get_tools()
    MCP_Server-->>MCP: Available tools
    MCP-->>Agent: Tool list
    
    Agent->>Agent: Score tools by relevance
    Agent->>MCP: Execute best tool
    MCP->>MCP_Server: execute_tool()
    MCP_Server->>External: Call external API
    External-->>MCP_Server: API response
    MCP_Server-->>MCP: Tool result
    MCP-->>Agent: Execution result
    
    Agent->>App: Format response
    App->>UI: Update display
    UI-->>User: Show result
```

## Component Dependencies

```mermaid
graph TD
    %% Main Application
    Main[main.rs] --> App[app.rs]
    App --> State[state.rs]
    App --> UI[ui.rs]
    
    %% Agent System
    App --> Orchestrator[orchestrator.rs]
    Orchestrator --> LinearAgent[agents/linear.rs]
    Orchestrator --> GitHubAgent[agents/github.rs]
    Orchestrator --> SupabaseAgent[agents/supabase.rs]
    
    %% Client System
    LinearAgent --> LinearClient[clients/linear.rs]
    GitHubAgent --> GitHubClient[clients/github.rs]
    SupabaseAgent --> SupabaseClient[clients/supabase.rs]
    
    %% Configuration
    App --> Config[config/config.rs]
    Config --> Env[.env file]
    
    %% Utilities
    App --> Formatter[formatter.rs]
    
    %% Dependencies
    LinearClient --> Reqwest[reqwest]
    GitHubClient --> Reqwest
    SupabaseClient --> Reqwest
    
    LinearAgent --> Swarms[swarms-rs]
    GitHubAgent --> Swarms
    SupabaseAgent --> Swarms
    
    UI --> Ratatui[ratatui]
    UI --> Crossterm[crossterm]
    
    App --> Tokio[tokio]
    App --> Anyhow[anyhow]
    App --> Serde[serde]
    
    %% Styling
    classDef mainComponent fill:#ff6b6b,stroke:#333,stroke-width:2px,color:#fff
    classDef agentComponent fill:#4ecdc4,stroke:#333,stroke-width:2px,color:#fff
    classDef clientComponent fill:#45b7d1,stroke:#333,stroke-width:2px,color:#fff
    classDef configComponent fill:#96ceb4,stroke:#333,stroke-width:2px,color:#fff
    classDef utilComponent fill:#feca57,stroke:#333,stroke-width:2px,color:#333
    classDef dependency fill:#ff9ff3,stroke:#333,stroke-width:2px,color:#fff

    class Main,App,State,UI mainComponent
    class Orchestrator,LinearAgent,GitHubAgent,SupabaseAgent agentComponent
    class LinearClient,GitHubClient,SupabaseClient clientComponent
    class Config,Env configComponent
    class Formatter utilComponent
    class Reqwest,Swarms,Ratatui,Crossterm,Tokio,Anyhow,Serde dependency
```

## Key Features Highlighted

- **Multi-Agent Architecture**: Specialized agents for different domains
- **Dynamic Tool Discovery**: Tools fetched from MCP servers at runtime
- **Intelligent Routing**: Query routing based on content and agent capabilities
- **Beautiful Terminal UI**: Modern interface with real-time status updates
- **Robust Error Handling**: Graceful fallbacks and helpful error messages
- **Configuration Management**: Environment-based configuration
- **Type Safety**: Rust's type system ensures reliability
- **Async Operations**: Tokio runtime for concurrent operations 