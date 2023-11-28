use anyhow::{anyhow, Result};
use std::{io::ErrorKind, net::SocketAddr};

use crate::connection::{Connection, Message};
use mio::{event::Event, net::TcpListener, Events, Interest, Poll, Token};
use slab::Slab;

const MAX_CONNECTIONS: usize = 128;
// Select an "out of bound" index for the server token
const SOCKET: Token = Token(MAX_CONNECTIONS + 1);

pub struct Server {
    poll: Poll,
    event_queue: Events,
    server_socket: TcpListener,
    connections_pool: Slab<Connection>,
}

impl Server {
    pub fn new(addr: SocketAddr) -> Result<Self> {
        Ok(Server {
            poll: Poll::new()?,
            event_queue: Events::with_capacity(MAX_CONNECTIONS),
            server_socket: TcpListener::bind(addr)?,
            connections_pool: Slab::new(),
        })
    }

    pub fn accept_connection(&mut self) -> Result<()> {
        let (mut conn, addr) = self.server_socket.accept()?;
        println!("Client {} connected.", addr.to_string());
        // Add the connection to the pool
        let entry = self.connections_pool.vacant_entry();
        self.poll
            .registry()
            .register(&mut conn, Token(entry.key()), Interest::READABLE)?;
        let handler = Connection::new(entry.key(), conn);
        entry.insert(handler);
        Ok(())
    }

    pub fn run(&mut self) -> Result<()> {
        self.poll
            .registry()
            .register(&mut self.server_socket, SOCKET, Interest::READABLE)?;

        loop {
            // Block the until we get an event that we previously wanted to monitor
            self.poll.poll(&mut self.event_queue, None)?;

            let events: Vec<Event> = self.event_queue.iter().cloned().collect();

            for event in events {
                match event.token() {
                    // Create a new connection
                    SOCKET => {
                        self.accept_connection()?;
                    }
                    Token(id) => {
                        // Whenever we get a `WouldBlock` error we continue to process other events
                        if let Err(e) = self.handle_connection(event, id) {
                            let error = e.downcast::<std::io::Error>()?;
                            if error.kind() == ErrorKind::WouldBlock {
                                continue;
                            } else {
                                eprintln!("{}", error)
                            };
                        };
                    }
                }
            }
        }
    }

    pub fn handle_connection(&mut self, event: Event, id: usize) -> Result<()> {
        // Handle readable event from the connection
        if event.is_readable() {
            let connection = self.connections_pool.get_mut(id).ok_or(anyhow!(
                "Got an event from a connection that doesn't exists in the pool"
            ))?;
            let id: usize = connection.id;

            // This short circuits if he gets errors, so that the event loop either continue or
            // stops processing
            let connection_addr = connection.get_addr()?.to_string();
            let message = match connection.handle_read()? {
                Message::Request(username) => {
                    println!("{} -> {}", connection_addr, username);
                    format!("{} entered the chat.", username)
                },
                Message::Message(message) => {
                    println!("{}: {}", connection_addr, message);
                    format!("{}: {}", connection.username.clone().unwrap(), message)
                }
                Message::Close => {
                    let username = connection.username.clone().unwrap();
                    self.connections_pool.remove(id);
                    println!("Client {} disconnected.", connection_addr);
                    format!("{} left the chat.", username)
                }
            };

            // Forward the message to all other participants
            for (_, connection) in &mut self.connections_pool {
                connection.try_write(&message)?;
            }
        }
        Ok(())
    }
}
