use anyhow::Result;
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast;
use tokio_tungstenite::{accept_async, tungstenite::protocol::Message};
use tracing::{error, info, warn};
use uuid::Uuid;
use hyper::{Request, Response, StatusCode};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use http_body_util::Full;
use hyper::body::Bytes;
use std::convert::Infallible;

// Player state
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Player {
    pub id: String,
    pub nickname: String,
    pub x: f32,
    pub y: f32,
    pub color: String,
    pub last_seen: u64,
}

impl Player {
    pub fn new(nickname: Option<String>) -> Self {
        let id = Uuid::new_v4().to_string();
        let nickname = nickname.unwrap_or_else(|| format!("Player{}", &id[..6]));
        let mut rng = thread_rng();
        let colors = ["#FF6B6B", "#4ECDC4", "#45B7D1", "#96CEB4", "#FECA57", "#FF9FF3"];
        let color = colors[rng.gen_range(0..colors.len())].to_string();
        
        Self {
            id,
            nickname,
            x: rng.gen_range(50.0..750.0),
            y: rng.gen_range(50.0..350.0),
            color,
            last_seen: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        }
    }
}

// Client -> Server messages
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ClientMessage {
    Join { nickname: Option<String> },
    Move { x: f32, y: f32 },
    Chat { message: String },
    ChangeNick { nickname: String },
}

// Server -> Client messages
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum ServerMessage {
    Welcome { 
        your_id: String, 
        players: Vec<Player> 
    },
    PlayerJoined { player: Player },
    PlayerLeft { player_id: String },
    PlayerMoved { 
        player_id: String, 
        x: f32, 
        y: f32 
    },
    ChatMessage { 
        player_id: String, 
        nickname: String, 
        message: String, 
        timestamp: u64 
    },
    Error { message: String },
}

// Game server state
#[derive(Clone)]
pub struct GameServer {
    players: Arc<DashMap<String, Player>>,
    broadcast_tx: broadcast::Sender<ServerMessage>,
}

impl GameServer {
    pub fn new() -> Self {
        let (broadcast_tx, _) = broadcast::channel(1000);
        Self {
            players: Arc::new(DashMap::new()),
            broadcast_tx,
        }
    }

    pub fn add_player(&self, player: Player) -> Result<String> {
        let player_id = player.id.clone();
        let join_msg = ServerMessage::PlayerJoined { player: player.clone() };
        
        self.players.insert(player_id.clone(), player);
        self.broadcast_message(join_msg)?;
        
        Ok(player_id)
    }

    pub fn remove_player(&self, player_id: &str) -> Result<()> {
        if self.players.remove(player_id).is_some() {
            let leave_msg = ServerMessage::PlayerLeft { 
                player_id: player_id.to_string() 
            };
            self.broadcast_message(leave_msg)?;
        }
        Ok(())
    }

    pub fn move_player(&self, player_id: &str, x: f32, y: f32) -> Result<()> {
        let x = x.clamp(0.0, 800.0);
        let y = y.clamp(0.0, 400.0);

        if let Some(mut player) = self.players.get_mut(player_id) {
            player.x = x;
            player.y = y;
            player.last_seen = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

            let move_msg = ServerMessage::PlayerMoved {
                player_id: player_id.to_string(),
                x,
                y,
            };
            self.broadcast_message(move_msg)?;
        }
        Ok(())
    }

    pub fn send_chat(&self, player_id: &str, message: String) -> Result<()> {
        if let Some(player) = self.players.get(player_id) {
            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

            let chat_msg = ServerMessage::ChatMessage {
                player_id: player_id.to_string(),
                nickname: player.nickname.clone(),
                message,
                timestamp,
            };

            self.broadcast_message(chat_msg)?;
        }
        Ok(())
    }

    pub fn get_welcome_message(&self, player_id: &str) -> ServerMessage {
        let players: Vec<Player> = self.players.iter().map(|p| p.value().clone()).collect();
        ServerMessage::Welcome {
            your_id: player_id.to_string(),
            players,
        }
    }

