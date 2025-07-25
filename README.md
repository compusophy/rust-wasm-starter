# 🦀 Rust WebSocket Multiplayer Game

A **real-time multiplayer game** built with Rust WebSocket server and WebAssembly client. This is a true monolith - a single Rust binary that serves both the frontend static files and handles WebSocket connections for real-time gameplay.

## 🎮 What This Actually Does

**YES, this is REAL Rust code everywhere!** 🦀
- **Server**: Rust WebSocket server handling real-time multiplayer game logic
- **Client**: Rust WASM for game client with direct DOM manipulation  
- **Transport**: WebSocket for reliable real-time communication
- **Deployment**: Single binary monolith perfect for Railway deployment

## ✨ Features

- 🎯 **Real-time multiplayer** - Move around and see other players instantly
- 💬 **Live chat** - Chat with other players in real-time
- 🎨 **Unique player colors** - Each player gets a random color and nickname
- 🌐 **Monolith architecture** - Single Rust binary serves everything
- 🚀 **Easy deployment** - One-click deploy to Railway
- 🔄 **Hot reloading** - Vite dev server for rapid frontend development

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────┐
│                Rust Monolith Server                │
├─────────────────────┬───────────────────────────────┤
│   HTTP Server       │       WebSocket Server       │
│   (Static Files)    │    (Game Logic + Chat)       │
│   Port: 8080        │       Port: 8081              │
└─────────────────────┴───────────────────────────────┘
                      ↑
              ┌───────────────┐
              │  Rust WASM    │
              │    Client     │
              └───────────────┘
```

## 🚀 Quick Start

### Prerequisites

- **Rust** - Install from [rustup.rs](https://rustup.rs/)
- **wasm-pack** - Install with: `cargo install wasm-pack`
- **Node.js** - Install from [nodejs.org](https://nodejs.org/)

### Development

1. **Install frontend dependencies:**
   ```bash
   npm install
   ```

2. **Build and run the monolith:**
   ```bash
   npm run dev-monolith
   ```

3. **Open your browser** to http://localhost:8080

### Production Build

```bash
npm run build-all
./target/release/server
```

## 🛠️ Development Workflow

- **Frontend changes**: Use `npm run dev` for Vite hot reloading
- **WASM changes**: Run `npm run build-wasm` then restart server
- **Server changes**: Restart with `npm run dev-server`
- **Full rebuild**: `npm run dev-monolith`

## 🚀 Railway Deployment

This project is ready for one-click Railway deployment:

1. **Fork this repository**
2. **Connect to Railway** - Railway will auto-detect the Dockerfile
3. **Deploy** - Railway will build and deploy the monolith
4. **Play** - Your multiplayer game is live!

The monolith approach means:
- ✅ **Single service** - No need for separate frontend/backend deployments
- ✅ **No CORS issues** - Everything served from same origin
- ✅ **Cost effective** - One Railway service instead of two
- ✅ **Simpler networking** - WebSocket connection auto-discovers correct ports

## 📁 Project Structure

```
rust-wasm-websocket-game/
├── src/
│   ├── lib.rs          # 🦀 Rust WASM client code
│   └── main.rs         # 🦀 Rust WebSocket server + HTTP static server
├── Cargo.toml          # Rust dependencies (both WASM + server)
├── package.json        # Frontend build tools
├── index.html          # Game UI with HTMX
├── main.js             # Minimal JS glue layer
├── Dockerfile          # Railway deployment
└── railway.toml        # Railway configuration
```

## 🎯 How to Play

1. **Enter a nickname** (optional) and click "Connect to Game Server"
2. **Move around** using WASD or arrow keys
3. **Chat** with other players using the chat box
4. **See other players** moving around in real-time
5. **Change nickname** anytime with the button

## 🔧 Technology Stack

- **🦀 Backend**: Rust with tokio-tungstenite WebSocket server
- **🦀 Frontend**: Rust WASM with direct DOM manipulation
- **🔗 Transport**: WebSocket (TCP) for reliable real-time communication
- **🎯 Serialization**: JSON for human-readable debugging
- **🌐 Static Files**: Hyper HTTP server serving Vite-built frontend
- **🚀 Deployment**: Docker + Railway for production hosting
- **⚡ Development**: Vite for fast frontend iteration

## 🌐 Environment Variables

- `PORT` - HTTP server port (default: 8080, WebSocket uses PORT+1)
- `STATIC_PATH` - Path to static files (default: "./dist")

## 🎮 Game Features

- **Real-time movement** - Smooth player movement with collision detection
- **Player management** - Join/leave with automatic cleanup
- **Chat system** - Real-time chat with timestamps
- **Responsive UI** - Works on desktop and mobile
- **Error handling** - Graceful connection failures and reconnection

## 🔍 Debugging

- **Browser console** - Check for WebSocket connection logs
- **Server logs** - Run with `RUST_LOG=debug` for detailed logging
- **Network tab** - Inspect WebSocket messages in browser dev tools

## 🤝 Contributing

This is a demonstration project showcasing:
- Rust WebSocket servers with tokio
- Rust WASM for web clients
- Monolith architecture patterns
- Railway deployment strategies

Feel free to fork and extend with your own game features! 