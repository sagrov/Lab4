use std::io::{self, Write};
use std::collections::HashMap;
use std::sync::{Arc};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, WebSocketStream};
use futures::{StreamExt, SinkExt};
use serde::{Serialize, Deserialize};
use tokio::sync::broadcast::{self, Sender, Receiver};
use tokio::sync::Mutex;

#[derive(Clone, Serialize, Deserialize)]
struct Message {
    from: String,
    content: String,
    timestamp: i64,
}

struct ChatServer {
    messages: Arc<Mutex<Vec<Message>>>,
    users: Arc<Mutex<HashMap<String, String>>>, // username -> password
    tx: Sender<Message>,
}

impl ChatServer {
    fn new() -> Self {
        let (tx, _): (Sender<Message>, Receiver<Message>) = broadcast::channel(100);
        ChatServer {
            messages: Arc::new(Mutex::new(Vec::new())),
            users: Arc::new(Mutex::new(HashMap::new())),
            tx,
        }
    }

    async fn register_user(&self, username: String, password: String) -> Result<(), String> {
        let mut users = self.users.lock().await;
        if users.contains_key(&username) {
            return Err("Username already exists".to_string());
        }
        users.insert(username, password);
        Ok(())
    }

    async fn authenticate_user(&self, username: &str, password: &str) -> bool {
        let users = self.users.lock().await;
        match users.get(username) {
            Some(stored_password) => stored_password == password,
            None => false,
        }
    }

    async fn broadcast_message(&self, message: Message) {
        let mut messages = self.messages.lock().await;
        messages.push(message.clone());
        if let Ok(json) = serde_json::to_string(&message) {
            println!("Broadcasting message: {}", json);
            let _ = self.tx.send(message);
        }
    }

    async fn handle_connection(&self, stream: TcpStream) {
        let ws_stream = accept_async(stream).await.unwrap();
        let (mut write, mut read) = ws_stream.split();
        let mut rx = self.tx.subscribe();

        // Handle authentication
        if let Some(msg) = read.next().await {
            let msg = msg.unwrap().to_string();
            let auth: serde_json::Value = serde_json::from_str(&msg).unwrap();

            match auth["type"].as_str() {
                Some("register") => {
                    let username = auth["username"].as_str().unwrap().to_string();
                    let password = auth["password"].as_str().unwrap().to_string();

                    match self.register_user(username.clone(), password).await {
                        Ok(()) => {
                            let _ = write.send("Registration successful".into()).await;
                        }
                        Err(e) => {
                            let _ = write.send(format!("Registration failed: {}", e).into()).await;
                            return;
                        }
                    }
                }
                Some("login") => {
                    let username = auth["username"].as_str().unwrap();
                    let password = auth["password"].as_str().unwrap();

                    if !self.authenticate_user(username, password).await {
                        let _ = write.send("Authentication failed".into()).await;
                        return;
                    }
                    let _ = write.send("Authentication successful".into()).await;
                }
                _ => {
                    let _ = write.send("Invalid authentication type".into()).await;
                    return;
                }
            }
        }

        // Send message history
        let messages = self.messages.lock().await;
        for msg in messages.iter() {
            if let Ok(json) = serde_json::to_string(&msg) {
                println!("Sending history message: {}", &json);
                let _ = write.send(json.into()).await;
            }
        }
        drop(messages);

        // Handle messages
        loop {
            tokio::select! {
                Some(msg) = read.next() => {
                    let msg = msg.unwrap();
                    println!("Received message: {}", msg.to_string());
                    match serde_json::from_str::<Message>(&msg.to_string()) {
                        Ok(message) => {
                            self.broadcast_message(message).await;
                        }
                        Err(e) => {
                            println!("Error parsing message: {}", e);
                        }
                    }
                }
                Ok(msg) = rx.recv() => {
                    match serde_json::to_string(&msg) {
                        Ok(json) => {
                            println!("Broadcasting: {}", &json);
                            let _ = write.send(json.into()).await;
                        }
                        Err(e) => {
                            println!("Error serializing message: {}", e);
                        }
                    }
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let server = Arc::new(ChatServer::new());
    let listener = TcpListener::bind("127.0.0.1:8100").await.unwrap();

    println!("WebSocket server listening on ws://127.0.0.1:8100");

    while let Ok((stream, _)) = listener.accept().await {
        let server = server.clone();
        tokio::spawn(async move {
            server.handle_connection(stream).await;
        });
    }
}

