use async_channel::{bounded, Receiver, Sender};
use async_net::TcpStream;
use asynchronous_codec::Framed;
use bevy::log::info;
use bevy::{prelude::*, tasks::IoTaskPool};
use futures::{FutureExt, SinkExt, StreamExt};
use std::io;

use crate::{
    conf,
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
pub struct SendMessage {
    pub msg: ClientMessage,
}

#[derive(Resource, Debug)]
pub struct ConnectionState {
    startup_messages: Option<Vec<ServerMessage>>,
    sender: Sender<ClientMessage>,
    receiver: Receiver<ServerMessage>,
}

pub fn connect(mut commands: Commands) {
    commands.trigger(Connect {
        character_id: 1,
        auth_token: "".to_string(),
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
    if connection.startup_messages.is_some() {
        while let Ok(msg) = connection.receiver.try_recv() {
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

pub(super) fn on_send_message(event: On<SendMessage>, connection: Option<Res<ConnectionState>>) {
    if connection.is_none() {
        return;
    }

    let connection = connection.unwrap();
    if let Err(e) = connection.sender.send_blocking(event.msg.clone()) {
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
                        info!("sending msg: {:?}", msg);
                        self.stream.send(msg).await?;
                    } else {
                        break;
                    }
                },
                msg = self.stream.next().fuse() => {
                    if let Some(msg) = msg {
                        if let Ok(msg) = msg {
                            info!("receiveing msg: {:?}", msg);
                            if self.sender.send(msg).await.is_err() {
                                break;
                            }
                        } else {
                            return Err(io::Error::new(io::ErrorKind::InvalidData, msg.err().unwrap()));
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
