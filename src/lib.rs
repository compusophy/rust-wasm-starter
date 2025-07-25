use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use web_sys::*;
use wasm_bindgen::closure::Closure;

// Import the `console.log` function from the `console` module
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
    #[wasm_bindgen(js_namespace = console)]
    fn error(s: &str);
}

// WebTransport JavaScript bindings
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = WebTransport)]
    type WebTransport;

    #[wasm_bindgen(constructor)]
    fn new(url: &str, options: &JsValue) -> WebTransport;

    #[wasm_bindgen(method, getter)]
    fn ready(this: &WebTransport) -> js_sys::Promise;

    #[wasm_bindgen(method, js_name = createUnidirectionalStream)]
    fn create_unidirectional_stream(this: &WebTransport) -> js_sys::Promise;

    #[wasm_bindgen(method, js_name = sendDatagram)]
    fn send_datagram(this: &WebTransport, data: &js_sys::Uint8Array);

    #[wasm_bindgen(method, getter)]
    fn datagrams(this: &WebTransport) -> WebTransportDatagramDuplexStream;

    #[wasm_bindgen(js_name = WebTransportDatagramDuplexStream)]
    type WebTransportDatagramDuplexStream;

    #[wasm_bindgen(method, getter)]
    fn readable(this: &WebTransportDatagramDuplexStream) -> web_sys::ReadableStream;

    #[wasm_bindgen(js_name = WritableStreamDefaultWriter)]
    type WritableStreamDefaultWriter;

    #[wasm_bindgen(method)]
    fn write(this: &WritableStreamDefaultWriter, chunk: &JsValue) -> js_sys::Promise;

    #[wasm_bindgen(method)]
    fn close(this: &WritableStreamDefaultWriter) -> js_sys::Promise;
}

// Define a macro to make console.log easier to use
macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

macro_rules! console_error {
    ($($t:tt)*) => (error(&format_args!($($t)*).to_string()))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Player {
    id: String,
    name: String,
    x: f32,
    y: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum GameMessage {
    PlayerJoined { player: Player },
    PlayerLeft { player_id: String },
    PlayerMoved { player_id: String, x: f32, y: f32 },
    ChatMessage { player_id: String, message: String },
    GameState { players: Vec<Player> },
}

static mut GAME_CLIENT: Option<GameClient> = None;

struct GameClient {
    transport: Option<WebTransport>,
    players: Arc<Mutex<HashMap<String, Player>>>,
    my_player_id: Option<String>,
}

impl GameClient {
    fn new() -> Self {
        Self {
            transport: None,
            players: Arc::new(Mutex::new(HashMap::new())),
            my_player_id: None,
        }
    }

    async fn connect(&mut self) -> Result<(), JsValue> {
        console_log!("Connecting to WebTransport server...");
        
        // Create WebTransport options allowing self-signed certificates for development
        let options = js_sys::Object::new();
        js_sys::Reflect::set(&options, &"allowPooling".into(), &false.into())?;
        
        let transport = WebTransport::new("https://localhost:4433", &options);
        
        // Wait for connection to be ready
        JsFuture::from(transport.ready()).await?;
        console_log!("WebTransport connected!");
        
        // Set up datagram reading (for incoming game messages)
        let datagrams = transport.datagrams();
        let readable = datagrams.readable();
        
        // Note: In a real implementation, we'd set up a reader loop for incoming datagrams
        // For now, we'll focus on sending outbound messages
        
        self.transport = Some(transport);
        Ok(())
    }

    async fn send_message(&self, message: GameMessage) -> Result<(), JsValue> {
        if let Some(transport) = &self.transport {
            // Serialize message to binary
            let data = bincode::serialize(&message).map_err(|e| {
                JsValue::from_str(&format!("Serialization error: {}", e))
            })?;
            
            // Convert to Uint8Array using js-sys
            let uint8_array = js_sys::Uint8Array::new_with_length(data.len() as u32);
            uint8_array.copy_from(&data);
            
            // Send as datagram for low latency
            transport.send_datagram(&uint8_array);
        }
        Ok(())
    }

    fn update_player_position(&mut self, x: f32, y: f32) {
        if let Some(player_id) = &self.my_player_id {
            if let Ok(mut players) = self.players.lock() {
                if let Some(player) = players.get_mut(player_id) {
                    player.x = x;
                    player.y = y;
                }
            }
        }
    }

    fn render_players(&self) -> String {
        if let Ok(players) = self.players.lock() {
            let mut html = String::new();
            for player in players.values() {
                html.push_str(&format!(
                    r#"<div class="player" style="position: absolute; left: {}px; top: {}px; 
                        width: 20px; height: 20px; background: #007acc; border-radius: 50%;" 
                        title="{}"></div>"#,
                    player.x, player.y, player.name
                ));
            }
            html
        } else {
            String::new()
        }
    }
}

// Export functions for JavaScript to call
#[wasm_bindgen]
pub async fn connect_to_game() -> Result<(), JsValue> {
    unsafe {
        if GAME_CLIENT.is_none() {
            GAME_CLIENT = Some(GameClient::new());
        }
        if let Some(client) = &mut GAME_CLIENT {
            client.connect().await?;
        }
    }
    Ok(())
}

#[wasm_bindgen]
pub async fn move_player(x: f32, y: f32) -> Result<(), JsValue> {
    unsafe {
        if let Some(client) = &mut GAME_CLIENT {
            client.update_player_position(x, y);
            if let Some(player_id) = &client.my_player_id {
                let message = GameMessage::PlayerMoved {
                    player_id: player_id.clone(),
                    x,
                    y,
                };
                client.send_message(message).await?;
            }
        }
    }
    Ok(())
}

#[wasm_bindgen]
pub fn get_players_html() -> String {
    unsafe {
        if let Some(client) = &GAME_CLIENT {
            client.render_players()
        } else {
            String::new()
        }
    }
}

#[wasm_bindgen]
pub async fn send_chat_message(message: String) -> Result<(), JsValue> {
    unsafe {
        if let Some(client) = &GAME_CLIENT {
            if let Some(player_id) = &client.my_player_id {
                let game_message = GameMessage::ChatMessage {
                    player_id: player_id.clone(),
                    message,
                };
                client.send_message(game_message).await?;
            }
        }
    }
    Ok(())
}

// Legacy functions (keep for compatibility)
#[wasm_bindgen]
pub fn greet(name: &str) {
    console_log!("Hello, {}! This is from Rust via WASM2 🦀", name);
}

#[wasm_bindgen]
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[wasm_bindgen]
pub fn get_message() -> String {
    "Hello from Rust and WebAssembly! 🚀".to_string()
}

// Called when the WASM module is instantiated
#[wasm_bindgen(start)]
pub fn main() {
    console_log!("Rust WASM Game Client loaded successfully!");
} 