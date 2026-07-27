# Live Audio Transcription (Rust)

Real-time live audio transcription workspace. Captures microphone and system speaker audio concurrently, transcribes both streams, and merges the output into a timestamped transcript.


## Architecture

```text
mic-transcription    ──┐
                       ├─► transcript-merger ──► stdout 
speaker-transcription ─┘
```

`transcript-merger` spawns both child processes, reads their stdout concurrently using async tasks, timestamps each line, and prints the merged stream to stdout. On exit (Ctrl+C or natural termination).


## Workspace Members

| Crate | Description |
|-------|-------------|
| [mic-transcription](mic-transcription/) | Real-time microphone capture and transcription using CPAL and Foundry Local SDK. |
| [speaker-transcription](speaker-transcription/) | Real-time system speaker (loopback) capture and transcription using CPAL and Foundry Local SDK. |
| [transcript-merger](transcript-merger/) | Orchestrator that spawns both transcribers, merges their output with timestamps, and saves `transcript.md`. |

## Prerequisites

- [Rust](https://www.rust-lang.org/) 1.70.0 or later
- [Foundry Local](https://learn.microsoft.com/azure/foundry-local/) installed and running

## Quick Start

Build all crates:

```bash
cargo build --release
```

Run the full pipeline (mic + speaker + merger):

```bash
cargo run -p transcript-merger
```

Press **Ctrl+C** to stop. The merged transcript is written to `transcript.md`.

## Running Individual Components

```bash
# Microphone transcription only
cargo run -p mic-transcription

# Speaker (system audio) transcription only
cargo run -p speaker-transcription
```
