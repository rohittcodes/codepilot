use anyhow::Result;
use async_openai::{
    config::OpenAIConfig,
    types::chat::{
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
    Client,
};
use std::path::PathBuf;

use crate::config::Config;
use crate::runs::{self, RunKind, RunStatus};

/// A single proposed file edit: write `content` to `path` (relative to the target repo).
#[derive(Debug, Clone)]
pub struct FileEdit {
    pub path: String,
    pub content: String,
}

/// Result of running one code task end-to-end.
#[derive(Debug, Clone)]
pub struct TaskResult {
    pub edit: FileEdit,
    pub target_path: PathBuf,
    pub verification: RunStatus,
    /// `true` if the edit passed verification and was kept on disk; `false` if it
    /// failed and was reverted (no retry loop yet - that's the next slice).
    pub applied: bool,
}

/// Orchestrates a single "code task" flow: take a task description, ask the LLM
/// to propose a file edit, write it to disk, and gate it on `tsc --noEmit`.
///
/// Only the type-check gate exists so far - ESLint, the generated-test gate, and
/// the bounded retry loop are the next slices of Phase 2 (see PLAN.md).
pub struct CodeTaskOrchestrator {
    client: Client<OpenAIConfig>,
    model: String,
    target_repo_path: PathBuf,
}

const SYSTEM_PROMPT: &str = "You are a coding agent that edits files in a JS/TS codebase. \
Given a task, propose exactly one file edit. Respond in EXACTLY this format, with no other text:

FILE: <relative path to the file>
---
<the full new content of the file>

Always output the complete file content, not a diff or snippet.";

impl CodeTaskOrchestrator {
    pub async fn new(config: &Config) -> Result<Self> {
        let api_key = config
            .openai_api_key
            .clone()
            .ok_or_else(|| anyhow::anyhow!("OPENAI_API_KEY must be set"))?;

        let mut openai_config = OpenAIConfig::new().with_api_key(api_key);
        if let Some(base_url) = &config.openai_base_url {
            openai_config = openai_config.with_api_base(base_url.clone());
        }

        Ok(Self {
            client: Client::with_config(openai_config),
            model: "gpt-4-turbo".to_string(),
            target_repo_path: PathBuf::from(&config.target_repo_path),
        })
    }

    /// Run a single task end-to-end: ask the LLM for an edit, then write it to disk.
    pub async fn run_task(&mut self, task: &str) -> Result<TaskResult> {
        let system_message = ChatCompletionRequestSystemMessageArgs::default()
            .content(SYSTEM_PROMPT)
            .build()?;
        let user_message = ChatCompletionRequestUserMessageArgs::default()
            .content(task)
            .build()?;

        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model)
            .messages(vec![system_message.into(), user_message.into()])
            .temperature(0.2)
            .max_completion_tokens(4096u32)
            .build()?;

        let response = self.client.chat().create(request).await?;
        let content = response
            .choices
            .first()
            .and_then(|choice| choice.message.content.clone())
            .filter(|content| !content.trim().is_empty())
            .ok_or_else(|| anyhow::anyhow!("LLM returned an empty response"))?;

        let edit = Self::parse_file_edit(&content)?;
        let target_path = self.resolve_safe_path(&edit.path)?;

        // Captured so a failed gate can put the file back exactly as it was,
        // rather than leaving a half-applied, unverified edit on disk.
        let previous_content = std::fs::read_to_string(&target_path).ok();

        if let Some(parent) = target_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&target_path, &edit.content)?;

        let verification = runs::execute(RunKind::TypeCheck, &self.target_repo_path).await?;

        let applied = match &verification {
            RunStatus::Succeeded => true,
            RunStatus::Failed(_) => {
                match &previous_content {
                    Some(content) => std::fs::write(&target_path, content)?,
                    None => std::fs::remove_file(&target_path)?,
                }
                false
            }
        };

        Ok(TaskResult {
            edit,
            target_path,
            verification,
            applied,
        })
    }

    /// Resolve `path` against the target repo root, rejecting any path that would
    /// escape it (e.g. via `../..`).
    fn resolve_safe_path(&self, path: &str) -> Result<PathBuf> {
        std::fs::create_dir_all(&self.target_repo_path)?;
        let repo_root = std::fs::canonicalize(&self.target_repo_path)?;

        let joined = repo_root.join(path);
        let parent = joined
            .parent()
            .ok_or_else(|| anyhow::anyhow!("proposed edit path has no parent directory: {path}"))?;
        std::fs::create_dir_all(parent)?;
        let canonical_parent = std::fs::canonicalize(parent)?;

        if !canonical_parent.starts_with(&repo_root) {
            return Err(anyhow::anyhow!(
                "proposed edit path escapes target repo: {path}"
            ));
        }

        Ok(canonical_parent.join(joined.file_name().ok_or_else(|| {
            anyhow::anyhow!("proposed edit path has no file name: {path}")
        })?))
    }

    /// Parse the agent's `FILE: <path>\n---\n<content>` response into a FileEdit.
    fn parse_file_edit(response: &str) -> Result<FileEdit> {
        let (header, content) = response
            .split_once("---")
            .ok_or_else(|| anyhow::anyhow!("Agent response missing '---' separator: {response}"))?;

        let path = header
            .lines()
            .find_map(|line| line.trim().strip_prefix("FILE:"))
            .map(|p| p.trim().to_string())
            .ok_or_else(|| anyhow::anyhow!("Agent response missing 'FILE:' header: {response}"))?;

        if path.is_empty() {
            return Err(anyhow::anyhow!("Agent response had an empty file path"));
        }

        Ok(FileEdit {
            path,
            content: content.trim_start_matches('\n').to_string(),
        })
    }
}
