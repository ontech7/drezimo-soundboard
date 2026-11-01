//! The link to the board over USB serial: finds it, keeps it connected and reports what it sends.

use crate::protocol::BoardMessage;
use serde::Serialize;
use serialport::{SerialPort, SerialPortType};
use std::collections::HashMap;
use std::io;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

// The board's USB identity, docs/project.md section 9. Shared with arduino-pico's generic RP2040 boards until the board
// gets its own product ID: the handshake tells them apart.
const USB_VID: u16 = 0x2E8A;
const USB_PID: u16 = 0xF00A;

const SEARCH_INTERVAL: Duration = Duration::from_secs(1);
/// How long a read waits for the board. Messages for the board go out between reads: one thread does both, because
/// Windows runs the reads and writes of a serial port one at a time anyway.
const READ_TIMEOUT: Duration = Duration::from_millis(25);
/// How long a write may wait for room in the USB buffers
const WRITE_TIMEOUT: Duration = Duration::from_secs(1);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(2);
const RETRY_AFTER: Duration = Duration::from_secs(3);
/// After a few unanswered handshakes the port is probably another device with the same ID. It is still tried now and
/// then, in case it is a board whose answer got lost.
const QUICK_RETRIES: u32 = 3;
const SLOW_RETRY_AFTER: Duration = Duration::from_secs(30);
/// How often an idle session checks that the board is still there
const PRESENCE_CHECK: Duration = Duration::from_secs(2);

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum BoardStatus {
    #[default]
    Searching,
    /// Plugged in, but another program holds the port
    Busy {
        port: String,
    },
    Connected {
        port: String,
        firmware: String,
    },
}

pub enum BoardEvent {
    Status(BoardStatus),
    Message(BoardMessage),
}

pub struct Board {
    commands: Sender<String>,
}

/// The board thread's side of the link, consumed by `start`.
pub struct BoardLink {
    outgoing: Receiver<String>,
}

impl Board {
    pub fn new() -> (Self, BoardLink) {
        let (commands, outgoing) = mpsc::channel();
        (Self { commands }, BoardLink { outgoing })
    }

    /// Sends a message to the board. Dropped if no board is connected.
    pub fn send(&self, message: impl Into<String>) {
        // The board thread lives as long as the app, so the channel never closes
        let _ = self.commands.send(message.into());
    }
}

#[derive(Default)]
struct PortAttempts {
    failed_handshakes: u32,
    busy: bool,
    retry_at: Option<Instant>,
}

impl BoardLink {
    /// Looks for the board in the background and keeps it connected; `on_event` runs on that thread.
    pub fn start(self, on_event: impl Fn(BoardEvent) + Send + 'static) {
        thread::spawn(move || self.search(&on_event));
    }

    fn search(&self, on_event: &impl Fn(BoardEvent)) {
        let mut attempts: HashMap<String, PortAttempts> = HashMap::new();
        let mut status = BoardStatus::Searching;
        loop {
            let ports = drezimo_ports();
            attempts.retain(|port, _| ports.contains(port));
            for port in &ports {
                let attempt = attempts.entry(port.clone()).or_default();
                if attempt.retry_at.is_some_and(|at| Instant::now() < at) {
                    continue;
                }
                match connect(port) {
                    Ok(connection) => {
                        *attempt = PortAttempts::default();
                        let connected = BoardStatus::Connected {
                            port: port.clone(),
                            firmware: connection.firmware.clone(),
                        };
                        report(&mut status, connected, on_event);
                        self.run(port, connection, on_event);
                        report(&mut status, idle_status(&attempts), on_event);
                    }
                    Err(error) => {
                        attempt.busy = matches!(error, ConnectError::Busy);
                        match error {
                            ConnectError::NoAnswer => attempt.failed_handshakes += 1,
                            ConnectError::Io(error) => eprintln!("cannot open the board on {port}: {error}"),
                            ConnectError::Busy => {}
                        }
                        let delay = if attempt.failed_handshakes < QUICK_RETRIES {
                            RETRY_AFTER
                        } else {
                            SLOW_RETRY_AFTER
                        };
                        attempt.retry_at = Some(Instant::now() + delay);
                    }
                }
            }
            report(&mut status, idle_status(&attempts), on_event);
            thread::sleep(SEARCH_INTERVAL);
        }
    }

    /// Runs a session with a connected board until it goes away.
    fn run(&self, path: &str, mut connection: Connection, on_event: &impl Fn(BoardEvent)) {
        // Messages queued while no board was connected are stale
        while self.outgoing.try_recv().is_ok() {}
        if let Err(error) = self.exchange(connection.port.as_mut(), &mut connection.lines, on_event) {
            eprintln!("board on {path} disconnected: {error}");
        }
    }

