# 🎮 MVP: Chat Room with 2D Movement

## Vision
A simple multiplayer chat room where players can:
- Move around in a 2D space using WASD/arrow keys
- Send chat messages to all players
- See other players moving in real-time
- Join/leave seamlessly

## Technical Stack
- **Frontend**: Rust WASM + HTMX + Canvas/DOM for visualization
- **Backend**: Rust WebSocket server (tokio-tungstenite)
- **Transport**: WebSockets (JSON messages)
- **Deployment**: Railway for both frontend and backend

## Game Mechanics

### Player Movement
- **2D coordinate system**: (x, y) positions
- **Movement**: WASD or arrow keys
- **Speed**: Fixed movement speed (e.g., 5 pixels per keypress)
- **Boundaries**: 800x600 game area with collision detection
- **Smooth movement**: Client-side prediction + server reconciliation

### Chat System
- **Real-time messaging**: All messages broadcast to all players
- **Message format**: `[Timestamp] PlayerName: Message`
- **Message history**: Last 50 messages stored and shown to new joiners
- **Commands**: Basic `/nick <name>` to change nickname

### Player Management
- **Anonymous joining**: No registration required
- **Random nicknames**: Generated if not provided (e.g., "Player123")
- **Player list**: Show all connected players
- **Graceful disconnect**: Clean up when players leave

## Data Structures

### Player State
```rust
#[derive(Serialize, Deserialize, Clone)]
pub struct Player {
    pub id: String,           // UUID
    pub nickname: String,     // Display name
    pub x: f32,              // X position (0-800)
    pub y: f32,              // Y position (0-600)
    pub color: String,       // Player color (hex)
    pub last_seen: SystemTime,
}
```

### Messages
```rust
#[derive(Serialize, Deserialize)]
pub enum ClientMessage {
    Join { nickname: Option<String> },
    Move { x: f32, y: f32 },
    Chat { message: String },
    ChangeNick { nickname: String },
}

#[derive(Serialize, Deserialize)]
pub enum ServerMessage {
    Welcome { your_id: String, players: Vec<Player> },
    PlayerJoined { player: Player },
    PlayerLeft { player_id: String },
    PlayerMoved { player_id: String, x: f32, y: f32 },
    ChatMessage { player_id: String, nickname: String, message: String, timestamp: u64 },
    PlayerList { players: Vec<Player> },
}
```

## UI Design

### Game Area
```
┌─────────────────────────────────────┐
│  🔵 Player1    🟢 You    🟡 Player3  │  ← 2D movement area
│                                     │    800x400 pixels
│      🔴 Player2                     │
│                                     │
└─────────────────────────────────────┘
```

### Chat Panel
```
┌─────────────────────────────────────┐
│ Chat Messages:                      │
│ [14:32] Player1: Hello everyone!    │
│ [14:33] You: Hey there!             │
│ [14:33] Player2: Nice game          │
│ ┌─────────────────────────────────┐ │
│ │ Type message...                 │ │ ← HTMX form
│ └─────────────────────────────────┘ │
└─────────────────────────────────────┘
```

### Player List
```
┌─────────────────┐
│ Online Players  │
│ • You (Player1) │
│ • Player2       │
│ • Player3       │
│ • Player4       │
└─────────────────┘
```

## Implementation Plan

### Phase 1: Basic WebSocket Server
```rust
// Simple echo server with player management
// Message routing: Join → Move → Chat
// In-memory player storage with HashMap
```

### Phase 2: Frontend WebSocket Client
```rust
// WASM WebSocket client
// JSON message serialization/deserialization
// Basic connection management
```

### Phase 3: Movement System
```rust
// Keyboard input handling in WASM
// Position validation on server
// Broadcast position updates
```

### Phase 4: Chat Integration
```rust
// Chat form with HTMX
// Message broadcasting
// Chat history management
```

### Phase 5: Visualization
```rust
// HTML Canvas or CSS-based player representation
// Real-time position updates
// Simple player avatars (colored circles)
```

## Success Criteria
1. ✅ 2+ players can join the room
2. ✅ Players can move around and see each other move
3. ✅ Chat messages appear in real-time for all players
4. ✅ New players see current game state when joining
5. ✅ Players can leave gracefully without breaking others

## Future Extensions (Post-MVP)
- Player avatars/sprites
- Private messaging
- Multiple rooms
- Spectator mode
- Voice chat integration
- Game elements (collectibles, mini-games)

## Development Timeline
- **Week 1**: WebSocket server + basic frontend connection
- **Week 2**: Movement system + player visualization  
- **Week 3**: Chat system + UI polish
- **Week 4**: Deployment + multi-player testing

Ready to start building! Should we begin with the WebSocket server? 