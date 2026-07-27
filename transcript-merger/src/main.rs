use axum::{
    extract::{State, WebSocketUpgrade},
    extract::ws::{Message, WebSocket},
    response::Response,
    routing::get,
    Json, Router,
};
use chrono::Local;
use serde::Serialize;
use std::io::Write;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpListener;
use tokio::process::Command;
use tokio::signal;
use tokio::sync::broadcast;

#[derive(Serialize, Clone)]
struct Entry {
    timestamp: String,
    source: &'static str,
    text: String,
}

struct PrintState {
    current_source: Option<&'static str>,
    current_minute: Option<String>,
}

fn print_fragment(st: &mut PrintState, source: &'static str, emoji: &str, ts: &str, text: &str) {
    let minute = &ts[..5]; // "HH:MM" — ASCII, safe slice
    if st.current_minute.as_deref() != Some(minute) {
        if st.current_source.is_some() {
            println!();
            println!();
        }
        st.current_minute = Some(minute.to_string());
        st.current_source = None;
    }

    if st.current_source == Some(source) {
        print!(" {text}");
    } else {
        if st.current_source.is_some() {
            println!();
            println!();
        }
        println!("[{ts}] {emoji} {source}");
        print!("{text}");
        st.current_source = Some(source);
    }
    let _ = std::io::stdout().flush();
}

// ── Shared state ──────────────────────────────────────────────────────────────

#[derive(Clone)]
struct AppState {
    transcript: Arc<Mutex<Vec<Entry>>>,
    tx: broadcast::Sender<String>,
}

// ── REST handler ──────────────────────────────────────────────────────────────

async fn get_transcript(State(state): State<AppState>) -> Json<Vec<Entry>> {
    Json(state.transcript.lock().unwrap().clone())
}

// ── WebSocket handler ─────────────────────────────────────────────────────────

async fn ws_transcript(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    // Subscribe before reading history so we don't miss entries written between
    // the two steps.
    let mut rx = state.tx.subscribe();

    // Send full history to the new client.
    let history = state.transcript.lock().unwrap().clone();
    for entry in history {
        let msg = format!("[{}] {}: {}", entry.timestamp, entry.source, entry.text);
        if socket.send(Message::Text(msg.into())).await.is_err() {
            return;
        }
    }

    // Stream new entries as they arrive.
    loop {
        match rx.recv().await {
            Ok(line) => {
                if socket.send(Message::Text(line.into())).await.is_err() {
                    break;
                }
            }
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }
}

// ── Main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    match dotenvy::dotenv() {
        Ok(path) => eprintln!(".env loaded from {:?}", path),
        Err(e) => eprintln!(".env not loaded: {e}  (cwd={:?})", std::env::current_dir().unwrap()),
    }
    println!("====================================");
    println!(" Transcript Merger");
    println!("====================================");
    println!();
    println!("[INFO] Starting mic-transcription.exe");
    println!("[INFO] Starting speaker-transcription.exe");

    let mut mic_child = Command::new("mic-transcription.exe")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    let mut speaker_child = Command::new("speaker-transcription.exe")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    let mic_out = mic_child.stdout.take().unwrap();
    let speaker_out = speaker_child.stdout.take().unwrap();

    println!();
    println!("[INFO] Mic stream connected");
    println!("[INFO] Speaker stream connected");
    println!();
    println!("------------------------------------");
    println!();

    let (tx, _) = broadcast::channel::<String>(256);
    let state = AppState {
        transcript: Arc::new(Mutex::new(Vec::new())),
        tx: tx.clone(),
    };
    let print_state: Arc<Mutex<PrintState>> = Arc::new(Mutex::new(PrintState { current_source: None, current_minute: None }));

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3001);
    let app = Router::new()
        .route("/transcript", get(get_transcript))
        .route("/transcript/ws", get(ws_transcript))
        .with_state(state.clone());
    let listener = TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    println!("[INFO] REST  GET http://0.0.0.0:{port}/transcript");
    println!("[INFO] WS    ws://0.0.0.0:{port}/transcript/ws");
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let s1 = state.clone();
    let ps1 = print_state.clone();
    let mic_task = tokio::spawn(async move {
        let mut lines = BufReader::new(mic_out).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let text = line.trim().to_string();
            if text.is_empty() {
                continue;
            }
            let ts = Local::now().format("%H:%M:%S%.3f").to_string();
            print_fragment(&mut ps1.lock().unwrap(), "MIC", "\u{1f3a4}", &ts, &text);
            let entry = Entry { timestamp: ts.clone(), source: "MIC", text: text.clone() };
            s1.transcript.lock().unwrap().push(entry);
            let _ = s1.tx.send(format!("[{ts}] MIC: {text}"));
        }
    });

    let s2 = state.clone();
    let ps2 = print_state.clone();
    let speaker_task = tokio::spawn(async move {
        let mut lines = BufReader::new(speaker_out).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let text = line.trim().to_string();
            if text.is_empty() {
                continue;
            }
            let ts = Local::now().format("%H:%M:%S%.3f").to_string();
            print_fragment(&mut ps2.lock().unwrap(), "SPEAKER", "\u{1f50a}", &ts, &text);
            let entry = Entry { timestamp: ts.clone(), source: "SPEAKER", text: text.clone() };
            s2.transcript.lock().unwrap().push(entry);
            let _ = s2.tx.send(format!("[{ts}] SPEAKER: {text}"));
        }
    });

    tokio::select! {
        _ = async { let _ = tokio::join!(mic_task, speaker_task); } => {}
        _ = signal::ctrl_c() => {
            let _ = mic_child.kill().await;
            let _ = speaker_child.kill().await;
        }
    }

    if print_state.lock().unwrap().current_source.is_some() {
        println!();
    }

    println!();
    println!("------------------------------------");
    println!();
    Ok(())
}
