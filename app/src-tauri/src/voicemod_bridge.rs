//! Voicemod's own bridge between its window and its engine: a WebSocket on this computer, on a port that changes at
//! each start of Voicemod, outside the Control API and undocumented. The app uses it for what the Control API cannot
//! do: stop a single sound, read and set the mixer's and a playing sound's volumes, and change a sound's own options in
//! Voicemod (its mode, its saved volume). While it is not found, a pad falls back to stopping every sound and the
//! volumes do nothing (docs/project.md, section 8).

use crate::mixer::Channel;
use futures_util::{SinkExt, StreamExt};
use serde_json::{Map, Value, json};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::{self, Message};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

/// Voicemod's fixed ports, never the bridge: 59129 is the Control API's, and a WebSocket's handshake on 39998 made
/// Voicemod quit (macOS)
const NOT_THE_BRIDGE: [u16; 3] = [59129, 39998, 39999];
const SEARCH_EVERY: Duration = Duration::from_secs(5);
/// A port of Voicemod's that does not answer as the bridge is tried again for a while, in case Voicemod is still
/// starting, then left alone while its process runs: an update may have changed the bridge
const GIVE_UP_AFTER: Duration = Duration::from_secs(120);
/// For the bridge to greet the app, and to answer a stop: it answers within about 150 ms
const ANSWER_TIME: Duration = Duration::from_secs(1);
/// A sound Voicemod is still loading cannot be stopped yet (a first play takes up to 0.7 s): it is stopped once it
/// starts, if it does by then
const STOP_ONCE_STARTED: Duration = Duration::from_secs(3);
/// The sounds' options read from Voicemod are trusted for a while: Voicemod's window tells its own changes, but a
/// change made while the app was away would be undone by an older copy
const OPTIONS_FRESH: Duration = Duration::from_secs(600);
/// A change waiting for the options to be read is dropped after that: Voicemod's answers do not say whose read failed
const READ_WAIT: Duration = Duration::from_secs(10);

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;
/// Voicemod's ports, by process, that did not answer as the bridge, and since when
type Refused = HashMap<(u32, u16), Instant>;

enum Job {
    Stop {
        sound: String,
        done: oneshot::Sender<()>,
    },
    /// A key-down went to the sound: Voicemod loads it, and a stop that came too early for it no longer stands
    Pressed(String),
    SetGain {
        channel: Channel,
        gain: f32,
        enabled: bool,
    },
    SetSoundVolume {
        sound: String,
        volume: f32,
    },
    /// The sounds' saved volumes again, as after Voicemod's soundboards changed
    ReadSounds,
    /// Some of a sound's own options, kept by Voicemod: Voicemod takes them only all together
    SetSoundOptions {
        sound: String,
        changes: Map<String, Value>,
    },
}

#[derive(Clone)]
pub struct Bridge {
    jobs: mpsc::UnboundedSender<Job>,
    connected: Arc<AtomicBool>,
}

impl Bridge {
    /// Looks for the bridge in the background, again whenever it goes away, and tells what it learns.
    pub fn start(on_event: impl Fn(BridgeEvent) + Send + Sync + 'static) -> Self {
        let (jobs, received) = mpsc::unbounded_channel();
        let connected = Arc::new(AtomicBool::new(false));
        tauri::async_runtime::spawn(keep_connected(received, connected.clone(), on_event));
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
        self.send(Job::Pressed(sound.to_string()));
    }

    /// `enabled` false is the channel's mute in Voicemod's mixer, as its window sends it: Mute for me, for the
    /// soundboard for the user. Sent without it, Voicemod's window takes the channel as on.
    pub fn set_gain(&self, channel: Channel, gain: f32, enabled: bool) {
        self.send(Job::SetGain { channel, gain, enabled });
    }

    /// For the play under way only: Voicemod keeps the sound's saved volume
    pub fn set_sound_volume(&self, sound: &str, volume: f32) {
        self.send(Job::SetSoundVolume {
            sound: sound.to_string(),
            volume,
        });
    }

    pub fn read_sounds(&self) {
        self.send(Job::ReadSounds);
    }

    /// Changes some of a sound's options in Voicemod for good, as its own window does (`reproductionMode`, `volume`…)
    pub fn set_sound_options(&self, sound: &str, changes: Map<String, Value>) {
        self.send(Job::SetSoundOptions {
            sound: sound.to_string(),
            changes,
        });
    }

    /// Dropped while the bridge is not there
    fn send(&self, job: Job) {
        if self.connected.load(Ordering::Relaxed) {
            let _ = self.jobs.send(job);
        }
    }
}

