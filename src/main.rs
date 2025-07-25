use anyhow::Result;
use dashmap::DashMap;
use rand::prelude::*;
use rcgen::{Certificate, CertificateParams, DistinguishedName};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::broadcast;
use tracing::{error, info, warn};
use uuid::Uuid;
use wtransport::endpoint::IncomingSession;
use wtransport::{ClientConfig, Endpoint, ServerConfig};

// Player state
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Player {
    pub id: String,
    pub nickname: String,
    pub x: f32,
    pub y: f32,
    pub color: String,
    pub last_seen: SystemTime,
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
            x: rng.gen_range(50.0..750.0), // Random starting position
            y: rng.gen_range(50.0..350.0),
            color,
            last_seen: SystemTime::now(),
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

// Chat message history
#[derive(Clone)]
pub struct ChatHistory {
    messages: Arc<std::sync::Mutex<Vec<ServerMessage>>>,
}

impl ChatHistory {
    pub fn new() -> Self {
        Self {
            messages: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    pub fn add_message(&self, msg: ServerMessage) {
        let mut messages = self.messages.lock().unwrap();
        messages.push(msg);
        if messages.len() > 50 {
            messages.remove(0);
        }
    }

    pub fn get_recent(&self) -> Vec<ServerMessage> {
        self.messages.lock().unwrap().clone()
    }
}

// Game server state
pub struct GameServer {
    players: DashMap<String, Player>,
    chat_history: ChatHistory,
    broadcast_tx: broadcast::Sender<ServerMessage>,
}

impl GameServer {
    pub fn new() -> Self {
        let (broadcast_tx, _) = broadcast::channel(1000);
        Self {
            players: DashMap::new(),
            chat_history: ChatHistory::new(),
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
        // Validate bounds
        let x = x.clamp(0.0, 800.0);
        let y = y.clamp(0.0, 400.0);

        if let Some(mut player) = self.players.get_mut(player_id) {
            player.x = x;
            player.y = y;
            player.last_seen = SystemTime::now();

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
            let timestamp = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            let chat_msg = ServerMessage::ChatMessage {
                player_id: player_id.to_string(),
                nickname: player.nickname.clone(),
                message,
                timestamp,
            };

            self.chat_history.add_message(chat_msg.clone());
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
        if let Err(_) = self.broadcast_tx.send(message) {
            // No receivers, that's fine
        }
        Ok(())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ServerMessage> {
        self.broadcast_tx.subscribe()
    }
}

async fn handle_session(session: IncomingSession, server: Arc<GameServer>) -> Result<()> {
    let session_request = session.await?;
    let connection = session_request.accept().await?;
    
    info!("✅ WebTransport session established");
    
    // Accept bidirectional stream for this client
    let stream = connection.accept_bi().await?;
    let mut broadcast_rx = server.subscribe();
    let mut player_id: Option<String> = None;

    // Send welcome message with current game state
    // TODO: Send existing players and chat history

    // Handle incoming messages
    let server_clone = server.clone();
    let (mut send_stream, mut recv_stream) = stream;
    
    // Incoming message handler
    let incoming_task = tokio::spawn(async move {
        loop {
            let mut buffer = vec![0u8; 1024];
            match recv_stream.read(&mut buffer).await {
                Ok(Some(bytes_read)) => {
                    buffer.truncate(bytes_read);
                    
                    // Try to parse as JSON
                    if let Ok(text) = String::from_utf8(buffer) {
                        if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                            match client_msg {
                                ClientMessage::Join { nickname } => {
                                    let player = Player::new(nickname);
                                    match server_clone.add_player(player) {
                                        Ok(pid) => {
                                            player_id = Some(pid.clone());
                                            // Send welcome message
                                            let welcome = server_clone.get_welcome_message(&pid);
                                            let welcome_json = serde_json::to_string(&welcome).unwrap();
                                            if let Err(e) = send_stream.write_all(welcome_json.as_bytes()).await {
                                                error!("Failed to send welcome: {}", e);
                                                break;
                                            }
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
                                    info!("Nickname change requested: {}", nickname);
                                }
                            }
                        } else {
                            warn!("Invalid message format");
                        }
                    }
                }
                Ok(None) => {
                    info!("Stream ended");
                    break;
                }
                Err(e) => {
                    error!("Stream read error: {}", e);
                    break;
                }
            }
        }

        // Clean up player when connection closes
        if let Some(pid) = player_id {
            if let Err(e) = server_clone.remove_player(&pid) {
                error!("Failed to remove player: {}", e);
            }
        }
    });

    // Outgoing message handler
    let outgoing_task = tokio::spawn(async move {
        while let Ok(server_msg) = broadcast_rx.recv().await {
            let json = serde_json::to_string(&server_msg).unwrap();
            if let Err(e) = send_stream.write_all(json.as_bytes()).await {
                error!("Failed to send broadcast message: {}", e);
                break;
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = incoming_task => {},
        _ = outgoing_task => {},
    }

    info!("Session closed");
    Ok(())
}

fn generate_certificate() -> Result<Certificate> {
    let mut params = CertificateParams::new(vec!["localhost".to_string()]);
    params.distinguished_name = DistinguishedName::new();
    params.subject_alt_names = vec![
        rcgen::SanType::DnsName("localhost".to_string()),
        rcgen::SanType::IpAddress(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))),
        rcgen::SanType::IpAddress(std::net::IpAddr::V6(std::net::Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1))),
    ];
    
    Ok(Certificate::from_params(params)?)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::init();

    let server = Arc::new(GameServer::new());
    info!("🎮 WebTransport Game Server starting...");

    // Generate self-signed certificate for development
    let cert = generate_certificate()?;
    let cert_der = cert.serialize_der()?;
    let key_der = cert.serialize_private_key_der();

    // Configure WebTransport server
    let config = ServerConfig::builder()
        .with_bind_default(4433)
        .with_identity(&cert_der, &key_der)
        .build();

    let server_endpoint = Endpoint::server(config)?;
    info!("🚀 WebTransport server listening on https://localhost:4433");
    info!("🔒 Using self-signed certificate for development");
    info!("⚠️  You may need to accept the certificate in your browser");

    // Accept incoming sessions
    loop {
        match server_endpoint.accept().await {
            Ok(incoming_session) => {
                let server_clone = server.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_session(incoming_session, server_clone).await {
                        error!("Session error: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("Failed to accept session: {}", e);
            }
        }
    }
} 