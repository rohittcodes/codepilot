# CodePilot v2 — Plan

## Why this exists

CodePilot today is a Rust multi-agent CLI that routes queries to Linear/GitHub/Supabase agents via MCP servers. It has real traction (13 stars, 2 forks) but is a single commit — essentially a scaffold, not a finished product.

We're repurposing it entirely: **CodePilot becomes a coding agent for JS/TS codebases, whose core differentiator is verifiably safe autonomous execution.**

The gap we're targeting: every major coding agent today (Claude Code, Cursor, Aider, pi.dev) assumes a human is watching and approving each risky action. None of them are built to be trusted running *unattended* on a real codebase — there's no mechanical proof a change is safe, just an LLM's self-report. We're building the one that is.

The wedge has two parts, and neither alone is enough:
- **Rust implementation** — reuses what already exists here (Tokio async, the multi-agent orchestration pattern, the Ratatui TUI shell), ships as a single binary, and is a real technical differentiator (every competitor above is built in TS or Python — same pattern as Biome/oxc/SWC/ruff: Rust tooling built for another language's ecosystem).
- **Verification-gated changes, not LLM self-report** — every agent-made change must pass `tsc` (type-check), ESLint (lint-clean), and a generated test before it's considered done. A change that fails any gate is rejected/retried automatically, never shipped silently.

Note: the verification primitives themselves (`tsc`, ESLint) are Node-based — there's no mature Rust-native TypeScript type-checker yet. The Rust orchestrator will shell out to these as subprocesses. This is expected and matches how respected Rust dev-tools already interoperate with JS tooling at the edges where no Rust-native equivalent exists.

## What's being dropped from the current codebase

- `agents/linear.rs`, `agents/github.rs`, `agents/supabase.rs` and their corresponding clients — the Linear/GitHub/Supabase ops-agent framing is gone
- The MCP-based tool-discovery integration as the primary mechanism (may return later for a different purpose, not in scope now)
- `formatter.rs` — was built for chat-style responses about ops tools, not code diffs; will likely be rewritten

## What's being kept and reused

- Tokio async runtime setup (`main.rs`, `lib.rs`)
- Ratatui + Crossterm TUI shell (`cli/ui.rs`, `cli/state.rs`, `cli/app.rs`) — repurposed to show live agent activity, verification status, and diffs instead of ops-agent chat
- The orchestrator pattern (`orchestrator.rs`) — same shape (route a task to the right handler), repointed at code-editing operations instead of agent selection
- `config/config.rs` — env-based configuration, reusable as-is
- `async-openai` as the LLM client (swapped in for `swarms-rs` once multi-agent routing was gone — we need deterministic single-shot completions driven by our own retry loop, not an agent framework's own loop/router)

## Target architecture

```
User task
   |
   v
[Planner] -- breaks task into concrete file edits, using LLM via async-openai
   |
   v
[Sandbox] -- isolates execution: a dedicated git worktree per task (FS scope is
   |          the worktree, not the user's working tree), no network except the
   |          configured LLM provider, no destructive ops outside an explicit
   |          allowlist, minimal subprocess environment, per-command timeout
   v
[Editor] -- applies proposed edits to JS/TS files
   |
   v
[Verification Gate] -- the core of the product:
   |     1. tsc --noEmit (type-check passes)
   |     2. eslint (lint-clean)
   |     3. generated test for the change (written by the agent, must pass)
   |     -> any failure: retry the edit with the failure as feedback, bounded retries
   |     -> all pass: accept the change
   v
[Trust Report] -- emitted per change: what was verified, pass/fail per gate,
   |              diff, plain-language explanation of what changed and why
   v
[TUI] -- shows live status: planning -> editing -> verifying -> done/rejected,
         final trust report rendered at the end
```

## Execution modes

One pipeline (Planner → Sandbox → Editor → Verification Gate), three ways to run it — a control layer over the existing flow, not separate code paths:

- **Plan mode** — runs only the Planner, against the target repo read-only. Produces the intended file edits and the commands it would run (tsc/ESLint/test), but writes nothing and executes nothing. Lowest risk; available as soon as the Planner exists (Phase 1), since it's just "stop before Editor."
- **Ask mode** — runs the full pipeline, but the TUI pauses for explicit confirmation before each consequential action (writing a file, running a shell command). Useful while trusting the agent on a new/unfamiliar repo. Gets more valuable as Phase 2 (verification subprocesses) and Phase 3 (shell allowlist) land, since there's more to confirm.
- **Agent mode** — the full autonomous loop, no per-action confirmation: propose edit → write → verify → retry (bounded) → accept/reject. This is the default, trusted-unattended-execution pitch of the project, and is what Phases 2-4 are building. Still bounded by the Sandbox regardless of mode — modes control *confirmation*, not the safety rails.

Mode is configured via `.env` (`AGENT_MODE=plan|ask|agent`) and should be visible in the TUI status bar, as a distinct badge from the existing input mode (NORMAL/INSERT) — don't conflate the two "mode" concepts.

Non-goal guardrail: modes are a thin control layer; Plan mode reuses the same Planner code path and just stops early, it is not a parallel "preview" implementation.

## Phased build plan

**Phase 1 — Strip and repoint (housekeeping, ~1-2 days)**
- Remove Linear/GitHub/Supabase agents, clients, MCP integration
- Repoint orchestrator at a single "code task" flow (no multi-domain routing yet)
- Get a trivial end-to-end path working: take a task string, call the LLM, write a file edit, no verification yet

**Phase 2 — Verification gate (the actual product, ~1-2 weeks)**
- Move off the Phase 1 `FILE:`/`---` full-file-rewrite edit format onto a structured/diff-style multi-file edit representation — full-file rewrites are noisy to diff and expensive to retry, and won't scale once the retry loop is driving repeated edits
- Path-traversal validation (the proposed edit path must resolve inside `target_repo_path`) is needed as soon as these subprocesses exist, not deferred to Phase 3 — `tsc`/ESLint will run against whatever path the LLM proposes
- Subprocess execution goes through one `Run` abstraction (`RunKind::TypeCheck/Lint/Test`, statuses `Pending/Running/Succeeded/Failed`, captured stdout/stderr) instead of three ad hoc wrappers — Phase 3's shell allowlist later becomes `RunKind::Shell(cmd)` on the same plumbing, not a new mechanism
- `tsc --noEmit` run as a `Run`, parsed for pass/fail + errors
- ESLint run as a `Run`, same pattern
- Agent-generated test step, run as a `Run`: this is a **sub-agent** call — an isolated system prompt/context that's only given the `FileEdit` it's writing a test for, with no shared retry history with the main planner — not a generic agent-spawning framework, just a second isolated LLM call with its own prompt and parser
- Task-shape classification biases the planner's system prompt for known JS/TS shapes (fix-type-error, add-test, fix-lint-violation, etc.) before the main edit call — a small, fixed, compiled-in enum, not a plugin/skill-loading system
- Retry loop: on any gate failure, feed the failure back to the planner and re-attempt, bounded to N retries before surfacing a failure to the user. Bound the retry context with a deterministic (non-LLM) summarizer that keeps the latest attempt's errors verbatim and collapses older attempts to one line each — never re-send full file content from prior attempts
- This phase is the entire pitch — do not skip steps here for speed

**Phase 3 — Sandbox (~3-5 days)**
- Each task runs in its own `git worktree` (`git worktree add <scratch-dir>/<task-id> <branch>`) rather than directly on the user's working tree — this is the FS-isolation mechanism (the single-path-escape check from Phase 2 still applies *within* the worktree). On success, merge the worktree branch back; on failure or a merge conflict, fail mechanically and surface it to the user — never have the agent "decide what's best" in a conflict, that's exactly the unverified self-report judgment the whole project exists to avoid. Worktree-per-task is adopted now purely as isolation for one task at a time, not as parallelism (see non-goals)
- Restrict network access to just the configured LLM provider endpoint
- Explicit allowlist for any shell commands the agent can run (no arbitrary `rm`, no unscoped `git push`, etc.)
- Subprocess hardening for everything run via the `Run` abstraction (Phase 2): spawn with a minimal explicit environment (not the full inherited env — no cloud credentials, no SSH agent socket, etc.), pin `current_dir` to the task's worktree, and enforce a per-command timeout that kills runaway processes
- Heavier OS-level sandboxing (seccomp/Landlock on Linux, AppContainer on Windows, sandbox-exec on macOS) is real hardening but a large, platform-specific lift — explicitly a stretch goal, not required to ship Phase 3

**Phase 4 — Trust report + TUI polish (~3-5 days)**
- Render live status per task in the TUI (planning/editing/verifying/done)
- Generate the final trust report (what passed, diff, explanation) as both TUI output and a saved artifact (e.g. markdown file) for sharing/demo purposes

**Phase 5 — Demo + README**
- Record a demo showing the agent running unattended on a small real JS/TS repo, including at least one case where it catches its own bad edit via the verification gate and self-corrects
- Rewrite README and `ARCHITECTURE.md` to reflect the new direction
- Basic CI (build + lint) on the repo itself

## Explicit non-goals (for now)

- No other languages besides JS/TS
- No IDE integration
- No multi-model provider switching (pick one provider, get it working end to end first)
- No general-purpose "do anything" agent framing — stay narrow to the verification-gated code-editing story
- No user-defined or dynamically loaded skills — the task-shape set is a fixed, compiled-in enum
- No generic task-runner/job-queue framework — `Run` is a closed enum scoped to tsc/ESLint/test/allowlisted-shell
- No parallel multi-agent execution, router, or master-agent merge arbitration yet — one task at a time; Phase 3's worktree-per-task is adopted purely as the FS-isolation mechanism, not a committed step toward parallelism. If/when parallelism is built, conflicts must be resolved mechanically (re-run the verification gate), never by an agent's judgment call
- No OS-level process sandboxing (seccomp/Landlock/AppContainer/etc.) in v1 — worktree isolation + path validation + command allowlist + minimal subprocess env + per-command timeout is the v1 layered sandbox; OS-level hardening is a documented future stretch, not a Phase 3 blocker
