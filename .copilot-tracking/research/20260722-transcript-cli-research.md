<!-- markdownlint-disable-file -->

# Task Research Notes: Transcript Service CLI

## Research Executed

### File Analysis

- `transcript-merger/src/main.rs`
  - Single binary, no CLI parsing, hardcodes `mic-transcription.exe` and `speaker-transcription.exe` as child processes
  - Merges stdout from both into a timestamped chronological stream via `Arc<Mutex<Vec<Entry>>>`
  - Ctrl+C kills both children; commented-out summary block never runs
- `transcript-merger/Cargo.toml`
  - Package name `transcript-merger`, no `[[bin]]` override, no `clap`/`axum` dependency
- `mic-transcription/src/main.rs`
  - Uses `foundry_local_sdk` + `cpal`; model alias hardcoded as const; prints raw fragments to stdout
- `speaker-transcription/src/main.rs`
  - Same as mic but opens WASAPI loopback device
- `Cargo.toml` (workspace)
  - Members: `transcript-merger`, `mic-transcription`, `speaker-transcription`, `67-assistant`

### Code Search Results

- No `clap`, `axum`, or MCP crate usage anywhere in the workspace
- No session/PID state files exist
- No IPC between binaries

### External Research

- `clap` 4 derive: standard Rust CLI framework; subcommand enums, multi-value args
- `axum` 0.8: Tokio-native HTTP framework; integrates with `tokio::spawn` directly alongside existing async tasks
- Ollama CLI pattern: `serve` starts a long-running process that embeds an HTTP server; other subcommands (`ls`, `ps`, `stop`, `show`) connect to it via HTTP
- `rmcp` crate: official MCP Rust SDK for stdio and HTTP (SSE) transports — usable for `mcp` subcommand

## Key Discoveries

### Project Structure

```text
rust/
├── transcript-merger/   ← binary to be refactored into CLI
├── mic-transcription/   ← child process: mic → ASR → stdout
├── speaker-transcription/ ← child process: speaker loopback → ASR → stdout
└── 67-assistant/
```

### Current Binary Behavior

- `transcript-merger` spawns both children unconditionally; keeps entries in `Arc<Mutex<Vec<Entry>>>`
- "channel" = which children to spawn: `mic`, `speaker`, or both. Default - both. 
- "model" = Foundry Local model alias (currently a compile-time const in each child binary; passed via `TRANSCRIPT_MODEL` env var in the new design)
- Transcript is already entirely in-memory; no persistence was ever completed

### Architecture: In-Memory Transcript with Embedded HTTP

The `serve` process is the **only** process that holds transcript state. It never writes transcript data to disk. Other subcommands and external tools access it through the HTTP API the `serve` process embeds.

```text
transcript serve
  ├── spawns mic-transcription.exe   → reads stdout → Arc<Mutex<Vec<Entry>>>
  ├── spawns speaker-transcription.exe → reads stdout → same Arc<Mutex<Vec<Entry>>>
  ├── streams merged output to its own stdout (live, same as today)
  └── axum HTTP server at localhost:{port}
        GET /transcript   → current in-memory transcript as JSON or markdown
        GET /health       → { pid, channels, model, entry_count }

transcript get last        → reads session file → HTTP GET /transcript → prints to stdout
transcript api last        → prints the API URL for the newest running session
transcript mcp last        → reads session file → HTTP GET /transcript → re-emits via MCP stdio
transcript mcp --http      → starts an HTTP MCP server proxying /transcript from running serve
transcript ls              → reads all session files, prints table
transcript ps              → same as ls, compact
transcript stop            → reads session file, TerminateProcess(pid)
```

**Transcript never written to a file** — it lives only in the `serve` process's heap until the process exits.

### Session File Design (connection info only — no transcript data)

`serve` writes one JSON file to `%APPDATA%\transcript\sessions\<session-id>.json` on start and removes it on exit. It contains only connection info:

```json
{
  "id": "20260722-100532",
  "pid": 12345,
  "port": 11435,
  "channels": ["mic", "speaker"],
  "model": "nemotron-speech-streaming-en-0.6b",
  "started_at": "2026-07-22T10:05:32Z"
}
```

No `transcript_path` field — the transcript is never on disk.

### Complete Examples

```rust
// transcript-merger/Cargo.toml additions
[dependencies]
clap    = { version = "4", features = ["derive"] }
axum    = "0.8"
serde   = { version = "1", features = ["derive"] }
serde_json = "1"
chrono  = "0.4"   # already present
tokio   = { version = "1", features = ["full"] }  # already present

[[bin]]
name = "transcript"
path = "src/main.rs"
```

