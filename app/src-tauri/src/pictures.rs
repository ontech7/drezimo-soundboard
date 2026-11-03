//! The pictures of the sounds and voices, shown on the window's pads only, never on the overlay or the display.
//! Voicemod sends them through its API; they are kept in the app's cache folder under the checksum Voicemod names
//! them by, so each is asked for once. The window loads them from pad-image://localhost/<kind>/<id>/<checksum>.
//!
//! Voicemod carries out its actions one at a time, so the pictures are asked for one at a time: a pad pressed while a
//! page of them loads waits for one picture at most, not for all of them.

use crate::soundboard::Pad;
use crate::voicemod::{Picture, Request, Voicemod};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;
use tauri::http::{Response, StatusCode, header};
use tauri::{AppHandle, Manager};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot;
use tokio::time::Instant;

pub const SCHEME: &str = "pad-image";
/// Voicemod never answers for an id it does not know
const ANSWER_WAIT: Duration = Duration::from_secs(5);
/// How long the window waits for a picture, its turn included
const WINDOW_WAIT: Duration = Duration::from_secs(20);
/// Pictures wanted this close to the newest one belong to the same page: the page shown last goes first, in order
const SAME_PAGE: Duration = Duration::from_millis(500);

type Answer = oneshot::Sender<Option<Vec<u8>>>;

pub struct Pictures {
    folder: Option<PathBuf>,
    jobs: OnceLock<UnboundedSender<Job>>,
}

enum Job {
    Want {
        of: Picture,
        checksum: String,
        answer: Answer,
    },
    Received {
        of: Picture,
        file: Option<Vec<u8>>,
    },
    /// Voicemod connected again: what was asked before is lost
    Reconnected,
}

/// Where the window loads the pad's picture from, if it has one.
pub fn path(pad: &Pad) -> Option<String> {
    let checksum = pad
        .image_url()
        .rsplit('/')
        .next()
        .filter(|checksum| is_checksum(checksum))?;
    Some(match pad {
        Pad::Sound(sound) if is_id(&sound.id) => format!("sound/{}/{checksum}", sound.id),
        Pad::Voice(voice) if is_id(&voice.id) => format!("voice/{}/{checksum}", voice.id),
        _ => return None,
    })
}

impl Pictures {
    pub fn new(app: &AppHandle) -> Self {
        Self {
            folder: app.path().app_cache_dir().ok().map(|folder| folder.join("pictures")),
            jobs: OnceLock::new(),
        }
    }

    /// Starts asking Voicemod for the pictures the window wants.
    pub fn connect(&self, voicemod: Voicemod) {
        let (jobs, received) = mpsc::unbounded_channel();
        if self.jobs.set(jobs).is_ok() {
            let fetcher = Fetcher::new(voicemod, self.folder.clone());
            tauri::async_runtime::spawn(fetcher.run(received));
        }
    }

    /// Voicemod's answer to a picture asked for.
    pub fn received(&self, of: Picture, file: Option<Vec<u8>>) {
        self.send(Job::Received { of, file });
    }

    pub fn reconnected(&self) {
        self.send(Job::Reconnected);
    }

    fn send(&self, job: Job) {
        if let Some(jobs) = self.jobs.get() {
            let _ = jobs.send(job);
        }
    }

    async fn get(&self, of: Picture, checksum: String) -> Option<Vec<u8>> {
        if let Some(path) = file_path(self.folder.as_deref(), &of, &checksum) {
            let cached = tauri::async_runtime::spawn_blocking(move || fs::read(path)).await;
            if let Ok(Ok(file)) = cached {
                return Some(file);
            }
        }
        let (answer, answered) = oneshot::channel();
        let jobs = self.jobs.get()?;
        jobs.send(Job::Want { of, checksum, answer }).ok()?;
        tokio::time::timeout(WINDOW_WAIT, answered).await.ok()?.ok()?
    }
}

struct Fetcher {
    voicemod: Voicemod,
    folder: Option<PathBuf>,
    /// Not asked for yet, in the order the window wanted them
    queue: Vec<Wanted>,
    asked: Option<Asked>,
    /// None from Voicemod, or no answer, until it connects again: not asked for on every visit
    missing: HashSet<(Picture, String)>,
    /// Given up on, still kept if Voicemod answers late: their checksums
    late: HashMap<Picture, String>,
}

struct Wanted {
    of: Picture,
    checksum: String,
    at: Instant,
    waiting: Vec<Answer>,
}

