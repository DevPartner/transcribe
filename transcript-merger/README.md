# Transcript Merger

Orchestrates real-time transcription from two concurrent audio streams — microphone and system speaker — and merges their output into a single chronological transcript.

## Architecture

```text
mic-transcription    ──┐
                       ├─► transcript-merger ──► stdout 
speaker-transcription ─┘
```

`transcript-merger` spawns both child processes, reads their stdout concurrently using async tasks, timestamps each line, and prints the merged stream to stdout. On exit (Ctrl+C or natural termination), it writes all accumulated entries to `transcript.md`.

## Dependencies

| Crate                                     | Purpose                                                        |
| ----------------------------------------- | -------------------------------------------------------------- |
| [tokio](https://crates.io/crates/tokio)   | Async runtime, process spawning, and concurrent stdout readers |
| [chrono](https://crates.io/crates/chrono) | Local timestamps (`HH:MM:SS.mmm`)                              |

## Run

Build the two child binaries first, then run the merger from the workspace root:

```bash
cargo build -p mic-transcription
cargo build -p speaker-transcription
cargo run -p transcript-merger
```

Press **Ctrl+C** to stop. 

## Console Output

```text
====================================

[INFO] Starting mic-transcription.exe
[INFO] Starting speaker-transcription.exe

[INFO] Mic stream connected
[INFO] Speaker stream connected

------------------------------------

[INFO] REST  GET http://0.0.0.0:3000/transcript
[INFO] WS    ws://0.0.0.0:3000/transcript/ws

[12:01:03.421] 🎤 MIC
Hello everyone, let's start the meeting.

[12:01:05.102] 🔊 SPEAKER
Good morning. Can you hear me?

[12:01:06.887] 🎤 MIC
Yes, your audio is clear.

------------------------------------
```
