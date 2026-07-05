use async_channel::{Receiver, Sender, bounded};
use async_net::TcpStream;
use asynchronous_codec::Framed;
use bevy::log::info;
use bevy::{prelude::*, tasks::IoTaskPool};
use futures::{FutureExt, SinkExt, StreamExt};
use std::io;

use crate::{
    conf,
    core::GameState,
    network::{
        events,
        messages::{ClientMessage, GameMessageCodec, ServerMessage},
    },
};

#[derive(Event, Debug)]
pub struct Connect {
    pub character_id: u32,
    pub auth_token: String,
}

#[derive(Event, Debug)]
pub struct SendMessage(pub ClientMessage);

/// Credentials the login screen hands to `connect()`. Currently always the
/// fake test values; becomes real data when the server gains accounts.
#[derive(Resource, Debug)]
pub struct LoginCredentials {
    pub character_id: u32,
    pub auth_token: String,
}

#[derive(Resource, Debug)]
pub struct ConnectionState {
    startup_messages: Option<Vec<ServerMessage>>,
    sender: Sender<ClientMessage>,
    receiver: Receiver<ServerMessage>,
}

/// Runs on `OnEnter(GameState::Connecting)`. `Connecting` is only reachable
/// via the character list's confirm handler, which inserts
/// [`LoginCredentials`] in the same command batch — so the `Res` here is
/// guaranteed present. Keep that invariant if adding new paths into
/// `Connecting` (e.g. a reconnect feature).
pub fn connect(mut commands: Commands, credentials: Res<LoginCredentials>) {
    commands.trigger(Connect {
        character_id: credentials.character_id,
        auth_token: credentials.auth_token.clone(),
    });
}

pub(super) fn on_connect(event: On<Connect>, mut commands: Commands) {
    let (cli_send, cli_recv) = bounded(5);
    let (srv_send, srv_recv) = bounded(5);

    IoTaskPool::get()
        .spawn(async move {
            let conn =
                PersistentConnection::new(conf::server::SERVER_ADDRESS, srv_send, cli_recv).await;
            if let Ok(conn) = conn {
                return conn.run().await;
            }
            Err(conn.err().unwrap())
        })
        .detach();

    if cli_send
        .send_blocking(ClientMessage::Login {
            character_id: event.character_id,
            auth_token: event.auth_token.clone(),
        })
        .is_err()
    {
        error!("Connection failed");
    };

    commands.insert_resource(ConnectionState {
        startup_messages: Some(Vec::new()),
        sender: cli_send,
        receiver: srv_recv,
    });
}

pub(super) fn receive_messages(mut commands: Commands, mut connection: ResMut<ConnectionState>) {
    // The async task drops its sender when the TCP connection fails or
    // closes. Without this check the client used to hang in Connecting
    // forever when the server was unreachable.
    if connection.receiver.is_closed() && connection.receiver.is_empty() {
        commands.trigger(events::ConnectionLost);
        return;
    }

    if connection.startup_messages.is_some() {
        while let Ok(msg) = connection.receiver.try_recv() {
            // A login rejection must route immediately — buffering it until
            // DescribePlayer would swallow it forever (DescribePlayer never
            // comes after a rejection).
            if matches!(msg, ServerMessage::LoginError) {
                events::route_event(msg, &mut commands);
                return;
            }
            if let ServerMessage::DescribePlayer { .. } = msg {
                events::route_event(msg, &mut commands);
                while let Some(start_msg) = connection.startup_messages.as_mut().unwrap().pop() {
                    events::route_event(start_msg, &mut commands);
                }
                connection.startup_messages = None;
                return;
            }
            connection.startup_messages.as_mut().unwrap().push(msg);
        }
        return;
    }
    while let Ok(msg) = connection.receiver.try_recv() {
        events::route_event(msg, &mut commands);
    }
}

/// Dropping ConnectionState drops the client-message sender, which ends
/// the async task's select loop and closes the TCP stream.
pub(super) fn on_login_error_cleanup(
    _: On<events::LoginError>,
    state: Res<State<GameState>>,
    mut commands: Commands,
) {
    if *state.get() == GameState::Connecting {
        commands.remove_resource::<ConnectionState>();
    }
}

/// Not gated on `Connecting`: an in-game connection drop must also remove
/// the resource, or `receive_messages` re-triggers `ConnectionLost` every
/// frame forever. (`on_send_message` already tolerates the missing
/// resource; in-game reconnect UI is future work.)
pub(super) fn on_connection_lost_cleanup(_: On<events::ConnectionLost>, mut commands: Commands) {
    commands.remove_resource::<ConnectionState>();
}

pub(super) fn on_send_message(event: On<SendMessage>, connection: Option<Res<ConnectionState>>) {
    if connection.is_none() {
        return;
    }

    let connection = connection.unwrap();
    if let Err(e) = connection.sender.send_blocking(event.0.clone()) {
        error!("Error sending client message: {e}");
    };
}

pub struct PersistentConnection {
    stream: Framed<TcpStream, GameMessageCodec>,
    sender: Sender<ServerMessage>,
    receiver: Receiver<ClientMessage>,
}

impl PersistentConnection {
    pub async fn new(
        server_addr: &str,
        sender: Sender<ServerMessage>,
        receiver: Receiver<ClientMessage>,
    ) -> Result<Self, io::Error> {
        let stream = TcpStream::connect(server_addr).await?;
        let stream = Framed::new(stream, GameMessageCodec {});
        Ok(Self {
            stream,
            sender,
            receiver,
        })
    }

    pub async fn run(mut self) -> Result<(), io::Error> {
        loop {
            futures::select! {
                msg = self.receiver.recv().fuse() => {
                    if let Ok(msg) = msg {
                        if !matches!(msg, ClientMessage::Ping) {
                            info!("sending msg: {:?}", msg);
                        }
                        self.stream.send(msg).await?;
                    } else {
                        break;
                    }
                },
                msg = self.stream.next().fuse() => {
                    if let Some(msg) = msg {
                        if let Ok(msg) = msg {
                            if !matches!(msg, ServerMessage::Pong) {
                                info!("receiveing msg: {}", msg);
                            }
                            if self.sender.send(msg).await.is_err() {
                                break;
                            }
                        } else {
                            error!("Error reading from server: {}", msg.err().unwrap());
                            break;
                        }
                    } else {
                        break;
                    }
                }
            };
        }

        info!("loop ended");

        Ok(())
    }
}
