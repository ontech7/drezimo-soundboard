//! The link to Voicemod's Control API (docs/project.md, section 8): a WebSocket on this computer, opened by the
//! Voicemod app, that plays the sounds and reports the soundboards and Voicemod's switches.

use crate::soundboard::Soundboard;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio_tungstenite::tungstenite::{self, Message};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

const URL: &str = "ws://localhost:59129/v1/";
/// Built in from the environment or the repository's .env file, see build.rs
const CLIENT_KEY: Option<&str> = option_env!("VOICEMOD_CLIENT_KEY");
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const RETRY_AFTER: Duration = Duration::from_secs(3);
/// A rejected key does not get better by asking often
const REJECTED_RETRY_AFTER: Duration = Duration::from_secs(60);

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum VoicemodStatus {
    #[default]
    Connecting,
    Connected,
    /// Voicemod is not running, or not answering
    Unreachable,
    KeyRejected,
    /// The app was built without a client key
    MissingKey,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Request {
    /// Exactly a click on the sound in Voicemod; the key-up only matters to the sounds that play while held
    PlaySound {
        id: String,
        key_down: bool,
    },
    StopAllSounds,
    ToggleMuteForMe,
    ToggleVoiceChanger,
    ToggleHearMyself,
}

impl Request {
    fn action(&self) -> (&'static str, Value) {
        match self {
            Self::PlaySound { id, key_down } => ("playMeme", json!({ "FileName": id, "IsKeyDown": key_down })),
            Self::StopAllSounds => ("stopAllMemeSounds", json!({})),
            Self::ToggleMuteForMe => ("toggleMuteMemeForMe", json!({})),
            Self::ToggleVoiceChanger => ("toggleVoiceChanger", json!({})),
            Self::ToggleHearMyself => ("toggleHearMyVoice", json!({})),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum VoicemodEvent {
    Status(VoicemodStatus),
    Soundboards(Vec<Soundboard>),
    /// The soundboard picked in Voicemod's window
    ActiveSoundboard(String),
    MuteForMe(bool),
    VoiceChanger(bool),
    HearMyself(bool),
}

pub struct Voicemod {
    requests: UnboundedSender<Request>,
}

impl Voicemod {
    /// Connects to Voicemod in the background, again whenever it goes away; `on_event` runs on the async runtime.
    pub fn start(on_event: impl Fn(VoicemodEvent) + Send + Sync + 'static) -> Self {
        let (requests, outgoing) = mpsc::unbounded_channel();
        tauri::async_runtime::spawn(keep_connected(outgoing, on_event));
        Self { requests }
    }

    /// Sends a request to Voicemod. Dropped if Voicemod is not connected.
    pub fn send(&self, request: Request) {
        // The connection task lives as long as the app, unless the build has no key
        let _ = self.requests.send(request);
    }
}

async fn keep_connected(mut requests: UnboundedReceiver<Request>, on_event: impl Fn(VoicemodEvent)) {
    let Some(key) = CLIENT_KEY.filter(|key| !key.is_empty()) else {
        on_event(VoicemodEvent::Status(VoicemodStatus::MissingKey));
        return;
    };
    let mut status = VoicemodStatus::Connecting;
    loop {
        // Requests made while Voicemod was away are stale
        while requests.try_recv().is_ok() {}
        let retry_after = match tokio::time::timeout(CONNECT_TIMEOUT, connect(key)).await {
            Ok(Ok(socket)) => {
                report(&mut status, VoicemodStatus::Connected, &on_event);
                if let Err(error) = exchange(socket, &mut requests, &on_event).await {
                    eprintln!("lost the connection to Voicemod: {error}");
                }
                report(&mut status, VoicemodStatus::Unreachable, &on_event);
                RETRY_AFTER
            }
            Ok(Err(Registration::Rejected(reason))) => {
                eprintln!("Voicemod rejected the client key: {reason}");
                report(&mut status, VoicemodStatus::KeyRejected, &on_event);
                REJECTED_RETRY_AFTER
            }
            // Not running, still starting, or not answering
            Ok(Err(Registration::Failed)) | Err(_) => {
                report(&mut status, VoicemodStatus::Unreachable, &on_event);
                RETRY_AFTER
            }
        };
        tokio::time::sleep(retry_after).await;
    }
}

fn report(status: &mut VoicemodStatus, new: VoicemodStatus, on_event: &impl Fn(VoicemodEvent)) {
    if *status != new {
        *status = new;
        on_event(VoicemodEvent::Status(new));
    }
}

enum Registration {
    Rejected(String),
    Failed,
}

impl From<tungstenite::Error> for Registration {
    fn from(_: tungstenite::Error) -> Self {
        Self::Failed
    }
}

/// Opens the socket and registers the app with its client key.
async fn connect(key: &str) -> Result<Socket, Registration> {
    let (mut socket, _) = tokio_tungstenite::connect_async(URL).await?;
    socket
        .send(frame("registerClient", json!({ "clientKey": key })))
        .await?;
    // Voicemod greets first, then answers the registration
    while let Some(message) = socket.next().await {
        if let Message::Text(text) = message? {
            match parse(&text) {
                Some(Incoming::Registered) => return Ok(socket),
                Some(Incoming::Rejected(reason)) => return Err(Registration::Rejected(reason)),
                _ => {}
            }
        }
    }
    Err(Registration::Failed)
}

/// Asks for the current state, then passes requests and events until Voicemod goes away.
async fn exchange(
    mut socket: Socket,
    requests: &mut UnboundedReceiver<Request>,
    on_event: &impl Fn(VoicemodEvent),
) -> Result<(), tungstenite::Error> {
    for action in [
        "getAllSoundboard",
        "getMuteMemeForMeStatus",
        "getVoiceChangerStatus",
        "getHearMyselfStatus",
    ] {
        socket.send(frame(action, json!({}))).await?;
    }
    loop {
        tokio::select! {
            message = socket.next() => match message {
                Some(Ok(Message::Text(text))) => {
                    if let Some(Incoming::Event(event)) = parse(&text) {
                        on_event(event);
                    }
                }
                // Pings are answered by tungstenite
                Some(Ok(_)) => {}
                Some(Err(error)) => return Err(error),
                None => return Ok(()),
            },
            Some(request) = requests.recv() => {
                let (action, payload) = request.action();
                socket.send(frame(action, payload)).await?;
            }
        }
    }
}

fn frame(action: &str, payload: Value) -> Message {
    static NEXT_ID: AtomicU64 = AtomicU64::new(1);
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed).to_string();
    Message::text(json!({ "id": id, "action": action, "payload": payload }).to_string())
}

#[derive(Debug, PartialEq)]
enum Incoming {
    Registered,
    Rejected(String),
    Event(VoicemodEvent),
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Envelope {
    action_type: Option<String>,
    action: Option<String>,
    #[serde(default)]
    payload: Value,
    #[serde(default)]
    action_object: Value,
}

fn parse(text: &str) -> Option<Incoming> {
    let envelope: Envelope = serde_json::from_str(text).ok()?;
    let kind = envelope.action_type.or(envelope.action)?;
    let object = &envelope.action_object;
    let switch = || object["value"].as_bool();

    let event = match kind.as_str() {
        "registerClient" => {
            let status = [&envelope.payload["status"], &object["status"]]
                .into_iter()
                .find(|status| !status.is_null())?;
            return Some(match status["code"].as_u64() {
                Some(200) => Incoming::Registered,
                _ => Incoming::Rejected(status["description"].as_str().unwrap_or_default().to_string()),
            });
        }
        "getAllSoundboard" => match Vec::<Soundboard>::deserialize(&object["soundboards"]) {
            Ok(soundboards) => VoicemodEvent::Soundboards(soundboards),
            Err(error) => {
                eprintln!("unexpected soundboard list from Voicemod: {error}");
                return None;
            }
        },
        "getActiveSoundboardProfile" => {
            let id = envelope.payload["profileId"]
                .as_str()
                .or(object["profileId"].as_str())?;
            VoicemodEvent::ActiveSoundboard(id.to_string())
        }
        // Voicemod 3.17 answers the status requests with the toggle actions
        "toggleMuteMemeForMe" | "getMuteMemeForMeStatus" => VoicemodEvent::MuteForMe(switch()?),
        "toggleVoiceChanger" | "getVoiceChangerStatus" => VoicemodEvent::VoiceChanger(switch()?),
        "voiceChangerEnabledEvent" => VoicemodEvent::VoiceChanger(true),
        "voiceChangerDisabledEvent" => VoicemodEvent::VoiceChanger(false),
        "toggleHearMyVoice" | "getHearMyselfStatus" => VoicemodEvent::HearMyself(switch()?),
        _ => return None,
    };
    Some(Incoming::Event(event))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Shapes recorded from Voicemod 3.17.43

    #[test]
    fn reads_the_registration_answer() {
        let greeting = r#"{"appVersion": "3.17.43", "msg": "Pending authentication", "server": "Kestrel"}"#;
        assert_eq!(parse(greeting), None);

        let authorized = r#"{"action": "registerClient", "actionID": "1", "actionObject": {"status": {"code": 200,
            "description": "Authorized"}}, "actionType": "registerClient", "id": "1",
            "payload": {"status": {"code": 200, "description": "Authorized"}}}"#;
        assert_eq!(parse(authorized), Some(Incoming::Registered));

        let rejected = r#"{"actionType": "registerClient", "actionObject": {"status": {"code": 401,
            "description": "Unauthorized"}}}"#;
        assert_eq!(parse(rejected), Some(Incoming::Rejected("Unauthorized".into())));

        let no_status = r#"{"actionType": "registerClient", "payload": {}}"#;
        assert_eq!(parse(no_status), None);
    }

    #[test]
    fn reads_the_switches() {
        let cases = [
            ("toggleMuteMemeForMe", true, VoicemodEvent::MuteForMe(true)),
            ("toggleVoiceChanger", false, VoicemodEvent::VoiceChanger(false)),
            ("toggleHearMyVoice", true, VoicemodEvent::HearMyself(true)),
        ];
        for (action, value, expected) in cases {
            let message =
                format!(r#"{{"actionID": "9", "actionObject": {{"value": {value}}}, "actionType": "{action}"}}"#);
            assert_eq!(parse(&message), Some(Incoming::Event(expected)), "{action}");
        }
        let enabled = r#"{"actionType": "voiceChangerEnabledEvent"}"#;
        assert_eq!(parse(enabled), Some(Incoming::Event(VoicemodEvent::VoiceChanger(true))));
    }

    #[test]
    fn reads_the_soundboards_and_the_active_one() {
        let list = r#"{"actionType": "getAllSoundboard", "actionObject": {"soundboards": [{"enabled": true,
            "id": "b1", "name": "My first soundboard", "showProLogo": false, "sounds": [{"enabled": true, "id": "s1",
            "name": "Honk", "playbackMode": "PlayRestart", "loop": false}]}]}}"#;
        let Some(Incoming::Event(VoicemodEvent::Soundboards(soundboards))) = parse(list) else {
            panic!("a soundboard list");
        };
        assert_eq!(soundboards[0].sounds[0].name, "Honk");

        let active = r#"{"action": "getActiveSoundboardProfile", "payload": {"profileId": "b1"}}"#;
        assert_eq!(
            parse(active),
            Some(Incoming::Event(VoicemodEvent::ActiveSoundboard("b1".into())))
        );
    }

    #[test]
    fn ignores_everything_else() {
        let not_found =
            r#"{"actionObject": {"commandName": "", "commandNotFound": "true"}, "actionType": "commandNotFound"}"#;
        for text in [not_found, r#"{"actionType": "playMeme"}"#, "not json"] {
            assert_eq!(parse(text), None, "{text}");
        }
    }

    #[test]
    fn writes_the_requests() {
        let play = Request::PlaySound {
            id: "s1".into(),
            key_down: true,
        };
        assert_eq!(
            play.action(),
            ("playMeme", json!({ "FileName": "s1", "IsKeyDown": true }))
        );
        assert_eq!(Request::ToggleHearMyself.action().0, "toggleHearMyVoice");

        let Message::Text(text) = frame("stopAllMemeSounds", json!({})) else {
            panic!("a text message")
        };
        let sent: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(sent["action"], "stopAllMemeSounds");
        assert!(sent["id"].as_str().is_some_and(|id| !id.is_empty()));
    }
}
