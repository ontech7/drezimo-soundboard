//! Voicemod's own bridge between its window and its engine: a WebSocket on this computer, on a port that changes at
//! each start of Voicemod, outside the Control API and undocumented. The app uses it only to stop a single sound, which
//! the Control API cannot do; while it is not found, a pad falls back to stopping every sound (docs/project.md,
//! section 8).

use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::{self, Message};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

const CONTROL_API_PORT: u16 = 59129;
const SEARCH_EVERY: Duration = Duration::from_secs(5);
/// For the bridge to greet the app, and to answer a stop: it answers within about 150 ms
const ANSWER_TIME: Duration = Duration::from_secs(1);
/// A sound Voicemod is still loading cannot be stopped yet (a first play takes up to 0.7 s): it is stopped once it
/// starts, if it does by then
const STOP_ONCE_STARTED: Duration = Duration::from_secs(3);

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

enum Job {
    Stop {
        sound: String,
        done: oneshot::Sender<()>,
    },
    /// A key-down went to the sound: Voicemod loads it, and a stop that came too early for it no longer stands
    Pressed(String),
}

#[derive(Clone)]
pub struct Bridge {
    jobs: mpsc::UnboundedSender<Job>,
    connected: Arc<AtomicBool>,
}

impl Bridge {
    /// Looks for the bridge in the background, again whenever it goes away; `on_found` tells whether it is there.
    pub fn start(on_found: impl Fn(bool) + Send + 'static) -> Self {
        let (jobs, received) = mpsc::unbounded_channel();
        let connected = Arc::new(AtomicBool::new(false));
        tauri::async_runtime::spawn(keep_connected(received, connected.clone(), on_found));
        Self { jobs, connected }
    }

    /// Stops a single sound, once Voicemod has answered, so that a press sent next comes after it. False when the
    /// bridge is not there.
    pub async fn stop_sound(&self, sound: &str) -> bool {
        if !self.connected.load(Ordering::Relaxed) {
            return false;
        }
        let (done, answered) = oneshot::channel();
        let job = Job::Stop {
            sound: sound.to_string(),
            done,
        };
        if self.jobs.send(job).is_err() {
            return false;
        }
        match timeout(ANSWER_TIME, answered).await {
            Ok(answer) => answer.is_ok(),
            // Sent, and answered late, unless the bridge went away meanwhile
            Err(_) => self.connected.load(Ordering::Relaxed),
        }
    }

    pub fn pressed(&self, sound: &str) {
        if self.connected.load(Ordering::Relaxed) {
            let _ = self.jobs.send(Job::Pressed(sound.to_string()));
        }
    }
}

async fn keep_connected(mut jobs: mpsc::UnboundedReceiver<Job>, connected: Arc<AtomicBool>, on_found: impl Fn(bool)) {
    // Until the link to Voicemod ends, as in a build without a client key
    while !jobs.is_closed() {
        if let Some(socket) = find().await {
            // Asked while it was away: stale
            while jobs.try_recv().is_ok() {}
            connected.store(true, Ordering::Relaxed);
            on_found(true);
            let result = serve(socket, &mut jobs).await;
            connected.store(false, Ordering::Relaxed);
            // Their callers fall back to stopping every sound
            while jobs.try_recv().is_ok() {}
            on_found(false);
            match result {
                Ok(()) => return,
                Err(error) => crate::errors::log(format!("lost Voicemod's bridge: {error}")),
            }
        }
        tokio::time::sleep(SEARCH_EVERY).await;
    }
}

/// A stop sent, waiting for its answer
struct Stopping {
    sound: String,
    /// None for a stop sent again once its sound started
    done: Option<oneshot::Sender<()>>,
    sent: Instant,
}

