<!-- markdownlint-disable-file -->

# Task Checklist: Transcript Service CLI

## Overview

Transform `transcript-merger` into a `transcript` binary with Ollama-style subcommands, in-memory transcript state, and a local HTTP API for data access.

## Objectives

- Rename the compiled binary from `transcript-merger` to `transcript`
- Add `clap 4` subcommand CLI with `serve`, `ls`, `ps`, `stop`, `get`, `api`, `mcp` subcommands
- Embed an `axum 0.8` HTTP server in `serve` exposing `/transcript` and `/health` endpoints
- Keep transcript state exclusively in process memory (`Arc<Mutex<Vec<Entry>>>`) — never written to disk
- Write a session JSON file (pid + port only) on `serve` start; delete on exit
- Implement client subcommands (`get last`, `api last`) that connect to the running `serve` via HTTP
- `mcp` subcommands implemented as stubs for future work

## Research Summary

### Project Files

- `transcript-merger/src/main.rs` — current single-binary implementation to be refactored
- `transcript-merger/Cargo.toml` — package config; needs `[[bin]]` rename and new dependencies

### External References

- `.copilot-tracking/research/20260722-transcript-cli-research.md` — complete architecture design, CLI shape, session file schema, axum router sketch, and implementation guidance

## Implementation Checklist

### [ ] Phase 1: Cargo.toml Configuration

- [ ] Task 1.1: Add `[[bin]]` entry to rename binary to `transcript`
  - Details: `.copilot-tracking/details/20260723-transcript-cli-details.md` (Lines 11–28)

- [ ] Task 1.2: Add `clap`, `axum`, `serde`, `serde_json`, `reqwest` dependencies
  - Details: `.copilot-tracking/details/20260723-transcript-cli-details.md` (Lines 30–49)

### [ ] Phase 2: CLI Types and Data Structures

- [ ] Task 2.1: Define `Cli`, `Command`, `Entry`, `SessionFile` types and wire up `main` dispatch
  - Details: `.copilot-tracking/details/20260723-transcript-cli-details.md` (Lines 53–134)

### [ ] Phase 3: `serve` Subcommand

- [ ] Task 3.1: Refactor child-process spawning with channel selection and `TRANSCRIPT_MODEL` env var
  - Details: `.copilot-tracking/details/20260723-transcript-cli-details.md` (Lines 138–158)

- [ ] Task 3.2: Write session file on start, delete on exit, and embed axum HTTP server
  - Details: `.copilot-tracking/details/20260723-transcript-cli-details.md` (Lines 160–207)

### [ ] Phase 4: Session Management (`ls`, `ps`, `stop`)

- [ ] Task 4.1: Implement `ls_sessions()` and `ps_sessions()` reading from `%APPDATA%\transcript\sessions\`
  - Details: `.copilot-tracking/details/20260723-transcript-cli-details.md` (Lines 211–241)

- [ ] Task 4.2: Implement `stop_session()` via `taskkill /PID`
  - Details: `.copilot-tracking/details/20260723-transcript-cli-details.md` (Lines 243–263)

### [ ] Phase 5: Client Subcommands (`get`, `api`, `mcp`)

- [ ] Task 5.1: Implement `get_transcript()` — HTTP GET `/transcript` from newest session
  - Details: `.copilot-tracking/details/20260723-transcript-cli-details.md` (Lines 267–289)

- [ ] Task 5.2: Implement `api_url()` — print the `/transcript` endpoint URL
  - Details: `.copilot-tracking/details/20260723-transcript-cli-details.md` (Lines 291–310)

- [ ] Task 5.3: Implement `mcp_stub()` — "not yet implemented" stubs for both variants
  - Details: `.copilot-tracking/details/20260723-transcript-cli-details.md` (Lines 312–334)

## Dependencies

- `clap 4` with `derive` feature
- `axum 0.8`
- `serde 1` with `derive` feature
- `serde_json 1`
- `reqwest 0.12` with `json` feature
- `chrono 0.4` (already present)
- `tokio 1` with `full` features (already present)

## Success Criteria

- `transcript --help` prints all subcommands
- `transcript serve` starts, streams merged transcript to stdout, writes session file, serves HTTP on port 11435
- `transcript ls` / `transcript ps` list running sessions
- `transcript stop` terminates the running serve process
- `transcript get last` fetches and prints the current in-memory transcript
- `transcript api last` prints the HTTP endpoint URL
- `transcript mcp last` and `transcript mcp --http` exit cleanly with a stub message
- No transcript data is ever written to disk
- Binary name is `transcript` (not `transcript-merger`)