struct Asked {
    wanted: Wanted,
    at: Instant,
}

impl Wanted {
    fn answer(self, file: Option<Vec<u8>>) {
        for answer in self.waiting {
            let _ = answer.send(file.clone());
        }
    }
}

impl Fetcher {
    fn new(voicemod: Voicemod, folder: Option<PathBuf>) -> Self {
        Self {
            voicemod,
            folder,
            queue: Vec::new(),
            asked: None,
            missing: HashSet::new(),
            late: HashMap::new(),
        }
    }

    async fn run(mut self, mut jobs: UnboundedReceiver<Job>) {
        loop {
            let deadline = self
                .asked
                .as_ref()
                .map_or_else(Instant::now, |asked| asked.at + ANSWER_WAIT);
            tokio::select! {
                job = jobs.recv() => match job {
                    Some(Job::Want { of, checksum, answer }) => self.want(of, checksum, answer, Instant::now()),
                    Some(Job::Received { of, file }) => self.received(&of, file),
                    Some(Job::Reconnected) => self.reconnected(),
                    None => return,
                },
                () = tokio::time::sleep_until(deadline), if self.asked.is_some() => self.unanswered(),
            }
            self.ask_next(Instant::now());
        }
    }

    fn want(&mut self, of: Picture, checksum: String, answer: Answer, now: Instant) {
        if self.missing.contains(&(of.clone(), checksum.clone())) {
            let _ = answer.send(None);
        } else if let Some(asked) = self.asked.as_mut().filter(|asked| asked.wanted.of == of) {
            asked.wanted.waiting.push(answer);
        } else if let Some(wanted) = self.queue.iter_mut().find(|wanted| wanted.of == of) {
            wanted.waiting.push(answer);
            wanted.at = now;
        } else {
            self.queue.push(Wanted {
                of,
                checksum,
                at: now,
                waiting: vec![answer],
            });
        }
    }

    fn received(&mut self, of: &Picture, file: Option<Vec<u8>>) {
        let file = file.filter(|file| content_type(file).is_some());
        if let Some(asked) = self.asked.take_if(|asked| asked.wanted.of == *of) {
            self.keep(of, &asked.wanted.checksum, file.as_ref());
            asked.wanted.answer(file);
        } else if let Some(checksum) = self.late.remove(of) {
            self.keep(of, &checksum, file.as_ref());
        }
    }

    /// Saves the picture, or remembers that there is none.
    fn keep(&mut self, of: &Picture, checksum: &str, file: Option<&Vec<u8>>) {
        let key = (of.clone(), checksum.to_string());
        let Some(file) = file else {
            self.missing.insert(key);
            return;
        };
        self.missing.remove(&key);
        if let Some(path) = file_path(self.folder.as_deref(), of, checksum) {
            let file = file.clone();
            tauri::async_runtime::spawn_blocking(move || {
                if let Err(error) = save(&path, &file) {
                    crate::errors::log(format!("cannot keep a picture: {error}"));
                }
            });
        }
    }

    fn reconnected(&mut self) {
        self.missing.clear();
        self.late.clear();
        if let Some(asked) = self.asked.take() {
            self.queue.insert(0, asked.wanted);
        }
    }

    fn unanswered(&mut self) {
        if let Some(asked) = self.asked.take() {
            let Wanted { of, checksum, .. } = &asked.wanted;
            self.missing.insert((of.clone(), checksum.clone()));
            self.late.insert(of.clone(), checksum.clone());
            asked.wanted.answer(None);
        }
    }

    /// Asks for the first picture of the page wanted last, unless one is being asked for already.
    fn ask_next(&mut self, now: Instant) {
        // A pad no longer shown stopped waiting
        self.queue.retain_mut(|wanted| {
            wanted.waiting.retain(|answer| !answer.is_closed());
            !wanted.waiting.is_empty()
        });
        if self.asked.is_some() {
            return;
        }
        let Some(newest) = self.queue.iter().map(|wanted| wanted.at).max() else {
            return;
        };
        let Some(next) = self.queue.iter().position(|wanted| wanted.at + SAME_PAGE >= newest) else {
            return;
        };
        let wanted = self.queue.remove(next);
        self.voicemod.send(Request::GetPicture(wanted.of.clone()));
        self.asked = Some(Asked { wanted, at: now });
    }
}

fn file_path(folder: Option<&Path>, of: &Picture, checksum: &str) -> Option<PathBuf> {
    let kind = match of {
        Picture::Sound(_) => "sound",
        Picture::Voice(_) => "voice",
    };
    Some(folder?.join(format!("{kind}-{checksum}")))
}

