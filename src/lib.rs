use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use web_sys::*;
use wasm_bindgen::closure::Closure;

// Import console functions
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
    #[wasm_bindgen(js_namespace = console)]
    fn error(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

macro_rules! console_error {
    ($($t:tt)*) => (error(&format_args!($($t)*).to_string()))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Player {
    id: String,
    nickname: String,
    x: f32,
    y: f32,
    color: String,
    last_seen: u64,
}

// Client -> Server messages
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
enum ClientMessage {
    Join { nickname: Option<String> },
    Move { x: f32, y: f32 },
    Chat { message: String },
    ChangeNick { nickname: String },
}

// Server -> Client messages
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
enum ServerMessage {
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

static mut GAME_CLIENT: Option<GameClient> = None;

struct GameClient {
    websocket: Option<WebSocket>,
    players: Arc<Mutex<HashMap<String, Player>>>,
    my_player_id: Option<String>,
    _on_message_closure: Option<Closure<dyn FnMut(MessageEvent)>>,
    _on_close_closure: Option<Closure<dyn FnMut(CloseEvent)>>,
    _on_error_closure: Option<Closure<dyn FnMut(Event)>>,
}

impl GameClient {
    fn new() -> Self {
        Self {
            websocket: None,
            players: Arc::new(Mutex::new(HashMap::new())),
            my_player_id: None,
            _on_message_closure: None,
            _on_close_closure: None,
            _on_error_closure: None,
        }
    }

    fn connect(&mut self, nickname: Option<String>) -> Result<(), JsValue> {
        console_log!("Connecting to WebSocket server...");
        
        // Create WebSocket connection - connect to /ws endpoint on same port
        let ws_url = if let Some(window) = web_sys::window() {
            let location = window.location();
            if let (Ok(hostname), Ok(protocol)) = (location.hostname(), location.protocol()) {
                if hostname == "localhost" || hostname == "127.0.0.1" {
                    "ws://127.0.0.1:8080/ws".to_string()
                } else {
                    // For production, use same domain with /ws endpoint
                    let ws_protocol = if protocol == "https:" { "wss" } else { "ws" };
                    let port = if let Ok(port_str) = location.port() {
                        if !port_str.is_empty() {
                            format!(":{}", port_str)
                        } else {
                            "".to_string()
                        }
                    } else {
                        "".to_string()
                    };
                    format!("{}://{}{}/ws", ws_protocol, hostname, port)
                }
            } else {
                "ws://127.0.0.1:8080/ws".to_string()
            }
        } else {
            "ws://127.0.0.1:8080/ws".to_string()
        };
        
        console_log!("Connecting to WebSocket: {}", ws_url);
        let ws = WebSocket::new(&ws_url)?;
        ws.set_binary_type(BinaryType::Arraybuffer);

        let players_clone = Arc::clone(&self.players);
        let mut my_id = None;
        
        // Handle incoming messages
        let on_message = Closure::wrap(Box::new(move |e: MessageEvent| {
            if let Ok(text) = e.data().dyn_into::<js_sys::JsString>() {
                let message_str = String::from(text);
                console_log!("Received: {}", message_str);
                
                if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&message_str) {
                    if let Ok(mut players) = players_clone.lock() {
                        match server_msg {
                            ServerMessage::Welcome { your_id, players: player_list } => {
                                console_log!("Welcome! Your ID: {}", your_id);
                                my_id = Some(your_id);
                                players.clear();
                                for player in player_list {
                                    players.insert(player.id.clone(), player);
                                }
                                update_ui();
                            }
                            ServerMessage::PlayerJoined { player } => {
                                console_log!("Player joined: {}", player.nickname);
                                players.insert(player.id.clone(), player);
                                update_ui();
                            }
                            ServerMessage::PlayerLeft { player_id } => {
                                console_log!("Player left: {}", player_id);
                                players.remove(&player_id);
                                update_ui();
                            }
                            ServerMessage::PlayerMoved { player_id, x, y } => {
                                if let Some(player) = players.get_mut(&player_id) {
                                    player.x = x;
                                    player.y = y;
                                }
                                update_ui();
                            }
                            ServerMessage::ChatMessage { player_id, nickname, message, timestamp } => {
                                add_chat_message(&nickname, &message, timestamp);
                            }
                            ServerMessage::Error { message } => {
                                console_error!("Server error: {}", message);
                            }
                        }
                    }
                } else {
                    console_error!("Failed to parse server message: {}", message_str);
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);

        let on_close = Closure::wrap(Box::new(move |e: CloseEvent| {
            console_log!("WebSocket closed: code={}, reason={}", e.code(), e.reason());
        }) as Box<dyn FnMut(CloseEvent)>);

        let on_error = Closure::wrap(Box::new(move |e: Event| {
            console_error!("WebSocket error: {:?}", e);
        }) as Box<dyn FnMut(Event)>);

        ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));
        ws.set_onerror(Some(on_error.as_ref().unchecked_ref()));

        // Send join message when connection opens
        let join_msg = ClientMessage::Join { nickname };
        let join_json = serde_json::to_string(&join_msg).unwrap();
        
        let ws_clone = ws.clone();
        let on_open = Closure::wrap(Box::new(move |_: Event| {
            console_log!("WebSocket connected!");
            if let Err(e) = ws_clone.send_with_str(&join_json) {
                console_error!("Failed to send join message: {:?}", e);
            }
        }) as Box<dyn FnMut(Event)>);
        
        ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));
        on_open.forget(); // Let the closure live

        self.websocket = Some(ws);
        self._on_message_closure = Some(on_message);
        self._on_close_closure = Some(on_close);
        self._on_error_closure = Some(on_error);

        Ok(())
    }

    fn send_message(&self, message: ClientMessage) -> Result<(), JsValue> {
        if let Some(ws) = &self.websocket {
            let json = serde_json::to_string(&message).unwrap();
            ws.send_with_str(&json)?;
        }
        Ok(())
    }
}

fn update_ui() {
    unsafe {
        if let Some(client) = &GAME_CLIENT {
            if let Ok(players) = client.players.lock() {
                let mut html = String::new();
                for player in players.values() {
                    html.push_str(&format!(
                        r#"<div class="player" style="position: absolute; left: {}px; top: {}px; 
                            width: 20px; height: 20px; background: {}; border-radius: 50%; 
                            border: 2px solid #fff; box-shadow: 0 2px 4px rgba(0,0,0,0.3);" 
                            title="{}"></div>"#,
                        player.x, player.y, player.color, player.nickname
                    ));
                }
                
                if let Some(window) = web_sys::window() {
                    if let Some(document) = window.document() {
                        if let Some(container) = document.get_element_by_id("players-container") {
                            container.set_inner_html(&html);
                        }
                    }
                }
            }
        }
    }
}

fn add_chat_message(nickname: &str, message: &str, timestamp: u64) {
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            if let Some(chat_messages) = document.get_element_by_id("chat-messages") {
                let time = js_sys::Date::new(&JsValue::from_f64(timestamp as f64 * 1000.0));
                let time_str = time.to_locale_time_string("en-US");
                
                let current_html = chat_messages.inner_html();
                let new_message = format!(
                    r#"<div><strong>[{}] {}:</strong> {}</div>"#,
                    time_str.as_string().unwrap_or_default(),
                    nickname,
                    message
                );
                
                chat_messages.set_inner_html(&(current_html + &new_message));
                chat_messages.set_scroll_top(chat_messages.scroll_height());
            }
        }
    }
}

