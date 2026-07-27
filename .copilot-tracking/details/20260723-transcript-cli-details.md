<!-- markdownlint-disable-file -->

# Task Details: Transcript Service CLI

## Research Reference

**Source Research**: `.copilot-tracking/research/20260722-transcript-cli-research.md`

## Phase 1: Cargo.toml Configuration

### Task 1.1: Add `[[bin]]` entry to rename binary to `transcript`

Add a `[[bin]]` section to `transcript-merger/Cargo.toml` specifying `name = "transcript"` and `path = "src/main.rs"`. This changes the compiled binary from `transcript-merger.exe` to `transcript.exe`.

```toml
[[bin]]
name = "transcript"
path = "src/main.rs"
```

- **Files**:
  - `transcript-merger/Cargo.toml` — add `[[bin]]` section after `[package]`
- **Success**:
  - `cargo build` produces `target/debug/transcript.exe`
- **Research references**:
  - `.copilot-tracking/research/20260722-transcript-cli-research.md` (Lines 96–110) — Cargo.toml additions with `[[bin]]` and full dependency list
- **Dependencies**:
  - None — first task

### Task 1.2: Add new crate dependencies

Add to `[dependencies]` in `transcript-merger/Cargo.toml`:

```toml
clap       = { version = "4", features = ["derive"] }
axum       = "0.8"
serde      = { version = "1", features = ["derive"] }
serde_json = "1"
reqwest    = { version = "0.12", features = ["json"] }
```

- **Files**:
  - `transcript-merger/Cargo.toml` — extend `[dependencies]` block
- **Success**:
  - `cargo check` resolves all new crates without error
- **Research references**:
  - `.copilot-tracking/research/20260722-transcript-cli-research.md` (Lines 97–106) — exact dependency versions
- **Dependencies**:
  - Task 1.1 (same file)

## Phase 2: CLI Types and Data Structures

### Task 2.1: Define `Cli`, `Command`, `Entry`, and `SessionFile` types

Replace the existing `Entry` struct and `main` function with the full set of types needed by all subcommands. Add `clap` and `serde` imports.

```rust
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct Entry {
    timestamp: String,
    source: String,
    text: String,
}

#[derive(Serialize, Deserialize)]
struct SessionFile {
    id: String,
    pid: u32,
    port: u16,
    channels: Vec<String>,
    model: String,
    started_at: String,
}

#[derive(Parser)]
#[command(name = "transcript")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Serve {
        #[arg(long, value_delimiter = ',', default_values = ["mic", "speaker"])]
        channel: Vec<String>,
        #[arg(long, default_value = "nemotron-speech-streaming-en-0.6b")]
        model: String,
        #[arg(long, default_value_t = 11435)]
        port: u16,
    },
    Ls,
    Ps,
    Stop,
    Get { what: String },
    Api { what: String },
    Mcp {
        what: Option<String>,
        #[arg(long)] http: bool,
    },
}
```