async fn keep_connected(
    mut jobs: mpsc::UnboundedReceiver<Job>,
    connected: Arc<AtomicBool>,
    on_event: impl Fn(BridgeEvent),
) {
    let mut refused = Refused::new();
    // Until the link to Voicemod ends, as in a build without a client key
    while !jobs.is_closed() {
        if let Some(socket) = find(&mut refused).await {
            // Asked while it was away: stale
            while jobs.try_recv().is_ok() {}
            connected.store(true, Ordering::Relaxed);
            on_event(BridgeEvent::Found(true));
            let result = serve(socket, &mut jobs, &on_event).await;
            connected.store(false, Ordering::Relaxed);
            // Their callers fall back to stopping every sound
            while jobs.try_recv().is_ok() {}
            on_event(BridgeEvent::Found(false));
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
async fn serve(
    mut socket: Socket,
    jobs: &mut mpsc::UnboundedReceiver<Job>,
    on_event: &impl Fn(BridgeEvent),
) -> Result<(), tungstenite::Error> {
    let mut stops = Stops::default();
    let mut options = SoundOptions::default();
    // Values from before this connection may be gone, as after Voicemod restarted
    for channel in Channel::ALL {
        socket.send(gain_request(&stops.next_id(), channel)).await?;
    }
    loop {
        tokio::select! {
            incoming = socket.next() => match incoming {
                Some(Ok(Message::Text(text))) => match message(&text) {
                    Some(BridgeMessage::Gain { channel, gain, enabled, ours }) => {
                        on_event(BridgeEvent::Gain { channel, gain, enabled, ours });
                    }
                    Some(BridgeMessage::Resources(resources)) => {
                        let ready = options.read(&resources, Instant::now());
                        // With the changes that waited for them: the volumes the knob just saved among them
                        on_event(BridgeEvent::SoundVolumes(volumes(&options.known)));
                        for (sound, changed) in ready {
                            socket.send(options_request(&stops.next_id(), &sound, changed)).await?;
                        }
                    }
                    Some(BridgeMessage::OptionsSet { sound, options: set }) => options.set(sound, set),
                    Some(BridgeMessage::OptionsAnswer { changed }) => {
                        if changed {
                            on_event(BridgeEvent::SoundChanged);
                        } else {
                            options.refused();
                            crate::errors::log("Voicemod refused to change a sound's options");
                        }
                    }
                    Some(other) => {
                        if let Some(sound) = stops.read(other) {
                            stops.send(&mut socket, sound, None).await?;
                        }
                    }
                    None => {}
                },
                Some(Ok(_)) => {}
                Some(Err(error)) => return Err(error),
                None => return Err(tungstenite::Error::ConnectionClosed),
            },
            job = jobs.recv() => match job {
                Some(Job::Stop { sound, done }) => stops.send(&mut socket, sound, Some(done)).await?,
                Some(Job::Pressed(sound)) => stops.pressed(sound),
                Some(Job::SetGain { channel, gain, enabled }) => {
                    socket.send(set_gain_request(&stops.next_id(), channel, gain, enabled)).await?;
                }
                Some(Job::SetSoundVolume { sound, volume }) => {
                    socket.send(sound_volume_request(&stops.next_id(), &sound, volume)).await?;
                }
                Some(Job::ReadSounds) => socket.send(resources_request(&stops.next_id())).await?,
                Some(Job::SetSoundOptions { sound, changes }) => match options.change(sound, changes, Instant::now()) {
                    Change::Send(sound, changed) => {
                        socket.send(options_request(&stops.next_id(), &sound, changed)).await?;
                    }
                    Change::Read => socket.send(resources_request(&stops.next_id())).await?,
                    Change::Wait => {}
                },
                None => return Ok(()),
            },
        }
    }
}

/// Each sound's options as Voicemod last told them, and the changes waiting for them to be known
#[derive(Default)]
struct SoundOptions {
    known: HashMap<String, Map<String, Value>>,
    read: Option<Instant>,
    /// When a read was asked for the changes waiting
    reading: Option<Instant>,
    /// With when each was asked
    waiting: HashMap<String, (Map<String, Value>, Instant)>,
}

#[derive(Debug, PartialEq)]
enum Change {
    /// The sound's options, all of them, with the change
    Send(String, Map<String, Value>),
    /// The options must be read first
    Read,
    /// A read is on its way
    Wait,
}

impl SoundOptions {
    fn change(&mut self, sound: String, changes: Map<String, Value>, now: Instant) -> Change {
        let fresh = self.read.is_some_and(|read| now < read + OPTIONS_FRESH);
        if fresh && let Some(known) = self.known.get_mut(&sound) {
            known.extend(changes);
            return Change::Send(sound, known.clone());
        }
        let (waiting, asked) = self.waiting.entry(sound).or_insert_with(|| (Map::new(), now));
        waiting.extend(changes);
        *asked = now;
        if self.reading.is_some_and(|reading| now < reading + READ_WAIT) {
            return Change::Wait;
        }
        self.reading = Some(now);
        Change::Read
    }

    /// A read came: the changes waiting, now with all the sounds' options. One waiting too long, as after a failed
    /// read, is dropped rather than made at a moment nobody expects.
    fn read(&mut self, resources: &Value, now: Instant) -> Vec<(String, Map<String, Value>)> {
        self.known = all_options(resources);
        self.read = Some(now);
        self.reading = None;
        let waiting: Vec<_> = self.waiting.drain().collect();
        let mut ready = Vec::new();
        for (sound, (changes, asked)) in waiting {
            if now >= asked + READ_WAIT {
                crate::errors::log("cannot change a sound's options: Voicemod did not send them in time");
                continue;
            }
            match self.known.get_mut(&sound) {
                Some(known) => {
                    known.extend(changes);
                    ready.push((sound, known.clone()));
                }
                None => crate::errors::log("cannot change a sound Voicemod does not list"),
            }
        }
        ready
    }

    /// Voicemod refused a change, whose answer does not say which: every sound is read again before the next one
    fn refused(&mut self) {
        self.known.clear();
    }

    /// Voicemod's window changed a sound, or the app did
    fn set(&mut self, sound: String, options: Map<String, Value>) {
        if self.known.contains_key(&sound) {
            self.known.insert(sound, options);
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
        let message_id = self.next_id();
        socket.send(stop_request(&message_id, &sound)).await?;
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

    /// The id of the next message sent: Voicemod's answers carry it
    fn next_id(&mut self) -> String {
        self.next_id += 1;
        format!("{OUR_IDS}{}", self.next_id)
    }

    fn pressed(&mut self, sound: String) {
        self.once_started.remove(&sound);
        self.loading.insert(sound);
    }

    /// Follows what the bridge tells about sounds; returns a sound to stop again, now that it started.
    fn read(&mut self, message: BridgeMessage) -> Option<String> {
        match message {
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
            BridgeMessage::Gain { .. }
            | BridgeMessage::Resources(_)
            | BridgeMessage::OptionsSet { .. }
            | BridgeMessage::OptionsAnswer { .. } => None,
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

/// What starts the ids of the app's messages, which Voicemod's answers and broadcasts carry
const OUR_IDS: &str = "drezimo-";

/// `kind`: "appMessage", for Voicemod's window, or "busMessage", for its engine
fn request(message_id: &str, kind: &str, command: &str, object: Value) -> Message {
    let request = json!({
        "bridgeData": {
            "data": { "id": message_id, "object": object, "type": command },
            "type": kind,
        }
    });
    Message::text(request.to_string())
}

fn stop_request(message_id: &str, sound: &str) -> Message {
    request(message_id, "appMessage", "stopSound", json!({ "id": sound }))
}

fn gain_request(message_id: &str, channel: Channel) -> Message {
    let object = json!({ "controlType": channel.control_type() });
    request(message_id, "appMessage", "GetVolumeGain", object)
}

fn set_gain_request(message_id: &str, channel: Channel, gain: f32, enabled: bool) -> Message {
    let object = json!({ "controlType": channel.control_type(), "enabled": enabled, "value": gain });
    request(message_id, "appMessage", "SetVolumeGain", object)
}

fn sound_volume_request(message_id: &str, sound: &str, volume: f32) -> Message {
    request(
        message_id,
        "busMessage",
        "updateSoundVolume",
        json!({ "id": sound, "volume": volume }),
    )
}

/// Voicemod's window sends a sound's options all together, with its id among them
fn options_request(message_id: &str, sound: &str, options: Map<String, Value>) -> Message {
    let mut object = options;
    object.insert("id".into(), sound.into());
    request(message_id, "appMessage", "ModifySoundOptions", Value::Object(object))
}

/// Every resource of Voicemod's, with its options: about 2 MB
fn resources_request(message_id: &str) -> Message {
    request(message_id, "appMessage", "GetAllResources", json!({}))
}

pub enum BridgeEvent {
    /// Whether the bridge is there
    Found(bool),
    /// A mixer channel's gain, and whether it is on when Voicemod tells. `ours`: the app's own change coming back
    Gain {
        channel: Channel,
        gain: f32,
        enabled: Option<bool>,
        ours: bool,
    },
    /// Each sound's volume saved in Voicemod, 0 to 1
    SoundVolumes(HashMap<String, f32>),
    /// A sound's own options changed in Voicemod, as the app asked: its soundboards tell them
    SoundChanged,
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
    Gain {
        channel: Channel,
        gain: f32,
        enabled: Option<bool>,
        ours: bool,
    },
    /// Every resource Voicemod has, with their options
    Resources(Value),
    /// A sound's options sent to the engine, by Voicemod's window or the app
    OptionsSet {
        sound: String,
        options: Map<String, Value>,
    },
    /// The engine's answer to a change of options
    OptionsAnswer {
        changed: bool,
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
        command @ ("GetVolumeGain" | "SetVolumeGain") => {
            let asked_by_us = object["frontendMessageId"]
                .as_str()
                .is_some_and(|id| id.starts_with(OUR_IDS));
            // The app's own read is Voicemod's own value; another client's read tells the knob's changes too
            if command == "GetVolumeGain" && !asked_by_us {
                return None;
            }
            Some(BridgeMessage::Gain {
                channel: Channel::from_control_type(object["controlType"].as_str()?)?,
                gain: object["value"].as_f64()? as f32,
                // A change tells it, a read does not: a channel off then reads as 0
                enabled: object["enabled"].as_bool(),
                ours: command == "SetVolumeGain" && asked_by_us,
            })
        }
        // Voicemod asks its servers: a failed answer is no empty library
        "GetAllResources" if data["error"].is_null() && object["collections"].is_array() => {
            Some(BridgeMessage::Resources(object.clone()))
        }
        "ModifySoundOptions" => match object {
            Value::Object(options) if options.contains_key("reproductionMode") => {
                let mut options = options.clone();
                let sound = options.remove("id")?.as_str()?.to_string();
                Some(BridgeMessage::OptionsSet { sound, options })
            }
            _ => Some(BridgeMessage::OptionsAnswer {
                changed: data["error"].is_null(),
            }),
        },
        _ => None,
    }
}

fn sounds(resources: &Value) -> impl Iterator<Item = &Value> {
    let collections = resources["collections"].as_array().into_iter().flatten();
    collections
        .flat_map(|collection| collection["resources"].as_array().into_iter().flatten())
        .filter(|resource| resource["type"] == "sound")
}

fn volumes(options: &HashMap<String, Map<String, Value>>) -> HashMap<String, f32> {
    options
        .iter()
        .filter_map(|(sound, options)| Some((sound.clone(), options.get("volume")?.as_f64()? as f32)))
        .collect()
}

fn all_options(resources: &Value) -> HashMap<String, Map<String, Value>> {
    sounds(resources)
        .filter_map(|resource| {
            let options = resource["options"].as_object()?;
            Some((resource["id"].as_str()?.to_string(), options.clone()))
        })
        .collect()
}

/// What a message of the bridge carries
fn data(text: &str) -> Option<Value> {
    let mut message: Value = serde_json::from_str(text).ok()?;
    Some(message.get_mut("bridgeData")?.get_mut("data")?.take())
}

/// The bridge among the ports Voicemod listens on, if any.
async fn find(refused: &mut Refused) -> Option<Socket> {
    let listeners = tauri::async_runtime::spawn_blocking(listeners)
        .await
        .unwrap_or_default();
    refused.retain(|&(process, port), _| {
        listeners
            .iter()
            .any(|listener| listener.process == process && listener.port == port)
    });
    for listener in bridge_candidates(&listeners, is_voicemod) {
        let key = (listener.process, listener.port);
        if refused.get(&key).is_some_and(|since| since.elapsed() > GIVE_UP_AFTER) {
            continue;
        }
        let url = format!("ws://127.0.0.1:{}/", listener.port);
        let Ok(Ok((mut socket, _))) = timeout(ANSWER_TIME, tokio_tungstenite::connect_async(url)).await else {
            continue;
        };
        if is_bridge(&mut socket).await {
            refused.remove(&key);
            return Some(socket);
        }
        refused.entry(key).or_insert_with(Instant::now);
    }
    None
}

/// On Windows the bridge greets every client with the state of Voicemod's window. On macOS it waits to be asked: it is
/// asked to stop a sound that does not exist, which it answers with an error and nothing else.
async fn is_bridge(socket: &mut Socket) -> bool {
    let probe = stop_request("drezimo-probe", "drezimo-probe");
    if cfg!(target_os = "macos") && socket.send(probe).await.is_err() {
        return false;
    }
    let until = tokio::time::Instant::now() + ANSWER_TIME;
    while let Ok(Some(Ok(message))) = tokio::time::timeout_at(until, socket.next()).await {
        // Past the WebSocket's own frames, such as a ping
        if let Message::Text(text) = message {
            return data(&text).is_some();
        }
    }
    false
}

/// A socket listening for connections
struct Listener {
    port: u16,
    loopback: bool,
    process: u32,
}

/// Voicemod's ports on this computer only, except the fixed ones.
fn bridge_candidates(listeners: &[Listener], is_voicemod: impl Fn(u32) -> bool) -> Vec<&Listener> {
    listeners
        .iter()
        .filter(|listener| {
            listener.loopback && !NOT_THE_BRIDGE.contains(&listener.port) && is_voicemod(listener.process)
        })
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

/// Voicemod's only, and none while it is closed: lsof is a program to start, and looks at every process otherwise.
#[cfg(target_os = "macos")]
fn listeners() -> Vec<Listener> {
    let voicemod: Vec<String> = processes()
        .into_iter()
        .filter(|&process| is_voicemod(process))
        .map(|process| process.to_string())
        .collect();
    if voicemod.is_empty() {
        return Vec::new();
    }
    // -b and -w: not held up by a network drive that does not answer
    let lsof = std::process::Command::new("/usr/sbin/lsof")
        .args([
            "-nP",
            "-b",
            "-w",
            "-a",
            "-iTCP",
            "-sTCP:LISTEN",
            "-Fpn",
            "-p",
            &voicemod.join(","),
        ])
        .output();
    match lsof {
        Ok(output) => lsof_listeners(&String::from_utf8_lossy(&output.stdout)),
        Err(_) => Vec::new(),
    }
}

/// lsof's `-F` output: a `p` line for each process, then an `n` line for each of its sockets.
#[cfg(target_os = "macos")]
fn lsof_listeners(output: &str) -> Vec<Listener> {
    let mut process = 0;
    let mut listeners = Vec::new();
    for line in output.lines() {
        if let Some(pid) = line.strip_prefix('p') {
            process = pid.parse().unwrap_or(0);
        } else if let Some(name) = line.strip_prefix('n')
            && let Some((address, port)) = name.rsplit_once(':')
            && let Ok(port) = port.parse()
        {
            listeners.push(Listener {
                port,
                loopback: address == "127.0.0.1",
                process,
            });
        }
    }
    listeners
}

#[cfg(target_os = "macos")]
fn processes() -> Vec<u32> {
    let count = unsafe { libc::proc_listallpids(std::ptr::null_mut(), 0) };
    // With room for those started in between
    let mut processes = vec![0i32; count.max(0) as usize + 32];
    let size = std::mem::size_of_val(processes.as_slice()) as i32;
    let count = unsafe { libc::proc_listallpids(processes.as_mut_ptr().cast(), size) };
    processes.truncate(count.max(0) as usize);
    processes.into_iter().map(|process| process as u32).collect()
}

#[cfg(target_os = "macos")]
fn is_voicemod(process: u32) -> bool {
    let mut path = [0u8; libc::PROC_PIDPATHINFO_MAXSIZE as usize];
    let length = unsafe { libc::proc_pidpath(process as i32, path.as_mut_ptr().cast(), path.len() as u32) };
    length > 0 && is_voicemod_path(&path[..length as usize])
}

/// Voicemod's launcher in Applications, and the app it keeps up to date in Application Support
#[cfg(target_os = "macos")]
fn is_voicemod_path(path: &[u8]) -> bool {
    path.ends_with(b"/Voicemod.app/Contents/MacOS/Voicemod")
}

/// Not looked for on other systems: the pads stop every sound there
#[cfg(not(any(target_os = "macos", windows)))]
fn listeners() -> Vec<Listener> {
    Vec::new()
}

#[cfg(not(any(target_os = "macos", windows)))]
fn is_voicemod(_process: u32) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    // Shapes recorded from Voicemod 3.17.45 on Windows, the same as 3.17.43's on macOS

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

    fn read(stops: &mut Stops, text: &str) -> Option<String> {
        stops.read(message(text)?)
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
        assert_eq!(read(&mut stops, &answer("drezimo-2", false)), None);
        assert!(second.try_recv().is_ok());
        assert!(first.try_recv().is_err(), "still waiting");
        assert_eq!(
            read(&mut stops, &answer("drezimo-probe", true)),
            None,
            "the one that found the bridge"
        );
    }

    #[test]
    fn a_sound_still_loading_is_stopped_once_it_starts() {
        let mut stops = Stops::default();
        stops.pressed("a".into());
        stops.pressed("b".into());
        waiting(&mut stops, "drezimo-1", "a");
        waiting(&mut stops, "drezimo-2", "b");
        read(&mut stops, &answer("drezimo-1", true));
        read(&mut stops, &answer("drezimo-2", true));
        assert_eq!(read(&mut stops, &state("a", "created")), None);
        assert_eq!(read(&mut stops, &state("a", "playing")).as_deref(), Some("a"));
        assert_eq!(read(&mut stops, &state("b", "playing")).as_deref(), Some("b"));
        assert_eq!(read(&mut stops, &state("a", "playing")), None, "once");

        // Pressed again meanwhile: that press is what the user wants
        stops.pressed("c".into());
        waiting(&mut stops, "drezimo-3", "c");
        read(&mut stops, &answer("drezimo-3", true));
        stops.pressed("c".into());
        assert_eq!(read(&mut stops, &state("c", "playing")), None);
    }

    #[test]
    fn a_sound_that_already_ended_is_not_stopped_later() {
        let mut stops = Stops::default();
        stops.pressed("a".into());
        read(&mut stops, &state("a", "playing"));
        read(&mut stops, &state("a", "finished"));
        waiting(&mut stops, "drezimo-1", "a");
        read(&mut stops, &answer("drezimo-1", true));
        // Started again from Voicemod's own window
        assert_eq!(read(&mut stops, &state("a", "playing")), None);
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
            listener(39999, true, 7),
            listener(50700, true, 8),
        ];
        let ports: Vec<_> = bridge_candidates(&listeners, |process| process == 7)
            .iter()
            .map(|listener| listener.port)
            .collect();
        assert_eq!(ports, [50658]);
    }

    /// A WebSocket for one client on this computer: it sends `first` at once, then answers each message as the bridge
    /// answers a stop, if `answers`
    async fn server(first: Vec<Message>, answers: bool) -> u16 {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tauri::async_runtime::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = tokio_tungstenite::accept_async(stream).await.unwrap();
            for message in first {
                let _ = socket.send(message).await;
            }
            while let Some(Ok(message)) = socket.next().await {
                if answers && message.is_text() {
                    let _ = socket.send(Message::text(answer("drezimo-probe", true))).await;
                }
            }
        });
        port
    }

    async fn is_bridge_at(port: u16) -> bool {
        let url = format!("ws://127.0.0.1:{port}/");
        let (mut socket, _) = tokio_tungstenite::connect_async(url).await.unwrap();
        is_bridge(&mut socket).await
    }

    #[test]
    fn knows_the_bridge_by_what_it_says() {
        tauri::async_runtime::block_on(async {
            let greeting = Message::text(state("s1", "playing"));
            let greets = server(vec![Message::Ping(Default::default()), greeting], false).await;
            assert!(is_bridge_at(greets).await, "greets, as on Windows");

            let answers = server(Vec::new(), true).await;
            assert_eq!(
                is_bridge_at(answers).await,
                cfg!(target_os = "macos"),
                "answers, as on macOS"
            );

            let silent = server(Vec::new(), false).await;
            assert!(!is_bridge_at(silent).await);

            let other = server(vec![Message::text(r#"{"actionType": "playMeme"}"#)], false).await;
            assert!(!is_bridge_at(other).await, "another program's messages");
        });
    }

    fn gain_message(command: &str, control_type: &str, message_id: &str, value: f32) -> String {
        format!(
            r#"{{"bridgeData":{{"data":{{"id":"","object":{{"controlType":"{control_type}","frontendMessageId":"{message_id}",
            "sourceTrigger":"UI","value":{value}}},"type":"{command}"}},"type":"appMessage"}}}}"#
        )
    }

    #[test]
    fn reads_a_channel_turned_off() {
        let muted = r#"{"bridgeData":{"data":{"id":"","object":{"controlType":"monitorSoundboardPreMix",
            "enabled":false,"sourceTrigger":"Websocket","value":1},"type":"SetVolumeGain"},"type":"appMessage"}}"#;
        let expected = BridgeMessage::Gain {
            channel: Channel::SoundboardForMe,
            gain: 1.0,
            enabled: Some(false),
            ours: false,
        };
        assert_eq!(message(muted), Some(expected));
    }

    #[test]
    fn a_failed_resources_answer_is_no_empty_library() {
        let answer = |data: Value| json!({"bridgeData": {"type": "appMessage", "data": data}}).to_string();
        let failed = answer(json!({"type": "GetAllResources", "error": {"code": 429}, "object": {}}));
        assert_eq!(message(&failed), None);
        let empty = answer(json!({"type": "GetAllResources", "object": {}}));
        assert_eq!(message(&empty), None);
        let read = answer(json!({"type": "GetAllResources", "object": {"collections": []}}));
        assert_eq!(
            message(&read),
            Some(BridgeMessage::Resources(json!({"collections": []})))
        );
    }

    #[test]
    fn reads_gains_and_tells_ours_apart() {
        let gain = |channel, gain, ours| {
            Some(BridgeMessage::Gain {
                channel,
                gain,
                enabled: None,
                ours,
            })
        };
        let monitor = gain_message("GetVolumeGain", "monitor", "probe-3", 0.3025);
        assert_eq!(message(&monitor), None, "not a channel of the knob's");
        let read = gain_message("GetVolumeGain", "input", "drezimo-2", 1.5);
        assert_eq!(
            message(&read),
            gain(Channel::Microphone, 1.5, false),
            "Voicemod's own value, asked by the app"
        );
        // Another client's read tells the value now, the knob's changes included: not Voicemod's own
        let their_read = gain_message("GetVolumeGain", "input", "3f1c9a", 1.2);
        assert_eq!(message(&their_read), None);
        let ours = gain_message("SetVolumeGain", "streamingSoundboardPreMix", "drezimo-4", 0.25);
        assert_eq!(message(&ours), gain(Channel::SoundboardForOthers, 0.25, true));
        let theirs = gain_message("SetVolumeGain", "monitorSoundboardPreMix", "3f1c9a", 0.5);
        assert_eq!(
            message(&theirs),
            gain(Channel::SoundboardForMe, 0.5, false),
            "Voicemod's own slider"
        );
        let no_value = gain_message("SetVolumeGain", "input", "3f1c9a", 1.0).replace(r#""value":1"#, r#""value":"x""#);
        assert_eq!(message(&no_value), None);
    }

    #[test]
    fn reads_every_sounds_saved_volume() {
        let resources = r#"{"bridgeData":{"data":{"id":"","object":{"collections":[
            {"id":"c1","type":"soundboard","resources":[
                {"id":"s1","type":"sound","name":"Honk","options":{"volume":0.5,"loop":false}},
                {"id":"v1","type":"voice","name":"Robot"},
                {"id":"s2","type":"sound","name":"Fart"}]},
            {"id":"c2","type":"soundboard","resources":[
                {"id":"s3","type":"sound","options":{"volume":1}},
                {"id":"s4","type":"sound","options":{"volume":"loud"}}]},
            {"id":"c3"}]},"type":"GetAllResources"},"type":"appMessage"}}"#;
        let Some(BridgeMessage::Resources(read)) = message(resources) else {
            panic!("the resources");
        };
        let volumes = HashMap::from([("s1".to_string(), 0.5), ("s3".to_string(), 1.0)]);
        let options = all_options(&read);
        assert_eq!(super::volumes(&options), volumes);
        assert_eq!(options.len(), 3, "every sound with options");
        assert_eq!(options["s1"]["loop"], false);
    }

    fn resources(mode: &str) -> Value {
        json!({"collections": [{"resources": [
            {"id": "s1", "type": "sound", "options": {"reproductionMode": mode, "volume": 0.5, "fadeInDuration": 0}}
        ]}]})
    }

    fn changes(pairs: &[(&str, Value)]) -> Map<String, Value> {
        pairs
            .iter()
            .map(|(key, value)| (key.to_string(), value.clone()))
            .collect()
    }

    #[test]
    fn a_sounds_options_go_all_together() {
        let now = Instant::now();
        let mut options = SoundOptions::default();
        let mode = changes(&[("reproductionMode", json!("playOverlap"))]);
        assert_eq!(options.change("s1".into(), mode, now), Change::Read, "read first");
        let volume = changes(&[("volume", json!(0.2))]);
        assert_eq!(
            options.change("s1".into(), volume, now),
            Change::Wait,
            "one read for both"
        );
        let ready = options.read(&resources("playRestart"), now);
        let expected = changes(&[
            ("reproductionMode", json!("playOverlap")),
            ("volume", json!(0.2)),
            ("fadeInDuration", json!(0)),
        ]);
        assert_eq!(ready, [("s1".to_string(), expected)]);

        let back = changes(&[("reproductionMode", json!("playRestart"))]);
        let Change::Send(sound, sent) = options.change("s1".into(), back, now) else {
            panic!("known: sent at once");
        };
        assert_eq!(
            (sound.as_str(), &sent["volume"], &sent["reproductionMode"]),
            ("s1", &json!(0.2), &json!("playRestart"))
        );
        assert_eq!(options.change("gone".into(), Map::new(), now), Change::Read);
        assert!(
            options.read(&resources("playRestart"), now).is_empty(),
            "not listed: dropped"
        );

        options.refused();
        assert_eq!(
            options.change("s1".into(), Map::new(), now),
            Change::Read,
            "read again after a refusal"
        );
    }

    #[test]
    fn a_change_waiting_too_long_is_dropped() {
        let now = Instant::now();
        let mut options = SoundOptions::default();
        options.change("s1".into(), changes(&[("volume", json!(0.1))]), now);
        let later = now + READ_WAIT;
        assert_eq!(
            options.change("s2".into(), Map::new(), later),
            Change::Read,
            "the first read never came"
        );
        assert!(
            options.read(&resources("playStop"), later).is_empty(),
            "s1 too old, s2 not listed"
        );
    }

    #[test]
    fn follows_the_options_voicemods_window_sends() {
        let now = Instant::now();
        let mut options = SoundOptions::default();
        options.read(&resources("playRestart"), now);
        let window = r#"{"bridgeData":{"data":{"id":"","object":{"id":"s1","reproductionMode":"playPause","volume":0.9},
            "type":"ModifySoundOptions"},"type":"appMessage"}}"#;
        let Some(BridgeMessage::OptionsSet { sound, options: set }) = message(window) else {
            panic!("the window's change");
        };
        options.set(sound, set);
        let Change::Send(_, sent) = options.change("s1".into(), Map::new(), now) else {
            panic!("known");
        };
        assert_eq!(
            (&sent["reproductionMode"], &sent["volume"]),
            (&json!("playPause"), &json!(0.9))
        );

        let answer =
            r#"{"bridgeData":{"data":{"id":"","object":null,"type":"ModifySoundOptions"},"type":"appMessage"}}"#;
        assert_eq!(message(answer), Some(BridgeMessage::OptionsAnswer { changed: true }));
        let refused = r#"{"bridgeData":{"data":{"error":{"code":"UnableModifySoundOptions"},"id":"","object":null,
            "type":"ModifySoundOptions"},"type":"appMessage"}}"#;
        assert_eq!(message(refused), Some(BridgeMessage::OptionsAnswer { changed: false }));
    }

    #[test]
    fn builds_the_volume_messages() {
        let text = |message: Message| serde_json::from_str::<Value>(message.to_text().unwrap()).unwrap();
        assert_eq!(
            text(set_gain_request("drezimo-1", Channel::Microphone, 1.5, true)),
            json!({"bridgeData": {"type": "appMessage", "data": {"id": "drezimo-1", "type": "SetVolumeGain",
                "object": {"controlType": "input", "enabled": true, "value": 1.5}}}})
        );
        let volume = text(sound_volume_request("drezimo-2", "s1", 0.35));
        assert_eq!(volume["bridgeData"]["type"], "busMessage");
        assert_eq!(volume["bridgeData"]["data"]["type"], "updateSoundVolume");
        assert_eq!(volume["bridgeData"]["data"]["object"]["id"], "s1");
        assert!((volume["bridgeData"]["data"]["object"]["volume"].as_f64().unwrap() - 0.35).abs() < 1e-6);
    }

    /// A WebSocket for clients on this computer that passes on every text it receives
    async fn recording_server() -> (u16, mpsc::UnboundedReceiver<Value>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let (heard, received) = mpsc::unbounded_channel();
        tauri::async_runtime::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let mut socket = tokio_tungstenite::accept_async(stream).await.unwrap();
                while let Some(Ok(Message::Text(text))) = socket.next().await {
                    let _ = heard.send(serde_json::from_str::<Value>(&text).unwrap());
                }
            }
        });
        (port, received)
    }

    #[test]
    fn gains_are_read_again_on_each_connection() {
        tauri::async_runtime::block_on(async {
            let (port, mut received) = recording_server().await;
            for _ in 0..2 {
                let url = format!("ws://127.0.0.1:{port}/");
                let (socket, _) = tokio_tungstenite::connect_async(url).await.unwrap();
                let (jobs, mut pending) = mpsc::unbounded_channel();
                let serving = tauri::async_runtime::spawn(async move { serve(socket, &mut pending, &|_| {}).await });
                let mut asked = Vec::new();
                for _ in 0..3 {
                    let request = timeout(ANSWER_TIME, received.recv()).await.unwrap().unwrap();
                    let data = &request["bridgeData"]["data"];
                    asked.push((
                        data["type"].as_str().unwrap().to_string(),
                        data["object"]["controlType"].clone(),
                    ));
                }
                let read_gains: Vec<_> = Channel::ALL
                    .iter()
                    .map(|channel| ("GetVolumeGain".to_string(), json!(channel.control_type())))
                    .collect();
                assert_eq!(asked, read_gains);
                drop(jobs);
                assert!(serving.await.unwrap().is_ok());
            }
        });
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn reads_the_listening_sockets_from_lsof() {
        let output = "p631\nf5\nn*:7000\np89725\nf30\nn127.0.0.1:50106\nf38\nn*:39998\nf41\nn[::1]:59130\n";
        let listeners = lsof_listeners(output);
        let read: Vec<_> = listeners
            .iter()
            .map(|listener| (listener.port, listener.loopback, listener.process))
            .collect();
        assert_eq!(
            read,
            [
                (7000, false, 631),
                (50106, true, 89725),
                (39998, false, 89725),
                (59130, false, 89725)
            ]
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn knows_voicemod_by_its_path() {
        let support = "/Users/me/Library/Application Support/VoicemodV3/app/last/Voicemod.app";
        assert!(is_voicemod_path(b"/Applications/Voicemod.app/Contents/MacOS/Voicemod"));
        assert!(is_voicemod_path(
            format!("{support}/Contents/MacOS/Voicemod").as_bytes()
        ));
        assert!(!is_voicemod_path(
            format!("{support}/Contents/MacOS/crashpad_handler").as_bytes()
        ));
        let helper = "/Contents/Frameworks/QtWebEngineCore.framework/Helpers/QtWebEngineProcess.app/Contents/MacOS";
        assert!(!is_voicemod_path(
            format!("{support}{helper}/QtWebEngineProcess").as_bytes()
        ));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn lists_the_processes() {
        assert!(processes().contains(&std::process::id()));
        assert!(!is_voicemod(std::process::id()));
    }
}