    fn exchange(
        &self,
        port: &mut dyn SerialPort,
        lines: &mut LineReader,
        on_event: &impl Fn(BoardEvent),
    ) -> io::Result<()> {
        let mut checked_at = Instant::now();
        loop {
            let received = lines.read(port)?;
            // Not every driver fails a read once the device is unplugged, but asking the port for its state does
            if received.is_empty() && checked_at.elapsed() >= PRESENCE_CHECK {
                port.bytes_to_read()?;
                checked_at = Instant::now();
            }
            for line in received {
                match BoardMessage::parse(&line) {
                    Some(message) => on_event(BoardEvent::Message(message)),
                    None => eprintln!("board sent an unknown line: {line:?}"),
                }
            }

            // Replies to what was just received go out at once, anything else within READ_TIMEOUT
            let pending: String = self.outgoing.try_iter().map(|message| message + "\n").collect();
            if !pending.is_empty() {
                port.set_timeout(WRITE_TIMEOUT)?;
                port.write_all(pending.as_bytes())?;
                port.set_timeout(READ_TIMEOUT)?;
            }
        }
    }
}

fn drezimo_ports() -> Vec<String> {
    serialport::available_ports()
        .unwrap_or_default()
        .into_iter()
        .filter(
            |port| matches!(&port.port_type, SerialPortType::UsbPort(usb) if usb.vid == USB_VID && usb.pid == USB_PID),
        )
        .map(|port| port.port_name)
        // macOS lists every device twice: /dev/cu.* is the one to open
        .filter(|name| !name.starts_with("/dev/tty."))
        .collect()
}

fn is_listed(path: &str) -> bool {
    serialport::available_ports().is_ok_and(|ports| ports.iter().any(|port| port.port_name == path))
}

/// The status while no board is connected: a port held by another program is worth telling the user about
fn idle_status(attempts: &HashMap<String, PortAttempts>) -> BoardStatus {
    match attempts.iter().find(|(_, attempt)| attempt.busy) {
        Some((port, _)) => BoardStatus::Busy { port: port.clone() },
        None => BoardStatus::Searching,
    }
}

fn report(status: &mut BoardStatus, new: BoardStatus, on_event: &impl Fn(BoardEvent)) {
    if *status != new {
        *status = new.clone();
        on_event(BoardEvent::Status(new));
    }
}

struct Connection {
    port: Box<dyn SerialPort>,
    lines: LineReader,
    firmware: String,
}

enum ConnectError {
    Busy,
    NoAnswer,
    Io(io::Error),
}

impl From<io::Error> for ConnectError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serialport::Error> for ConnectError {
    fn from(error: serialport::Error) -> Self {
        Self::Io(error.into())
    }
}

/// Opens `path` and checks that a Drezimo board answers.
fn connect(path: &str) -> Result<Connection, ConnectError> {
    let mut port = serialport::new(path, 115_200)
        .timeout(READ_TIMEOUT)
        .open()
        .map_err(|error| match error.kind() {
            // Also reported for a port that has just vanished: one still listed is held by another program
            serialport::ErrorKind::NoDevice if is_listed(path) => ConnectError::Busy,
            _ => error.into(),
        })?;
    // DTR on tells the board that the app is running
    port.write_data_terminal_ready(true)?;
    // The newline first ends any half line a broken session left in the board's buffer
    port.write_all(b"\nHELLO\n")?;

    let mut lines = LineReader::default();
    let deadline = Instant::now() + HANDSHAKE_TIMEOUT;
    while Instant::now() < deadline {
        for line in lines.read(port.as_mut())? {
            if let Some(BoardMessage::Hello { firmware }) = BoardMessage::parse(&line) {
                return Ok(Connection { port, lines, firmware });
            }
        }
    }
    Err(ConnectError::NoAnswer)
}

/// Collects the bytes from the port into whole lines.
#[derive(Default)]
struct LineReader {
    pending: Vec<u8>,
}

impl LineReader {
    /// Waits up to READ_TIMEOUT for data and returns the lines completed so far.
    fn read(&mut self, port: &mut dyn SerialPort) -> io::Result<Vec<String>> {
        let mut buffer = [0; 512];
        match port.read(&mut buffer) {
            // A readable port with nothing to read has been unplugged
            Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
            Ok(count) => self.pending.extend_from_slice(&buffer[..count]),
            Err(error) if error.kind() == io::ErrorKind::TimedOut => {}
            Err(error) => return Err(error),
        }

        let mut lines = Vec::new();
        while let Some(end) = self.pending.iter().position(|&byte| byte == b'\n') {
            let line: Vec<u8> = self.pending.drain(..=end).collect();
            lines.push(String::from_utf8_lossy(&line).trim().to_string());
        }
        Ok(lines)
    }
}
