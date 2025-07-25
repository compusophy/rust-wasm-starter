import init, { connect_to_game, move_player, send_chat_message, change_nickname } from './pkg/rust_wasm_hello.js';

let wasmModule = null;
let isConnected = false;
let playerPosition = { x: 200, y: 150 };
let keys = {};

// Initialize WASM
async function initWasm() {
    try {
        wasmModule = await init();
        console.log('✅ Rust WASM WebSocket Game Client loaded');
        document.getElementById('wasm-status').innerHTML = '✅ Rust WASM WebSocket Game Client loaded!';
        document.getElementById('connect-btn').disabled = false;
        setupKeyboardInput();
        startGameLoop();
    } catch (error) {
        console.error('❌ WASM failed:', error);
        document.getElementById('wasm-status').innerHTML = '❌ WASM failed to load';
    }
}

// Connect to game server
window.connectToGame = function() {
    if (isConnected) return;
    
    const statusEl = document.getElementById('connection-status');
    const connectBtn = document.getElementById('connect-btn');
    const nicknameInput = document.getElementById('nickname-input');
    
    try {
        const nickname = nicknameInput.value.trim() || null;
        
        connectBtn.disabled = true;
        statusEl.innerHTML = '🔄 Connecting to WebSocket server...';
        
        connect_to_game(nickname);
        
        // Give it a moment to connect
        setTimeout(() => {
            isConnected = true;
            statusEl.innerHTML = '✅ Connected! Use WASD or arrow keys to move around.';
            connectBtn.innerHTML = '✅ Connected';
            connectBtn.style.background = '#4caf50';
            console.log('✅ Connected to WebSocket game server');
        }, 500);
        
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
        }
        requestAnimationFrame(gameLoop);
    };
    requestAnimationFrame(gameLoop);
}

// Handle player movement
function handleMovement() {
    let moved = false;
    const speed = 3;
    
    // WASD or Arrow Keys
    if (keys['KeyW'] || keys['ArrowUp']) {
        if (playerPosition.y > 10) {
            playerPosition.y -= speed;
            moved = true;
        }
    }
    if (keys['KeyS'] || keys['ArrowDown']) {
        if (playerPosition.y < 360) {
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
        if (playerPosition.x < 770) {
            playerPosition.x += speed;
            moved = true;
        }
    }
    
    if (moved) {
        try {
            move_player(playerPosition.x, playerPosition.y);
        } catch (error) {
            console.error('Movement error:', error);
        }
    }
}

// Send chat message
window.sendChat = function(event) {
    event.preventDefault();
    
    if (!isConnected) {
        alert('Please connect to the game first!');
        return;
    }
    
    const input = document.getElementById('chat-input');
    const message = input.value.trim();
    
    if (!message) return;
    
    try {
        send_chat_message(message);
        input.value = '';
    } catch (error) {
        console.error('Chat error:', error);
    }
};

// Change nickname
window.changeNickname = function() {
    if (!isConnected) {
        alert('Please connect to the game first!');
        return;
    }
    
    const newNickname = prompt('Enter new nickname:');
    if (newNickname && newNickname.trim()) {
        try {
            change_nickname(newNickname.trim());
        } catch (error) {
            console.error('Nickname change error:', error);
        }
    }
};

// Disable connect button until WASM loads
document.getElementById('connect-btn').disabled = true;

initWasm(); 