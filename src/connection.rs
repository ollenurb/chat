use anyhow::{anyhow, Result};
use bytes::Buf;
use mio::net::TcpStream;
use std::{
    io::{Cursor, Read, Write},
    net::SocketAddr,
};

// Represents a message that has been received from the client
pub enum Message {
    Request(String),
    Message(String),
    Close,
}

// A connection with a client. It has an id that represents the socket resource on the host system,
// a username (if registered) and a stream. For simplicity reasons, the server doesn't drop the
// connection after some time if the user does not request register, but it keeps it open.
pub struct Connection {
    pub id: usize,
    stream: TcpStream,
    pub username: Option<String>,
}

impl Connection {
    const BUF_SIZE: usize = 512;

    pub fn new(id: usize, stream: TcpStream) -> Self {
        Connection {
            id,
            stream,
            username: None,
        }
    }

    // Handle a READ event from the Socket
    pub fn handle_read(&mut self) -> Result<Message> {
        let mut buffer = [0; Self::BUF_SIZE];

        loop {
            // We need to buffer manually everything and keep the buffer's ownership to the
            // connection
            let read_bytes = self.stream.read(&mut buffer)?;

            //  If we read 0 bytes, we received a signal that the connection is closed, then we
            //  return so that the event loop can close the connection
            if read_bytes == 0 {
                return Ok(Message::Close);
            }

            // If we reach this part, we can try to decode the frame
            let mut cursor = Cursor::new(buffer.as_slice());

            // Parse the frame
            let parsed_result = Connection::parse_message(&mut cursor);

            // Check protocol
            match parsed_result {
                // We got a request to enter
                Ok(Message::Request(ref r)) => {
                    if self.username.is_none() {
                        self.username = Some(r.to_string());
                    } else {
                        return Err(anyhow!("Protocol error, expected Message"));
                    }
                }
                Ok(Message::Message(_)) => {
                    if self.username.is_none() {
                        return Err(anyhow!("Protocol error, expected Request"));
                    }
                }
                _ => (),
            }

            return parsed_result;
        }
    }

    pub fn try_write(&mut self, message: &str) -> Result<()> {
        Ok(self.stream.write_all(message.as_bytes())?)
    }

    // Read until carriage return
    fn read_until_clrf<'a>(cursor: &mut Cursor<&'a [u8]>) -> Result<&'a str> {
        let start = cursor.position() as usize;
        let buffer = *cursor.get_ref();
        let mut end = start;
        // Iterate over chunks of length 2
        for chunk in buffer[start..].windows(2) {
            if chunk[0] == b'\r' && chunk[1] == b'\n' {
                cursor.set_position((end + 2) as u64);
                return Ok(std::str::from_utf8(&buffer[start..end])?);
            }
            end += 1;
        }
        Err(anyhow!("CRLF character not found."))
    }

    // Try to parse the message
    pub fn parse_message(cursor: &mut Cursor<&[u8]>) -> Result<Message> {
        let header = cursor.get_u8();
        let content = Self::read_until_clrf(cursor)?.to_string();

        match header {
            b'$' => Ok(Message::Request(content)),
            b'#' => Ok(Message::Message(content)),
            h => Err(anyhow!("Unexpected header byte '{}'.", h)),
        }
    }

    pub fn get_addr(&self) -> Result<SocketAddr> {
        self.stream.peer_addr().map_err(Into::into)
    }
}
