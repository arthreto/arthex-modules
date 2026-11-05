//! Discord's local protocol: frames of a little-endian operation, a length and JSON.

use std::{
    fs::File,
    io::{self, Read, Write},
};

use serde_json::{Value, json};

const HANDSHAKE: u32 = 0;
const FRAME: u32 = 1;
const CLOSE: u32 = 2;
const PING: u32 = 3;
const PONG: u32 = 4;
const MAX_BODY: u32 = 1024 * 1024;

#[derive(Debug)]
pub enum Error {
    /// Discord is closed or stopped answering.
    Lost,
    /// What Discord answered, such as an unknown application.
    Refused(String),
}

impl From<io::Error> for Error {
    fn from(_: io::Error) -> Self {
        Self::Lost
    }
}

fn write_frame(stream: &mut impl Write, operation: u32, body: &Value) -> io::Result<()> {
    let body = serde_json::to_vec(body)?;
    let length = u32::try_from(body.len()).map_err(|_| io::ErrorKind::InvalidInput)?;
    let mut frame = Vec::with_capacity(body.len() + 8);
    frame.extend_from_slice(&operation.to_le_bytes());
    frame.extend_from_slice(&length.to_le_bytes());
    frame.extend_from_slice(&body);
    stream.write_all(&frame)?;
    stream.flush()
}