/// Written aside first, so a half-written picture is never read.
fn save(path: &Path, file: &[u8]) -> std::io::Result<()> {
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)?;
    }
    let part = path.with_extension("part");
    fs::write(&part, file)?;
    fs::rename(part, path)
}

/// The answer to the window's request for a picture.
pub async fn respond(app: &AppHandle, path: &str) -> Response<Vec<u8>> {
    let file = match parse(path) {
        Some((of, checksum)) => app.state::<Pictures>().get(of, checksum).await,
        None => None,
    };
    let response = match file.and_then(|file| Some((content_type(&file)?, file))) {
        // A checksum names a single picture
        Some((kind, file)) => Response::builder()
            .header(header::CONTENT_TYPE, kind)
            .header(header::CACHE_CONTROL, "max-age=31536000, immutable")
            .body(file),
        // Asked again when the pad shows it again
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(header::CACHE_CONTROL, "no-store")
            .body(Vec::new()),
    };
    response.unwrap_or_default()
}

fn parse(path: &str) -> Option<(Picture, String)> {
    // The window's URL escapes the slashes
    let path = path.replace("%2F", "/").replace("%2f", "/");
    let mut parts = path.trim_start_matches('/').split('/');
    let (kind, id, checksum) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() || !is_id(id) || !is_checksum(checksum) {
        return None;
    }
    let of = match kind {
        "sound" => Picture::Sound(id.to_string()),
        "voice" => Picture::Voice(id.to_string()),
        _ => return None,
    };
    Some((of, checksum.to_string()))
}

