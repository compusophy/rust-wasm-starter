import init, { greet, add, get_message } from './pkg/rust_wasm_hello.js';

let wasmModule = null;

// Initialize WASM
async function initWasm() {
    try {
        wasmModule = await init();
        console.log('✅ Rust WASM loaded');
        document.getElementById('wasm-status').innerHTML = '✅ Rust WASM loaded!';
        document.querySelectorAll('button').forEach(btn => btn.disabled = false);
    } catch (error) {
        console.error('❌ WASM failed:', error);
        document.getElementById('wasm-status').innerHTML = '❌ WASM failed';
    }
}

// Simple event handlers
document.addEventListener('submit', function(evt) {
    const form = evt.target;
    const action = form.getAttribute('hx-post');
    
    if (action?.includes('/greet') || action?.includes('/add')) {
        evt.preventDefault();
        
        if (!wasmModule) return;

        const formData = new FormData(form);
        const target = document.querySelector(form.getAttribute('hx-target'));
        const timestamp = new Date().toLocaleTimeString();
        
        if (action.includes('/greet')) {
            const name = formData.get('name') || 'World';
            greet(name);
            target.innerHTML = `<div><strong>[${timestamp}]</strong> 👋 Greeted "${name}" from Rust!</div>`;
        } else if (action.includes('/add')) {
            const num1 = parseInt(formData.get('num1')) || 0;
            const num2 = parseInt(formData.get('num2')) || 0;
            const result = add(num1, num2);
            target.innerHTML = `<div><strong>[${timestamp}]</strong> 🔢 ${num1} + ${num2} = ${result}</div>`;
        }
    }
});

document.addEventListener('click', function(evt) {
    const button = evt.target.closest('button[hx-post="/message"]');
    if (button) {
        evt.preventDefault();
        if (!wasmModule) return;

        const message = get_message();
        const target = document.querySelector(button.getAttribute('hx-target'));
        const timestamp = new Date().toLocaleTimeString();
        target.innerHTML = `<div><strong>[${timestamp}]</strong> 📝 ${message}</div>`;
    }
});

// Disable buttons until WASM loads
document.querySelectorAll('button').forEach(btn => btn.disabled = true);

initWasm(); 