```rust
// CLI shape (clap derive)
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
    Stop {
        #[arg(long)]
        channel: Option<String>,
    },
    Get { what: String },   // "last" → HTTP GET /transcript from newest session
    Api { what: String },   // "last" → print API URL
    Mcp {
        what: Option<String>,          // "last" → MCP stdio
        #[arg(long)] http: bool,       // start HTTP MCP server
        #[arg(long, requires = "http")] daemon: bool,
    },
}
```

```rust
// serve handler sketch — axum router alongside existing tokio tasks
async fn serve(channels: Vec<String>, model: String, port: u16) {
    let transcript: Arc<Mutex<Vec<Entry>>> = Arc::new(Mutex::new(Vec::new()));

    // spawn mic/speaker child tasks (existing logic) ...

    // axum router — shares the same Arc
    let t = transcript.clone();
    let app = axum::Router::new()
        .route("/transcript", axum::routing::get(move || {
            let entries = t.lock().unwrap();
            // format as markdown or JSON
        }))
        .route("/health", axum::routing::get(|| async { "ok" }));

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

```rust
// transcript get last — client side
async fn get_last() {
    let session = read_newest_session(); // reads %APPDATA%\transcript\sessions\
    let url = format!("http://127.0.0.1:{}/transcript", session.port);
    let body = reqwest::get(&url).await?.text().await?;
    println!("{body}");
}
```

### Technical Requirements

1. Binary renamed to `transcript` via `[[bin]]` in Cargo.toml
2. `clap 4`, `axum 0.8`, `serde`/`serde_json`, `reqwest` added to dependencies
3. `serve` embeds axum HTTP server; transcript lives exclusively in `Arc<Mutex<Vec<Entry>>>`
4. Session file stores only `{ id, pid, port, channels, model, started_at }` — no transcript path
5. `get last` / `api last` / `mcp last` all resolve port from session file and call `GET /transcript`
6. `ls` / `ps` read all session JSON files from `%APPDATA%\transcript\sessions\`
7. `stop` reads PID from session file and terminates the process
8. Model passed to child processes via `TRANSCRIPT_MODEL` env var (no changes needed to child binaries)
9. `mcp --http` starts an HTTP MCP server that proxies `/transcript` from the running session
10. No transcript data is ever written to disk by the service

## Recommended Approach

**`serve` embeds an axum HTTP server; transcript lives exclusively in process memory**

- `serve` is the single source of truth for transcript data
- All other subcommands are thin clients that connect to `serve`'s local HTTP API
- Session file is purely a service-discovery record (PID + port), not a data store
- `get last` = HTTP client call to running session
- `api last` = print the URL of the running session's `/transcript` endpoint
- `mcp` = MCP stdio or HTTP proxy over the same `/transcript` endpoint
- No file I/O for transcript data; process exit = data gone (intentional)

## Implementation Guidance

- **Objectives**: Transform `transcript-merger` into a `transcript` binary with Ollama-style subcommands, in-memory transcript state, and a local HTTP API for data access
- **Key Tasks**:
  1. Add `[[bin]] name = "transcript"` to `Cargo.toml`
  2. Add `clap 4`, `axum 0.8`, `serde`/`serde_json`, `reqwest` to dependencies
  3. Refactor `main.rs`: parse `Cli`, dispatch to handlers
  4. `serve` handler: current merge logic + `axum::Router` with `/transcript` and `/health` endpoints sharing same `Arc<Mutex<Vec<Entry>>>`
  5. Write session JSON (pid + port only) on `serve` start; delete on exit (use `scopeguard` or `Drop`)
  6. `ls`/`ps`: read session JSON files, print table
  7. `stop`: read PID, call `TerminateProcess` on Windows via `windows` crate or `taskkill /PID`
  8. `get last`: find newest session file, `reqwest::get` → print
  9. `api last`: find newest session file, print URL
  10. `mcp` stubs: `what=Some("last")` → stdio MCP stub; `--http` → HTTP MCP stub
- **Dependencies**: `clap 4`, `axum 0.8`, `serde`, `serde_json`, `reqwest`, `chrono` (already present), `tokio` (already present)
- **Success Criteria**: `transcript serve`, `transcript ls`, `transcript stop`, `transcript get last`, `transcript api last` all work; no transcript data written to disk; binary is named `transcript`