fn is_id(text: &str) -> bool {
    (1..=64).contains(&text.len())
        && text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

fn is_checksum(text: &str) -> bool {
    (16..=128).contains(&text.len()) && text.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn content_type(file: &[u8]) -> Option<&'static str> {
    match file {
        [0x89, b'P', b'N', b'G', ..] => Some("image/png"),
        [0xFF, 0xD8, 0xFF, ..] => Some("image/jpeg"),
        [b'G', b'I', b'F', b'8', ..] => Some("image/gif"),
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => Some("image/webp"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::soundboard::tests::{sound, voice};

    const CHECKSUM: &str = "2118888560f7a5bd30ff4184e2c1e9ceea63cd0c04e9eb019afc59c90e4aa776";

    #[test]
    fn names_a_pad_picture_by_its_checksum() {
        let mut honk = sound("s1");
        honk.image_url = format!("https://content-storage.voicemod.net/{CHECKSUM}");
        assert_eq!(path(&Pad::Sound(honk)), Some(format!("sound/s1/{CHECKSUM}")));

        let mut dark = voice("v1");
        dark.image_url = format!("https://content-storage.voicemod.net/{CHECKSUM}");
        assert_eq!(path(&Pad::Voice(dark)), Some(format!("voice/v1/{CHECKSUM}")));

        let mut odd = sound("s2");
        odd.image_url = "https://example.com/honk.png".to_string();
        assert_eq!(path(&Pad::Sound(odd)), None);
        assert_eq!(path(&Pad::Sound(sound("s3"))), None);
    }

    #[test]
    fn reads_the_window_paths() {
        let picture = |path: &str| parse(path);
        assert_eq!(
            picture(&format!("/sound%2Fd7b84366-6adb%2F{CHECKSUM}")),
            Some((Picture::Sound("d7b84366-6adb".into()), CHECKSUM.into()))
        );
        assert_eq!(
            picture(&format!("/voice/v1/{CHECKSUM}")),
            Some((Picture::Voice("v1".into()), CHECKSUM.into()))
        );
        for path in [
            format!("/meme/v1/{CHECKSUM}"),
            format!("/sound/../{CHECKSUM}"),
            format!("/sound/v1/{CHECKSUM}/more"),
            "/sound/v1/..%2F..%2Fsettings.json".to_string(),
            "/sound/v1".to_string(),
        ] {
            assert_eq!(picture(&path), None, "{path}");
        }
    }

    #[test]
    fn knows_the_picture_formats() {
        assert_eq!(content_type(b"\x89PNG\r\n\x1a\n"), Some("image/png"));
        assert_eq!(content_type(b"\xff\xd8\xff\xe0\0\x10JFIF"), Some("image/jpeg"));
        assert_eq!(content_type(b"<svg"), None);
        assert_eq!(content_type(b""), None);
    }

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n";

    fn want(fetcher: &mut Fetcher, id: &str, now: Instant) -> oneshot::Receiver<Option<Vec<u8>>> {
        let (answer, answered) = oneshot::channel();
        fetcher.want(Picture::Sound(id.into()), CHECKSUM.into(), answer, now);
        answered
    }

    fn asked(sent: &mut UnboundedReceiver<Request>) -> Option<String> {
        match sent.try_recv() {
            Ok(Request::GetPicture(Picture::Sound(id))) => Some(id),
            _ => None,
        }
    }

    #[test]
    fn asks_for_one_picture_at_a_time() {
        let (voicemod, mut sent) = Voicemod::unconnected();
        let mut fetcher = Fetcher::new(voicemod, None);
        let now = Instant::now();
        let mut first = want(&mut fetcher, "a", now);
        let _second = want(&mut fetcher, "b", now);
        fetcher.ask_next(now);
        fetcher.ask_next(now);
        assert_eq!(asked(&mut sent), Some("a".into()));
        assert_eq!(asked(&mut sent), None);

        fetcher.received(&Picture::Sound("a".into()), Some(PNG.to_vec()));
        assert_eq!(first.try_recv(), Ok(Some(PNG.to_vec())));
        fetcher.ask_next(now);
        assert_eq!(asked(&mut sent), Some("b".into()));
    }

    #[test]
    fn asks_for_the_page_shown_last_first() {
        let (voicemod, mut sent) = Voicemod::unconnected();
        let mut fetcher = Fetcher::new(voicemod, None);
        let start = Instant::now();
        let later = start + Duration::from_secs(2);
        let _old = [want(&mut fetcher, "old1", start), want(&mut fetcher, "old2", start)];
        let _new = [want(&mut fetcher, "new1", later), want(&mut fetcher, "new2", later)];

        let mut order = Vec::new();
        for _ in 0..4 {
            fetcher.ask_next(later);
            let id = asked(&mut sent).expect("a picture asked for");
            fetcher.received(&Picture::Sound(id.clone()), None);
            order.push(id);
        }
        assert_eq!(order, ["new1", "new2", "old1", "old2"]);
    }

    #[test]
    fn asks_again_after_a_reconnection_and_drops_what_no_pad_waits_for() {
        let (voicemod, mut sent) = Voicemod::unconnected();
        let mut fetcher = Fetcher::new(voicemod, None);
        let now = Instant::now();
        let mut kept = want(&mut fetcher, "kept", now);
        drop(want(&mut fetcher, "gone", now));
        let _last = want(&mut fetcher, "last", now);

        fetcher.ask_next(now);
        fetcher.reconnected();
        fetcher.ask_next(now);
        fetcher.unanswered();
        fetcher.ask_next(now);
        assert_eq!(kept.try_recv(), Ok(None));
        let order: Vec<_> = std::iter::from_fn(|| asked(&mut sent)).collect();
        assert_eq!(order, ["kept", "kept", "last"]);
    }

    #[test]
    fn remembers_the_missing_pictures_until_voicemod_connects_again() {
        let (voicemod, mut sent) = Voicemod::unconnected();
        let mut fetcher = Fetcher::new(voicemod, None);
        let now = Instant::now();
        let unknown = Picture::Sound("unknown".into());
        let empty = Picture::Sound("empty".into());

        let _first = [want(&mut fetcher, "unknown", now), want(&mut fetcher, "empty", now)];
        fetcher.ask_next(now);
        fetcher.unanswered();
        fetcher.ask_next(now);
        fetcher.received(&empty, None);
        let mut again = [want(&mut fetcher, "unknown", now), want(&mut fetcher, "empty", now)];
        fetcher.ask_next(now);
        assert_eq!(
            again.each_mut().map(|answered| answered.try_recv()),
            [Ok(None), Ok(None)]
        );

        // A late answer still counts, and Voicemod connected again may have what it had not
        fetcher.received(&unknown, Some(PNG.to_vec()));
        let _late = want(&mut fetcher, "unknown", now);
        fetcher.ask_next(now);
        fetcher.received(&unknown, Some(PNG.to_vec()));
        fetcher.reconnected();
        let _after = want(&mut fetcher, "empty", now);
        fetcher.ask_next(now);

        let order: Vec<_> = std::iter::from_fn(|| asked(&mut sent)).collect();
        assert_eq!(order, ["unknown", "empty", "unknown", "empty"]);
    }
}
