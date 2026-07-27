<!-- markdownlint-disable-file -->

# Implementation Prompt: Transcript Service CLI

## Task Overview

Transform `transcript-merger/src/main.rs` and `transcript-merger/Cargo.toml` into a `transcript` binary with Ollama-style subcommands (`serve`, `ls`, `ps`, `stop`, `get`, `api`, `mcp`). The `serve` subcommand embeds an axum HTTP server; all other subcommands are thin HTTP clients or session-file readers. No transcript data is ever written to disk.

## Implementation Instructions

### Step 1: Execute implementation

Follow `.copilot-tracking/plans/20260723-transcript-cli-plan.md` task-by-task, checking off each item as it is completed. Follow ALL project standards and conventions defined in CLAUDE.md.

For each task:
1. Read the full task details from `.copilot-tracking/details/20260723-transcript-cli-details.md` at the referenced line range
2. Implement the task completely with working code
3. Mark the task `[x]` in the plan file
4. Append entries to `.copilot-tracking/changes/20260723-transcript-cli-changes.md` (Added / Modified / Removed)

### Step 2: Cleanup

When ALL phases are checked off (`[x]`) and completed:

1. Provide a brief summary of all changes made, with markdown links to each modified file.
2. Provide links to:
   - `.copilot-tracking/plans/20260723-transcript-cli-plan.md`
   - `.copilot-tracking/details/20260723-transcript-cli-details.md`
   - `.copilot-tracking/research/20260722-transcript-cli-research.md`
   Recommend cleaning these tracking files up after review.
3. Delete this prompt file: `.copilot-tracking/prompts/implement-transcript-cli.prompt.md`

## Success Criteria

- [ ] All plan items implemented with working code
- [ ] `cargo check` passes in `transcript-merger/`
- [ ] `transcript --help` shows all subcommands
- [ ] `transcript serve` starts, writes session file, streams stdout, serves HTTP on port 11435
- [ ] `transcript ls` and `transcript stop` work against a running session
- [ ] `transcript get last` and `transcript api last` return data from the running serve process
- [ ] No transcript data written to disk
- [ ] Plan file fully checked off; changes file updated after every task
