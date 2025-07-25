import init, { connect_to_game, move_player, get_players_html, send_chat_message } from './pkg/rust_wasm_hello.js';

let wasmModule = null;
let isConnected = false;
let playerPosition = { x: 200, y: 150 }; // Start in center of game area
let keys = {};

// Initialize WASM
async function initWasm() {
    try {
        wasmModule = await init();
        console.log('✅ Rust WASM Game Client loaded');
        document.getElementById('wasm-status').innerHTML = '✅ Rust WASM Game Client loaded!';
        document.getElementById('connect-btn').disabled = false;
        setupKeyboardInput();
        startGameLoop();
    } catch (error) {
        console.error('❌ WASM failed:', error);
        document.getElementById('wasm-status').innerHTML = '❌ WASM failed to load';
    }
}

// Connect to game server
window.connectToGame = async function() {
    if (isConnected) return;
    
    const statusEl = document.getElementById('connection-status');
    const connectBtn = document.getElementById('connect-btn');
    
    try {
        connectBtn.disabled = true;
        statusEl.innerHTML = '🔄 Connecting to WebTransport server...';
        
        await connect_to_game();
        
        isConnected = true;
        statusEl.innerHTML = '✅ Connected! You can now move around and chat.';
        connectBtn.innerHTML = '✅ Connected';
        connectBtn.style.background = '#4caf50';
        
        console.log('✅ Connected to game server via WebTransport');
    } catch (error) {
        console.error('❌ Connection failed:', error);
        statusEl.innerHTML = `❌ Connection failed: ${error.message}`;
        connectBtn.disabled = false;
        connectBtn.style.background = '#ff6b6b';
    }
};

// Setup keyboard input
function setupKeyboardInput() {
    document.addEventListener('keydown', (e) => {
        keys[e.code] = true;
        e.preventDefault();
    });
    
    document.addEventListener('keyup', (e) => {
        keys[e.code] = false;
        e.preventDefault();
    });
}

// Game loop for handling movement
function startGameLoop() {
    const gameLoop = () => {
        if (isConnected && wasmModule) {
            handleMovement();
            updatePlayersDisplay();
        }
        requestAnimationFrame(gameLoop);
    };
    requestAnimationFrame(gameLoop);
}

// Handle player movement
async function handleMovement() {
    let moved = false;
    const speed = 3;
    const gameArea = document.getElementById('game-area');
    const bounds = gameArea.getBoundingClientRect();
    
    // WASD or Arrow Keys
    if (keys['KeyW'] || keys['ArrowUp']) {
        if (playerPosition.y > 10) {
            playerPosition.y -= speed;
            moved = true;
        }
    }
    if (keys['KeyS'] || keys['ArrowDown']) {
        if (playerPosition.y < 270) {
            playerPosition.y += speed;
            moved = true;
        }
    }
    if (keys['KeyA'] || keys['ArrowLeft']) {
        if (playerPosition.x > 10) {
            playerPosition.x -= speed;
            moved = true;
        }
    }
    if (keys['KeyD'] || keys['ArrowRight']) {
        if (playerPosition.x < 370) {
            playerPosition.x += speed;
            moved = true;
        }
    }
    
    if (moved) {
        try {
            await move_player(playerPosition.x, playerPosition.y);
        } catch (error) {
            console.error('Movement error:', error);
        }
    }
}

// Update players display
function updatePlayersDisplay() {
    if (!wasmModule) return;
    
    try {
        const playersHtml = get_players_html();
        document.getElementById('players-container').innerHTML = playersHtml;
    } catch (error) {
        console.error('Display update error:', error);
    }
}

// Send chat message
window.sendChat = async function(event) {
    event.preventDefault();
    
    if (!isConnected) {
        alert('Please connect to the game first!');
        return;
    }
    
    const input = document.getElementById('chat-input');
    const message = input.value.trim();
    
    if (!message) return;
    
    try {
        await send_chat_message(message);
        
        // Add to local chat (in real implementation, this would come from server)
        const chatMessages = document.getElementById('chat-messages');
        const timestamp = new Date().toLocaleTimeString();
        chatMessages.innerHTML += `<div><strong>[${timestamp}] You:</strong> ${message}</div>`;
        chatMessages.scrollTop = chatMessages.scrollHeight;
        
        input.value = '';
    } catch (error) {
        console.error('Chat error:', error);
    }
};

// Disable connect button until WASM loads
document.getElementById('connect-btn').disabled = true;

initWasm(); 