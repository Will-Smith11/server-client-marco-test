use auto_server_common::{
    client_config::ClientConfig,
    server_config::{self, ServerConfig},
};
use auto_server_macros::client_server;
use futures::stream::select_all;
use futures::Future;
use std::task::Poll;
use std::time::Duration;
use tokio::time::{interval, Interval};

use futures::{SinkExt, StreamExt};
use std::{net::SocketAddr, pin::Pin};

client_server! {

    #[client(TestClient)]
    pub enum ClientReqests {
    }

    #[server(TestServer)]
    pub enum ServerMsg {
        Bytes(Vec<u8>)
    }
}

pub struct Server {
    server: TestServer,
    dummy_payload: Vec<u8>,
    send_interval: Interval,
}

impl Server {
    pub fn new(server: TestServer) -> Self {
        let inteval = interval(Duration::from_millis(100));
        let dummy_payload = vec![69u8; 1024];
        Self {
            server,
            dummy_payload,
            send_interval: inteval,
        }
    }
}

impl Future for Server {
    type Output = ();
    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if let Poll::Ready(_) = self.send_interval.poll_tick(cx) {
            println!("sending dummy payload");
            let msg = self.dummy_payload.clone();
            self.server.send_all(ServerMsg::Bytes(msg));
        }

        Poll::Pending
    }
}

#[tokio::main]
async fn main() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:4201".parse::<SocketAddr>().unwrap())
        .await
        .unwrap();

    let server_config = ServerConfig {
        listener,
        timeout: Duration::from_secs(30),
    };

    let server_handle = tokio::spawn(async move {
        let server = TestServer::new(server_config);
        Server::new(server).await;
    });

    let mut clients = Vec::new();
    for _ in 0..10 {
        println!("client connected");
        // no clone on interval so do it this way lol
        let client_config = ClientConfig {
            ping_interval: interval(Duration::from_secs(10)),
            addr: "ws://127.0.0.1:4201".to_owned(),
        };
        let client = TestClient::new(client_config).await;
        clients.push(client);
    }

    let mut client_handles = select_all(clients);

    tokio::select! {
        _ = server_handle => {}
        _ = client_handles.next() => {
            println!("got dummy payload"); }
    }
}