/// Until the jobs end, or the bridge fails.
async fn serve(mut socket: Socket, jobs: &mut mpsc::UnboundedReceiver<Job>) -> Result<(), tungstenite::Error> {
    let mut stops = Stops::default();
    loop {
        tokio::select! {
            message = socket.next() => match message {
                Some(Ok(Message::Text(text))) => {
                    if let Some(sound) = stops.read(&text) {
                        stops.send(&mut socket, sound, None).await?;
                    }
                }
                Some(Ok(_)) => {}
                Some(Err(error)) => return Err(error),
                None => return Err(tungstenite::Error::ConnectionClosed),
            },
            job = jobs.recv() => match job {
                Some(Job::Stop { sound, done }) => stops.send(&mut socket, sound, Some(done)).await?,
                Some(Job::Pressed(sound)) => stops.pressed(sound),
                None => return Ok(()),
            },
        }
    }
}

#[derive(Default)]
struct Stops {
    next_id: u64,
    /// By the id of the message sent
    waiting: HashMap<String, Stopping>,
    /// Pressed and not playing yet: Voicemod is loading them
    loading: HashSet<String>,
    /// Stops that came while their sound was loading, and until when they stand
    once_started: HashMap<String, Instant>,
}

impl Stops {
    async fn send(
        &mut self,
        socket: &mut Socket,
        sound: String,
        done: Option<oneshot::Sender<()>>,
    ) -> Result<(), tungstenite::Error> {
        self.next_id += 1;
        let message_id = format!("drezimo-{}", self.next_id);
        let request = json!({
            "bridgeData": {
                "data": { "id": message_id, "object": { "id": sound }, "type": "stopSound" },
                "type": "appMessage",
            }
        });
        socket.send(Message::text(request.to_string())).await?;
        // Those Voicemod never answered
        self.waiting
            .retain(|_, stopping| stopping.sent.elapsed() < ANSWER_TIME * 5);
        let stopping = Stopping {
            sound,
            done,
            sent: Instant::now(),
        };
        self.waiting.insert(message_id, stopping);
        Ok(())
    }

    fn pressed(&mut self, sound: String) {
        self.once_started.remove(&sound);
        self.loading.insert(sound);
    }

    /// Follows what the bridge tells; returns a sound to stop again, now that it started.
    fn read(&mut self, text: &str) -> Option<String> {
        match message(text)? {
            BridgeMessage::StopAnswer { message_id, stopped } => {
                let stopping = self.waiting.remove(&message_id)?;
                if !stopped && self.loading.contains(&stopping.sound) {
                    let until = Instant::now() + STOP_ONCE_STARTED;
                    self.once_started.insert(stopping.sound, until);
                }
                if let Some(done) = stopping.done {
                    let _ = done.send(());
                }
                None
            }
            BridgeMessage::SoundState { sound, state } => {
                if !matches!(state.as_str(), "created" | "ready") {
                    self.loading.remove(&sound);
                }
                if state != "playing" {
                    return None;
                }
                let until = self.once_started.remove(&sound)?;
                (Instant::now() < until).then_some(sound)
            }
        }
    }
}

#[derive(Debug, PartialEq)]
enum BridgeMessage {
    /// `stopped` false when Voicemod could not, as for a sound not playing, or not yet
    StopAnswer {
        message_id: String,
        stopped: bool,
    },
    SoundState {
        sound: String,
        state: String,
    },
}

fn message(text: &str) -> Option<BridgeMessage> {
    let data = data(text)?;
    let object = &data["object"];
    match data["type"].as_str()? {
        "stopSound" => Some(BridgeMessage::StopAnswer {
            message_id: object["frontendMessageId"].as_str()?.to_string(),
            stopped: data["error"].is_null(),
        }),
        "soundStateChanged" => Some(BridgeMessage::SoundState {
            sound: object["id"].as_str()?.to_string(),
            state: object["state"].as_str()?.to_string(),
        }),
        _ => None,
    }
}

/// What a message of the bridge carries
fn data(text: &str) -> Option<Value> {
    let mut message: Value = serde_json::from_str(text).ok()?;
    Some(message.get_mut("bridgeData")?.get_mut("data")?.take())
}