// Export functions for JavaScript to call
#[wasm_bindgen]
pub fn connect_to_game(nickname: Option<String>) -> Result<(), JsValue> {
    unsafe {
        if GAME_CLIENT.is_none() {
            GAME_CLIENT = Some(GameClient::new());
        }
        if let Some(client) = &mut GAME_CLIENT {
            client.connect(nickname)?;
        }
    }
    Ok(())
}

#[wasm_bindgen]
pub fn move_player(x: f32, y: f32) -> Result<(), JsValue> {
    unsafe {
        if let Some(client) = &GAME_CLIENT {
            let message = ClientMessage::Move { x, y };
            client.send_message(message)?;
        }
    }
    Ok(())
}

#[wasm_bindgen]
pub fn send_chat_message(message: String) -> Result<(), JsValue> {
    unsafe {
        if let Some(client) = &GAME_CLIENT {
            let chat_msg = ClientMessage::Chat { message };
            client.send_message(chat_msg)?;
        }
    }
    Ok(())
}

#[wasm_bindgen]
pub fn change_nickname(nickname: String) -> Result<(), JsValue> {
    unsafe {
        if let Some(client) = &GAME_CLIENT {
            let msg = ClientMessage::ChangeNick { nickname };
            client.send_message(msg)?;
        }
    }
    Ok(())
}

// Legacy functions (keep for compatibility)
#[wasm_bindgen]
pub fn greet(name: &str) {
    console_log!("Hello, {}! This is from Rust via WASM 🦀", name);
}

#[wasm_bindgen]
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[wasm_bindgen]
pub fn get_message() -> String {
    "Hello from Rust and WebAssembly! 🚀".to_string()
}

#[wasm_bindgen(start)]
pub fn main() {
    console_log!("Rust WASM WebSocket Game Client loaded successfully!");
} 