`main` becomes:

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Command::Serve { channel, model, port } => serve(channel, model, port).await?,
        Command::Ls => ls_sessions()?,
        Command::Ps => ps_sessions()?,
        Command::Stop => stop_session()?,
        Command::Get { what } => get_transcript(&what).await?,
        Command::Api { what } => api_url(&what)?,
        Command::Mcp { what, http } => mcp_stub(what, http)?,
    }
    Ok(())
}
```

- **Files**:
  - `transcript-merger/src/main.rs` — replace entire file top section with new types; add dispatch in `main`
- **Success**:
  - `cargo check` compiles without errors
  - `transcript --help` shows all subcommands
- **Research references**:
  - `.copilot-tracking/research/20260722-transcript-cli-research.md` (Lines 112–145) — complete CLI shape with clap derive
- **Dependencies**:
  - Phase 1 complete

## Phase 3: `serve` Subcommand

### Task 3.1: Refactor child-process spawning with channel selection and model env var

Update `serve()` to accept `channels: Vec<String>`, `model: String`, `port: u16` and:
- Spawn `mic-transcription.exe` only when `channels.contains("mic")`
- Spawn `speaker-transcription.exe` only when `channels.contains("speaker")`
- Pass model via `.env("TRANSCRIPT_MODEL", &model)` on each child `Command`
- Keep existing `Arc<Mutex<Vec<Entry>>>` and `Arc<Mutex<PrintState>>` merge logic

Signature: `async fn serve(channels: Vec<String>, model: String, port: u16) -> Result<(), Box<dyn std::error::Error>>`

- **Files**:
  - `transcript-merger/src/main.rs` — refactor existing spawn block into `serve()` function
- **Success**:
  - `transcript serve` spawns both children by default
  - `transcript serve --channel mic` spawns only mic
  - `TRANSCRIPT_MODEL` env var is set on child processes
- **Research references**:
  - `.copilot-tracking/research/20260722-transcript-cli-research.md` (Lines 48–53) — channel/model design
  - `.copilot-tracking/research/20260722-transcript-cli-research.md` (Lines 148–165) — serve handler sketch
- **Dependencies**:
  - Task 2.1 complete

### Task 3.2: Write session file and embed axum HTTP server

Within `serve()`, after spawning children but before the select loop:

1. **Write session file**: Generate `id = Local::now().format("YYYYMMDD-HHmmss")`, collect `pid = std::process::id()`. Write to `%APPDATA%\transcript\sessions\{id}.json`. Create the directory if it does not exist. Use `serde_json::to_string_pretty`.

2. **Register cleanup**: After the select completes (Ctrl-C or children exit), delete the session file with `std::fs::remove_file`.

3. **Start axum server**: Build a router with two routes sharing `Arc<Mutex<Vec<Entry>>>` via closure capture:

```rust
let t_http = transcript.clone();
let health_channels = channels.clone();
let health_model = model.clone();
let app = axum::Router::new()
    .route("/transcript", axum::routing::get(move || {
        let entries = t_http.lock().unwrap();
        let json = serde_json::to_string(&*entries).unwrap();
        async move { axum::response::Response::builder()
            .header("content-type", "application/json")
            .body(json)
            .unwrap() }
    }))
    .route("/health", axum::routing::get(move || {
        // return pid, channels, model, entry_count as JSON
        async move { "ok" }
    }));

tokio::spawn(async move {
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await.unwrap();
    axum::serve(listener, app).await.unwrap();
});
```

Note: axum 0.8 handler closures have stricter lifetime requirements. Use `Arc::clone` inside the handler closure, not direct capture of `MutexGuard`. For the `/transcript` route, clone the Arc before the handler, move the Arc into the handler, and lock inside the async block.

- **Files**:
  - `transcript-merger/src/main.rs` — add session file write/delete and axum router in `serve()`
- **Success**:
  - After `transcript serve`, `%APPDATA%\transcript\sessions\` contains a JSON file
  - `curl http://127.0.0.1:11435/transcript` returns a JSON array
  - `curl http://127.0.0.1:11435/health` returns OK
  - Session file is deleted when process exits (Ctrl-C or natural)
- **Research references**:
  - `.copilot-tracking/research/20260722-transcript-cli-research.md` (Lines 79–93) — session file schema (id, pid, port, channels, model, started_at)
  - `.copilot-tracking/research/20260722-transcript-cli-research.md` (Lines 148–165) — axum router sketch
- **Dependencies**:
  - Task 3.1 complete

## Phase 4: Session Management (`ls`, `ps`, `stop`)

### Task 4.1: Implement `ls_sessions()` and `ps_sessions()`

Add a helper `fn sessions_dir() -> PathBuf` that returns `dirs::data_local_dir()` / `transcript` / `sessions` (or fall back to `%APPDATA%\transcript\sessions` via env var). Read all `*.json` files, deserialize as `SessionFile`, sort by `started_at`.

`ls_sessions()`: print a formatted table:
```
ID                  PID    PORT   CHANNELS        MODEL                                   STARTED
20260723-100532     12345  11435  mic,speaker     nemotron-speech-streaming-en-0.6b       2026-07-23T10:05:32Z
```

`ps_sessions()`: print compact one-line per session: `{id} pid={pid} port={port}`.

Since `dirs` crate is not a dependency, resolve the sessions directory as:
```rust
fn sessions_dir() -> std::path::PathBuf {
    let base = std::env::var("APPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    base.join("transcript").join("sessions")
}
```

- **Files**:
  - `transcript-merger/src/main.rs` — add `sessions_dir()`, `ls_sessions()`, `ps_sessions()` functions
- **Success**:
  - `transcript ls` prints table of sessions (empty output if none)
  - `transcript ps` prints compact list
- **Research references**:
  - `.copilot-tracking/research/20260722-transcript-cli-research.md` (Lines 66–74) — ls/ps behavior
- **Dependencies**:
  - Task 3.2 complete

### Task 4.2: Implement `stop_session()`