/// The bridge among the ports Voicemod listens on, if any: it greets every client with the state of Voicemod's window.
async fn find() -> Option<Socket> {
    for port in bridge_ports(&listeners(), is_voicemod) {
        let url = format!("ws://127.0.0.1:{port}/");
        let Ok(Ok((mut socket, _))) = timeout(ANSWER_TIME, tokio_tungstenite::connect_async(url)).await else {
            continue;
        };
        if let Ok(Some(Ok(Message::Text(text)))) = timeout(ANSWER_TIME, socket.next()).await
            && data(&text).is_some()
        {
            return Some(socket);
        }
    }
    None
}

/// A socket listening for connections
struct Listener {
    port: u16,
    loopback: bool,
    process: u32,
}

/// Voicemod's ports on this computer only, except the Control API's.
fn bridge_ports(listeners: &[Listener], is_voicemod: impl Fn(u32) -> bool) -> Vec<u16> {
    listeners
        .iter()
        .filter(|listener| listener.loopback && listener.port != CONTROL_API_PORT && is_voicemod(listener.process))
        .map(|listener| listener.port)
        .collect()
}

#[cfg(windows)]
fn listeners() -> Vec<Listener> {
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        GetExtendedTcpTable, MIB_TCPROW_OWNER_PID, MIB_TCPTABLE_OWNER_PID, TCP_TABLE_OWNER_PID_LISTENER,
    };
    use windows_sys::Win32::Networking::WinSock::AF_INET;

    let mut size = 0u32;
    let mut table: Vec<u32> = Vec::new();
    // Sized by a first call; the table may grow in between
    for _ in 0..3 {
        table.resize((size as usize).div_ceil(4), 0);
        let buffer = if table.is_empty() {
            std::ptr::null_mut()
        } else {
            table.as_mut_ptr().cast()
        };
        let result =
            unsafe { GetExtendedTcpTable(buffer, &mut size, 0, AF_INET.into(), TCP_TABLE_OWNER_PID_LISTENER, 0) };
        if result != 0 || table.is_empty() {
            continue;
        }
        let header = table.as_ptr().cast::<MIB_TCPTABLE_OWNER_PID>();
        let rows = unsafe {
            std::slice::from_raw_parts(
                std::ptr::addr_of!((*header).table).cast::<MIB_TCPROW_OWNER_PID>(),
                (*header).dwNumEntries as usize,
            )
        };
        return rows
            .iter()
            .map(|row| Listener {
                // In network byte order, as the address
                port: u16::from_be(row.dwLocalPort as u16),
                loopback: u32::from_be(row.dwLocalAddr) == u32::from(std::net::Ipv4Addr::LOCALHOST),
                process: row.dwOwningPid,
            })
            .collect();
    }
    Vec::new()
}

#[cfg(windows)]
fn is_voicemod(process: u32) -> bool {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
    };

    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process) };
    if handle.is_null() {
        return false;
    }
    let mut path = [0u16; 1024];
    let mut length = path.len() as u32;
    let read = unsafe { QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, path.as_mut_ptr(), &mut length) };
    unsafe { CloseHandle(handle) };
    read != 0
        && String::from_utf16_lossy(&path[..length as usize])
            .to_lowercase()
            .ends_with("\\voicemod.exe")
}

/// Not looked for yet on macOS: the pads stop every sound there
#[cfg(not(windows))]
fn listeners() -> Vec<Listener> {
    Vec::new()
}