fn read_frame(stream: &mut impl Read) -> io::Result<(u32, Value)> {
    let mut operation = [0; 4];
    let mut length = [0; 4];
    stream.read_exact(&mut operation)?;
    stream.read_exact(&mut length)?;
    let length = u32::from_le_bytes(length);
    if length > MAX_BODY {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let mut body = vec![0; length as usize];
    stream.read_exact(&mut body)?;
    Ok((
        u32::from_le_bytes(operation),
        serde_json::from_slice(&body)?,
    ))
}

fn message_of(body: &Value) -> String {
    body.get("message")
        .or_else(|| body.pointer("/data/message"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

pub struct Connection<Stream> {
    stream: Stream,
    app_id: String,
    nonce: u64,
}

impl<Stream: Read + Write> Connection<Stream> {
    pub fn start(mut stream: Stream, app_id: &str) -> Result<Self, Error> {
        write_frame(
            &mut stream,
            HANDSHAKE,
            &json!({ "v": 1, "client_id": app_id }),
        )?;
        loop {
            let (operation, body) = read_frame(&mut stream)?;
            match operation {
                FRAME if body["evt"] == "READY" => {
                    return Ok(Self {
                        stream,
                        app_id: app_id.to_string(),
                        nonce: 0,
                    });
                }
                CLOSE => return Err(Error::Refused(message_of(&body))),
                PING => write_frame(&mut stream, PONG, &body)?,
                _ => {}
            }
        }
    }

    pub fn app_id(&self) -> &str {
        &self.app_id
    }

    /// Sets the presence, or clears it with `None`, and waits for Discord's answer.
    pub fn set_activity(&mut self, activity: Option<&Value>) -> Result<(), Error> {
        self.nonce += 1;
        let nonce = self.nonce.to_string();
        let request = json!({
            "cmd": "SET_ACTIVITY",
            "args": { "pid": std::process::id(), "activity": activity },
            "nonce": nonce,
        });
        write_frame(&mut self.stream, FRAME, &request)?;
        loop {
            let (operation, body) = read_frame(&mut self.stream)?;
            match operation {
                FRAME if body["nonce"] == nonce.as_str() => {
                    return if body["evt"] == "ERROR" {
                        Err(Error::Refused(message_of(&body)))
                    } else {
                        Ok(())
                    };
                }
                CLOSE => return Err(io::Error::from(io::ErrorKind::ConnectionAborted).into()),
                PING => write_frame(&mut self.stream, PONG, &body)?,
                _ => {}
            }
        }
    }

    /// Fails when Discord has closed.
    pub fn ping(&mut self) -> Result<(), Error> {
        write_frame(&mut self.stream, PING, &json!({}))?;
        loop {
            let (operation, body) = read_frame(&mut self.stream)?;
            match operation {
                PONG => return Ok(()),
                CLOSE => return Err(io::Error::from(io::ErrorKind::ConnectionAborted).into()),
                PING => write_frame(&mut self.stream, PONG, &body)?,
                _ => {}
            }
        }
    }
}

/// The first Discord that answers: each running client takes the next free number.
#[cfg(windows)]
pub fn open() -> io::Result<File> {
    let mut last = io::ErrorKind::NotFound.into();
    for number in 0..10 {
        match std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(format!(r"\\.\pipe\discord-ipc-{number}"))
        {
            Ok(pipe) => return Ok(pipe),
            Err(error) => last = error,
        }
    }
    Err(last)
}

#[cfg(not(windows))]
pub fn open() -> io::Result<File> {
    Err(io::ErrorKind::Unsupported.into())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    /// Reads what Discord would answer and records what the program writes.
    struct Fake {
        answers: Cursor<Vec<u8>>,
        written: Vec<u8>,
    }

    impl Fake {
        fn answering(frames: &[(u32, Value)]) -> Self {
            let mut answers = Vec::new();
            for (operation, body) in frames {
                write_frame(&mut answers, *operation, body).unwrap();
            }
            Self {
                answers: Cursor::new(answers),
                written: Vec::new(),
            }
        }

        fn sent(&self) -> Vec<(u32, Value)> {
            let mut written = Cursor::new(self.written.clone());
            let mut frames = Vec::new();
            while written.position() < self.written.len() as u64 {
                frames.push(read_frame(&mut written).unwrap());
            }
            frames
        }
    }

    impl Read for Fake {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            self.answers.read(buffer)
        }
    }

    impl Write for Fake {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.written.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn ready() -> (u32, Value) {
        (FRAME, json!({ "cmd": "DISPATCH", "evt": "READY" }))
    }

    #[test]
    fn frames_are_operation_length_then_json() {
        let mut bytes = Vec::new();
        write_frame(&mut bytes, FRAME, &json!({ "a": 1 })).unwrap();
        assert_eq!(&bytes[..4], &[1, 0, 0, 0]);
        assert_eq!(&bytes[4..8], &[7, 0, 0, 0]);
        assert_eq!(&bytes[8..], br#"{"a":1}"#);
        assert_eq!(
            read_frame(&mut Cursor::new(bytes)).unwrap(),
            (FRAME, json!({ "a": 1 }))
        );
    }

    #[test]
    fn refuses_a_body_too_long() {
        let mut bytes = FRAME.to_le_bytes().to_vec();
        bytes.extend_from_slice(&(MAX_BODY + 1).to_le_bytes());
        assert!(read_frame(&mut Cursor::new(bytes)).is_err());
    }

    #[test]
    fn says_who_it_is_then_waits_for_ready() {
        let connection = Connection::start(Fake::answering(&[ready()]), "123").unwrap();
        assert_eq!(connection.app_id(), "123");
        assert_eq!(
            connection.stream.sent(),
            [(HANDSHAKE, json!({ "v": 1, "client_id": "123" }))]
        );
    }

    #[test]
    fn reports_an_application_discord_refuses() {
        let closed = (
            CLOSE,
            json!({ "code": 4000, "message": "Invalid Client ID" }),
        );
        match Connection::start(Fake::answering(&[closed]), "1") {
            Err(Error::Refused(message)) => assert_eq!(message, "Invalid Client ID"),
            _ => panic!("expected a refusal"),
        }
    }

    #[test]
    fn matches_the_answer_to_its_request() {
        let answers = [
            ready(),
            (FRAME, json!({ "nonce": "other" })),
            (FRAME, json!({ "evt": null, "nonce": "1" })),
        ];
        let mut connection = Connection::start(Fake::answering(&answers), "1").unwrap();
        let activity = json!({ "type": 0 });
        connection.set_activity(Some(&activity)).unwrap();
        let sent = connection.stream.sent();
        let (operation, request) = &sent[1];
        assert_eq!(*operation, FRAME);
        assert_eq!(request["cmd"], "SET_ACTIVITY");
        assert_eq!(request["args"]["activity"], activity);
        assert_eq!(request["nonce"], "1");
    }

    #[test]
    fn clears_with_a_null_activity() {
        let answers = [ready(), (FRAME, json!({ "nonce": "1" }))];
        let mut connection = Connection::start(Fake::answering(&answers), "1").unwrap();
        connection.set_activity(None).unwrap();
        assert_eq!(
            connection.stream.sent()[1].1["args"]["activity"],
            Value::Null
        );
    }

    #[test]
    fn reports_what_discord_refuses_in_an_activity() {
        let answers = [
            ready(),
            (
                FRAME,
                json!({ "evt": "ERROR", "nonce": "1", "data": { "message": "Bad" } }),
            ),
        ];
        let mut connection = Connection::start(Fake::answering(&answers), "1").unwrap();
        match connection.set_activity(None) {
            Err(Error::Refused(message)) => assert_eq!(message, "Bad"),
            _ => panic!("expected a refusal"),
        }
    }

    #[test]
    fn answers_a_ping_while_waiting() {
        let answers = [
            ready(),
            (PING, json!({ "n": 1 })),
            (FRAME, json!({ "nonce": "1" })),
        ];
        let mut connection = Connection::start(Fake::answering(&answers), "1").unwrap();
        connection.set_activity(None).unwrap();
        assert_eq!(connection.stream.sent()[2], (PONG, json!({ "n": 1 })));
    }

    #[test]
    fn notices_that_discord_has_closed() {
        let mut connection = Connection::start(Fake::answering(&[ready()]), "1").unwrap();
        assert!(matches!(connection.ping(), Err(Error::Lost)));
    }
}