Read all session files via `sessions_dir()`, pick the most recently `started_at` entry, extract `pid`, and run:

```rust
std::process::Command::new("taskkill")
    .args(["/PID", &session.pid.to_string(), "/F"])
    .status()?;
```

Print `"Stopped session {id} (pid {pid})"` on success.

- **Files**:
  - `transcript-merger/src/main.rs` — add `stop_session()` function
- **Success**:
  - `transcript stop` terminates the running serve process
  - Subsequent `transcript ls` shows no sessions (session file cleanup by killed process)
- **Research references**:
  - `.copilot-tracking/research/20260722-transcript-cli-research.md` (Lines 211–212) — stop implementation via taskkill
- **Dependencies**:
  - Task 4.1 complete (session dir resolution reused)

## Phase 5: Client Subcommands (`get`, `api`, `mcp`)

### Task 5.1: Implement `get_transcript(what: &str)`

Resolve the newest session (same logic as `stop_session`). Build URL `http://127.0.0.1:{port}/transcript`. Call `reqwest::get(&url).await?.text().await?` and print to stdout. Return an error if no session found.

```rust
async fn get_transcript(what: &str) -> Result<(), Box<dyn std::error::Error>> {
    // "last" is the only supported value for now
    let session = newest_session()?;
    let url = format!("http://127.0.0.1:{}/transcript", session.port);
    let body = reqwest::get(&url).await?.text().await?;
    println!("{body}");
    Ok(())
}
```

- **Files**:
  - `transcript-merger/src/main.rs` — add `get_transcript()` async function; add `newest_session()` helper
- **Success**:
  - `transcript get last` prints the JSON transcript from the running serve process
- **Research references**:
  - `.copilot-tracking/research/20260722-transcript-cli-research.md` (Lines 168–175) — client-side get sketch
- **Dependencies**:
  - Task 3.2 complete (serve must have HTTP endpoint running)

### Task 5.2: Implement `api_url(what: &str)`

Resolve the newest session, print `http://127.0.0.1:{port}/transcript` to stdout.

```rust
fn api_url(what: &str) -> Result<(), Box<dyn std::error::Error>> {
    let session = newest_session()?;
    println!("http://127.0.0.1:{}/transcript", session.port);
    Ok(())
}
```

- **Files**:
  - `transcript-merger/src/main.rs` — add `api_url()` function
- **Success**:
  - `transcript api last` prints the HTTP URL for the running session
- **Research references**:
  - `.copilot-tracking/research/20260722-transcript-cli-research.md` (Lines 66–74) — `api` subcommand behavior
- **Dependencies**:
  - Task 5.1 complete (`newest_session()` helper reused)

### Task 5.3: Implement `mcp_stub(what, http)` stubs

Add a stub that prints a clear "not yet implemented" message and returns `Ok(())`. This satisfies the CLI contract without crashing on the `Mcp` variant.

```rust
fn mcp_stub(what: Option<String>, http: bool) -> Result<(), Box<dyn std::error::Error>> {
    if http {
        eprintln!("mcp --http: HTTP MCP server not yet implemented");
    } else {
        eprintln!("mcp {}: stdio MCP not yet implemented", what.as_deref().unwrap_or(""));
    }
    Ok(())
}
```

- **Files**:
  - `transcript-merger/src/main.rs` — add `mcp_stub()` function
- **Success**:
  - `transcript mcp last` and `transcript mcp --http` exit cleanly with a message
- **Research references**:
  - `.copilot-tracking/research/20260722-transcript-cli-research.md` (Line 33) — `rmcp` crate noted for future MCP implementation
- **Dependencies**:
  - Task 2.1 complete (`Mcp` variant defined in enum)

## Dependencies

- `clap 4` (derive feature)
- `axum 0.8`
- `serde 1` (derive feature)
- `serde_json 1`
- `reqwest 0.12` (json feature)
- `chrono 0.4` (already present)
- `tokio 1` with `full` features (already present)

## Success Criteria

- `transcript --help` shows all subcommands
- `transcript serve` starts, writes session file, streams merged transcript to stdout, serves HTTP on port 11435
- `transcript ls` and `transcript ps` list running sessions
- `transcript stop` terminates the running serve process
- `transcript get last` retrieves and prints transcript from running session
- `transcript api last` prints the HTTP endpoint URL
- `transcript mcp last` and `transcript mcp --http` print "not yet implemented" without crashing
- No transcript data written to disk; only session connection info persisted
- Binary name is `transcript` (not `transcript-merger`)
