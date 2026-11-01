//! The link to the board over USB serial: finds it, keeps it connected and reports what it sends.

use crate::protocol::BoardMessage;
use serialport::{SerialPort, SerialPortType};
use std::collections::HashSet;
use std::io;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

const RASPBERRY_PI_VID: u16 = 0x2E8A;
const SEARCH_INTERVAL: Duration = Duration::from_secs(1);
const READ_TIMEOUT: Duration = Duration::from_millis(10);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(2);

pub enum BoardEvent {
    Connected { port: String, firmware: String },
    Message(BoardMessage),
    Disconnected,
}

pub struct Board {
    commands: Sender<String>,
}

impl Board {
    /// The board handle, and the receiving end of its messages to pass to `start`.
    pub fn new() -> (Self, Receiver<String>) {
        let (commands, outgoing) = mpsc::channel();
        (Self { commands }, outgoing)
    }

    /// Sends a message to the board. Dropped if no board is connected.
    pub fn send(&self, message: impl Into<String>) {
        // The board thread lives as long as the app, so the channel never closes
        let _ = self.commands.send(message.into());
    }
}

/// Looks for the board in the background and keeps it connected; `on_event` runs on that thread.
pub fn start(outgoing: Receiver<String>, on_event: impl Fn(BoardEvent) + Send + 'static) {
    thread::spawn(move || search(&outgoing, &on_event));
}

fn search(outgoing: &Receiver<String>, on_event: &impl Fn(BoardEvent)) {
    // Other Raspberry Pi boards are skipped until they are unplugged
    let mut not_drezimo = HashSet::new();
    loop {
        let ports = raspberry_pi_ports();
        not_drezimo.retain(|port| ports.contains(port));
        for port in &ports {
            if not_drezimo.contains(port) {
                continue;
            }
            match connect(port, outgoing, on_event) {
                Err(error) if error.kind() == io::ErrorKind::InvalidData => {
                    not_drezimo.insert(port.clone());
                }
                Err(error) => eprintln!("board on {port}: {error}"),
                Ok(()) => {}
            }
        }
        thread::sleep(SEARCH_INTERVAL);
    }
}

fn raspberry_pi_ports() -> Vec<String> {
    serialport::available_ports()
        .unwrap_or_default()
        .into_iter()
        .filter(|port| matches!(&port.port_type, SerialPortType::UsbPort(usb) if usb.vid == RASPBERRY_PI_VID))
        .map(|port| port.port_name)
        // macOS lists every device twice: /dev/cu.* is the one to open
        .filter(|name| !name.starts_with("/dev/tty."))
        .collect()
}

/// Runs a session with the board on `path` until it is unplugged.
fn connect(path: &str, outgoing: &Receiver<String>, on_event: &impl Fn(BoardEvent)) -> io::Result<()> {
    let mut port = serialport::new(path, 115_200).timeout(READ_TIMEOUT).open()?;
    // DTR on tells the board that the app is running
    port.write_data_terminal_ready(true)?;
    let mut lines = LineReader::default();
    let firmware = handshake(port.as_mut(), &mut lines)?;

    // Messages queued while no board was connected are stale
    while outgoing.try_recv().is_ok() {}
    on_event(BoardEvent::Connected { port: path.to_string(), firmware });
    let result = exchange(port.as_mut(), &mut lines, outgoing, on_event);
    on_event(BoardEvent::Disconnected);
    result
}

fn handshake(port: &mut dyn SerialPort, lines: &mut LineReader) -> io::Result<String> {
    port.write_all(b"HELLO\n")?;
    let deadline = Instant::now() + HANDSHAKE_TIMEOUT;
    while Instant::now() < deadline {
        for line in lines.read(port)? {
            if let Some(BoardMessage::Hello { firmware }) = BoardMessage::parse(&line) {
                return Ok(firmware);
            }
        }
    }
    Err(io::Error::new(io::ErrorKind::InvalidData, "no Drezimo answer"))
}

fn exchange(
    port: &mut dyn SerialPort,
    lines: &mut LineReader,
    outgoing: &Receiver<String>,
    on_event: &impl Fn(BoardEvent),
) -> io::Result<()> {
    loop {
        for line in lines.read(port)? {
            match BoardMessage::parse(&line) {
                Some(message) => on_event(BoardEvent::Message(message)),
                None => eprintln!("board sent an unknown line: {line:?}"),
            }
        }
        for message in outgoing.try_iter() {
            port.write_all(format!("{message}\n").as_bytes())?;
        }
    }
}

/// Collects the bytes from the port into whole lines.
#[derive(Default)]
struct LineReader {
    pending: Vec<u8>,
}

impl LineReader {
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
