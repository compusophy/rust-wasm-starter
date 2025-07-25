# 🦀 Rust WASM + HTMX + Vite Hot Reload Stable

A simple **Rust WebAssembly2** project with HTMX for clean DOM interactions and Vite hot reloading for an excellent development experience.

## What This Actually Does

**YES, this is REAL Rust code!** 🦀
- All the core logic (`greet`, `add`, `get_message`) is written in **Rust**
- Compiled to WebAssembly for browser execution
- HTMX handles all DOM interactions (no JavaScript DOM manipulation!)
- Minimal JavaScript glue layer just loads WASM and provides HTTP endpoints for HTMX

## Prerequisites

Make sure you have the following installed:

1. **Rust** - Install from [rustup.rs](https://rustup.rs/)
2. **wasm-pack** - Install with: `cargo install wasm-pack`
3. **Node.js** - Install from [nodejs.org](https://nodejs.org/)

## Quick Start

1. **Install dependencies:**
   ```bash
   npm install
   ```

2. **Build the WASM module:**
   ```bash
   npm run build-wasm
   ```

3. **Start the development server:**
   ```bash
   npm run dev
   ```

4. **Open your browser** to the URL shown (usually http://localhost:5173)

## Development Workflow

- **Hot Reloading**: Vite automatically reloads when you change HTML/CSS/JS files
- **WASM Changes**: When you modify Rust code, run `npm run build-wasm` and Vite will hot-reload the page
- **No Stale Builds**: Vite's intelligent caching prevents the stale build issues you experienced
- **Console Logs**: Check the browser console to see output from your Rust functions

## Architecture

```
┌─────────────┐    ┌──────────────┐    ┌─────────────┐
│    HTMX     │───▶│  Fetch API   │───▶│ Rust WASM   │
│ (DOM magic) │    │ (minimal JS) │    │ (your code) │
└─────────────┘    └──────────────┘    └─────────────┘
```

- **HTMX**: Handles all form submissions and DOM updates declaratively
- **Fetch Interceptor**: Minimal JavaScript that intercepts HTMX requests and calls Rust functions
- **Rust WASM**: Your actual business logic, compiled from Rust

## Project Structure

```
rust-wasm/
├── src/
│   └── lib.rs          # 🦀 Your Rust WASM code (THE REAL STUFF!)
├── pkg/                # Generated WASM files (created after build)
├── Cargo.toml          # Rust dependencies
├── package.json        # Node.js dependencies (just build tools)
├── vite.config.js      # Vite configuration
├── index.html          # HTML with HTMX attributes
└── main.js             # Minimal JS glue (just WASM loading + HTMX endpoints)
```

## Available Rust Functions

The Rust module exports these functions (check `src/lib.rs`):

- `greet(name: &str)` - Logs a greeting to console from Rust
- `add(a: i32, b: i32) -> i32` - Adds two numbers in Rust
- `get_message() -> String` - Returns a string generated in Rust

## Why HTMX + Rust WASM?

✅ **No JavaScript DOM manipulation** - HTMX handles it declaratively  
✅ **Real Rust code** - Your logic is actually in Rust, not JavaScript  
✅ **Hot reloading** - Vite prevents stale builds  
✅ **Clean separation** - UI interactions in HTML, logic in Rust  
✅ **Performance** - WebAssembly is fast, HTMX is lightweight  

## Building for Production

```bash
npm run build-all
```

This will build both the WASM module and create an optimized production build in the `dist/` folder.

## Modifying the Rust Code

Edit `src/lib.rs` to add your own functions:

```rust
#[wasm_bindgen]
pub fn your_function(input: &str) -> String {
    format!("Rust processed: {}", input)
}
```

Then rebuild: `npm run build-wasm`

## Troubleshooting

- **WASM module not loading**: Make sure you've run `npm run build-wasm` first
- **Stale builds**: Vite should handle this automatically (that's why we're using it!)
- **Import errors**: Make sure the WASM package name matches in both `Cargo.toml` and `main.js`
- **HTMX not working**: Check browser console for any fetch errors 