    pub fn broadcast_message(&self, message: ServerMessage) -> Result<()> {
        let _ = self.broadcast_tx.send(message);
        Ok(())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ServerMessage> {
        self.broadcast_tx.subscribe()
    }
}

async fn handle_websocket(
    stream: tokio::net::TcpStream,
    addr: SocketAddr,
    server: GameServer,
) -> Result<()> {
    info!("WebSocket connection from: {}", addr);
    
    let ws_stream = accept_async(stream).await?;
    let (ws_sender, mut ws_receiver) = ws_stream.split();
    
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Message>();
    let mut broadcast_rx = server.subscribe();
    let mut player_id: Option<String> = None;
    
    // Handle incoming messages
    let server_clone = server.clone();
    let tx_clone = tx.clone();
    let incoming_task = tokio::spawn(async move {
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                        match client_msg {
                            ClientMessage::Join { nickname } => {
                                let player = Player::new(nickname);
                                match server_clone.add_player(player.clone()) {
                                    Ok(pid) => {
                                        player_id = Some(pid.clone());
                                        let welcome = server_clone.get_welcome_message(&pid);
                                        let welcome_json = serde_json::to_string(&welcome).unwrap();
                                        if let Err(e) = tx_clone.send(Message::Text(welcome_json)) {
                                            error!("Failed to send welcome: {}", e);
                                            break;
                                        }
                                        info!("Player {} joined as {}", pid, player.nickname);
                                    }
                                    Err(e) => error!("Failed to add player: {}", e),
                                }
                            }
                            ClientMessage::Move { x, y } => {
                                if let Some(ref pid) = player_id {
                                    if let Err(e) = server_clone.move_player(pid, x, y) {
                                        error!("Failed to move player: {}", e);
                                    }
                                }
                            }
                            ClientMessage::Chat { message } => {
                                if let Some(ref pid) = player_id {
                                    if let Err(e) = server_clone.send_chat(pid, message) {
                                        error!("Failed to send chat: {}", e);
                                    }
                                }
                            }
                            ClientMessage::ChangeNick { nickname } => {
                                if let Some(ref pid) = player_id {
                                    if let Some(mut player) = server_clone.players.get_mut(pid) {
                                        player.nickname = nickname;
                                        info!("Player {} changed nickname to {}", pid, player.nickname);
                                    }
                                }
                            }
                        }
                    } else {
                        warn!("Invalid message format: {}", text);
                    }
                }
                Ok(Message::Close(_)) => {
                    info!("WebSocket closed by client");
                    break;
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }

        // Clean up player when connection closes
        if let Some(pid) = player_id {
            if let Err(e) = server_clone.remove_player(&pid) {
                error!("Failed to remove player: {}", e);
            } else {
                info!("Player {} disconnected", pid);
            }
        }
    });

    // Handle outgoing messages
    let outgoing_task = tokio::spawn(async move {
        let mut ws_sender = ws_sender;
        loop {
            tokio::select! {
                // Send broadcast messages
                server_msg = broadcast_rx.recv() => {
                    match server_msg {
                        Ok(msg) => {
                            let json = serde_json::to_string(&msg).unwrap();
                            if let Err(e) = ws_sender.send(Message::Text(json)).await {
                                error!("Failed to send broadcast message: {}", e);
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                // Send direct messages
                direct_msg = rx.recv() => {
                    match direct_msg {
                        Some(msg) => {
                            if let Err(e) = ws_sender.send(msg).await {
                                error!("Failed to send direct message: {}", e);
                                break;
                            }
                        }
                        None => break,
                    }
                }
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = incoming_task => {},
        _ = outgoing_task => {},
    }

    info!("WebSocket connection {} closed", addr);
    Ok(())
}

async fn handle_http_request(
    req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let static_path = std::env::var("STATIC_PATH").unwrap_or_else(|_| "dist".to_string());
    
    // Create a path for the requested file
    let path = req.uri().path();
    let file_path = if path == "/" {
        format!("{}/index.html", static_path)
    } else {
        format!("{}{}", static_path, path)
    };

    // Try to read the file
    match tokio::fs::read(&file_path).await {
        Ok(contents) => {
            let content_type = match std::path::Path::new(&file_path).extension() {
                Some(ext) => match ext.to_str() {
                    Some("html") => "text/html",
                    Some("css") => "text/css", 
                    Some("js") => "application/javascript",
                    Some("wasm") => "application/wasm",
                    Some("json") => "application/json",
                    _ => "application/octet-stream",
                },
                None => "text/html",
            };

            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("content-type", content_type)
                .header("access-control-allow-origin", "*")
                .body(Full::new(Bytes::from(contents)))
                .unwrap())
        }
        Err(_) => {
            // If file not found, serve index.html for SPA routing
            let index_path = format!("{}/index.html", static_path);
            let index_content = tokio::fs::read(index_path).await
                .unwrap_or_else(|_| b"<h1>Error: Frontend not built. Run 'npm run build' first.</h1>".to_vec());
            
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "text/html")
                .header("access-control-allow-origin", "*")
                .body(Full::new(Bytes::from(index_content)))
                .unwrap())
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let server = GameServer::new();
    info!("🎮 Rust Monolith Server starting...");

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .unwrap_or(8080);
    
    let ws_port = port + 1; // WebSocket on port + 1

    // Start WebSocket server
    let ws_server = server.clone();
    let ws_task = tokio::spawn(async move {
        let ws_addr: SocketAddr = ([0, 0, 0, 0], ws_port).into();
        let ws_listener = tokio::net::TcpListener::bind(ws_addr).await.unwrap();
        info!("🔌 WebSocket server listening on ws://0.0.0.0:{}", ws_port);

        while let Ok((stream, addr)) = ws_listener.accept().await {
            let server_clone = ws_server.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_websocket(stream, addr, server_clone).await {
                    error!("WebSocket handler error: {}", e);
                }
            });
        }
    });

    // Start HTTP static file server
    let http_task = tokio::spawn(async move {
        let http_addr: SocketAddr = ([0, 0, 0, 0], port).into();
        let http_listener = tokio::net::TcpListener::bind(http_addr).await.unwrap();
        info!("🌐 HTTP server listening on http://0.0.0.0:{}", port);
        info!("📁 Serving static files from ./dist/");

        while let Ok((tcp, _)) = http_listener.accept().await {
            let io = TokioIo::new(tcp);
            
            tokio::task::spawn(async move {
                let service = service_fn(handle_http_request);
                
                if let Err(err) = http1::Builder::new()
                    .serve_connection(io, service)
                    .await
                {
                    error!("Error serving HTTP connection: {}", err);
                }
            });
        }
    });

    info!("✅ Server ready!");
    info!("🎯 Frontend: http://0.0.0.0:{}", port);
    info!("🔌 WebSocket: ws://0.0.0.0:{}", ws_port);

    // Wait for both servers
    tokio::select! {
        _ = ws_task => error!("WebSocket server stopped"),
        _ = http_task => error!("HTTP server stopped"),
    }

    Ok(())
} 