#[cfg(not(windows))]
fn is_voicemod(_process: u32) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    // Shapes recorded from Voicemod 3.17.45 on Windows

    fn answer(message_id: &str, failed: bool) -> String {
        let error = if failed {
            r#""error":{"code":"FailedToStopSound","description":"services.audio.handler.stopsound.error.1"},"#
        } else {
            ""
        };
        format!(
            r#"{{"bridgeData":{{"data":{{{error}"id":"","object":{{"frontendMessageId":"{message_id}","id":"s1"}},
            "type":"stopSound"}},"type":"appMessage"}}}}"#
        )
    }

    fn state(sound: &str, state: &str) -> String {
        format!(
            r#"{{"bridgeData":{{"data":{{"id":"","object":{{"id":"{sound}","state":"{state}"}},
            "type":"soundStateChanged"}},"type":"appMessage"}}}}"#
        )
    }

    fn waiting(stops: &mut Stops, message_id: &str, sound: &str) -> oneshot::Receiver<()> {
        let (done, answered) = oneshot::channel();
        let stopping = Stopping {
            sound: sound.into(),
            done: Some(done),
            sent: Instant::now(),
        };
        stops.waiting.insert(message_id.into(), stopping);
        answered
    }

    #[test]
    fn reads_the_bridges_messages() {
        assert_eq!(
            message(&answer("drezimo-1", false)),
            Some(BridgeMessage::StopAnswer {
                message_id: "drezimo-1".into(),
                stopped: true
            })
        );
        assert_eq!(
            message(&answer("drezimo-2", true)),
            Some(BridgeMessage::StopAnswer {
                message_id: "drezimo-2".into(),
                stopped: false
            })
        );
        assert_eq!(
            message(&state("s1", "playing")),
            Some(BridgeMessage::SoundState {
                sound: "s1".into(),
                state: "playing".into()
            })
        );
        let window = r#"{"bridgeData":{"data":{"id":"","object":{"isMaximized":false},
            "type":"WindowsMaximizeStatusChanged"},"type":"appMessage"}}"#;
        assert_eq!(message(window), None);
        assert!(data(window).is_some(), "the greeting that tells the bridge");
        assert!(data(r#"{"actionType": "playMeme"}"#).is_none(), "the Control API's");
    }

    #[test]
    fn a_stop_is_done_when_its_own_answer_comes() {
        let mut stops = Stops::default();
        let mut first = waiting(&mut stops, "drezimo-1", "a");
        let mut second = waiting(&mut stops, "drezimo-2", "b");
        assert_eq!(stops.read(&answer("drezimo-2", false)), None);
        assert!(second.try_recv().is_ok());
        assert!(first.try_recv().is_err(), "still waiting");
    }

    #[test]
    fn a_sound_still_loading_is_stopped_once_it_starts() {
        let mut stops = Stops::default();
        stops.pressed("a".into());
        stops.pressed("b".into());
        waiting(&mut stops, "drezimo-1", "a");
        waiting(&mut stops, "drezimo-2", "b");
        stops.read(&answer("drezimo-1", true));
        stops.read(&answer("drezimo-2", true));
        assert_eq!(stops.read(&state("a", "created")), None);
        assert_eq!(stops.read(&state("a", "playing")).as_deref(), Some("a"));
        assert_eq!(stops.read(&state("b", "playing")).as_deref(), Some("b"));
        assert_eq!(stops.read(&state("a", "playing")), None, "once");

        // Pressed again meanwhile: that press is what the user wants
        stops.pressed("c".into());
        waiting(&mut stops, "drezimo-3", "c");
        stops.read(&answer("drezimo-3", true));
        stops.pressed("c".into());
        assert_eq!(stops.read(&state("c", "playing")), None);
    }

    #[test]
    fn a_sound_that_already_ended_is_not_stopped_later() {
        let mut stops = Stops::default();
        stops.pressed("a".into());
        stops.read(&state("a", "playing"));
        stops.read(&state("a", "finished"));
        waiting(&mut stops, "drezimo-1", "a");
        stops.read(&answer("drezimo-1", true));
        // Started again from Voicemod's own window
        assert_eq!(stops.read(&state("a", "playing")), None);
    }

    #[test]
    fn looks_only_at_voicemods_ports_on_this_computer() {
        let listener = |port, loopback, process| Listener {
            port,
            loopback,
            process,
        };
        let listeners = [
            listener(59129, true, 7),
            listener(50658, true, 7),
            listener(39998, false, 7),
            listener(50700, true, 8),
        ];
        assert_eq!(bridge_ports(&listeners, |process| process == 7), [50658]);
    }
}
