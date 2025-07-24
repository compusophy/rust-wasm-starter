# Use latest Rust image
FROM rust:latest as rust-builder

# Install wasm-pack
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Set working directory
WORKDIR /app

# Copy Rust files
COPY Cargo.toml ./
COPY src/ ./src/

# Build WASM (will regenerate Cargo.lock with correct version)
RUN wasm-pack build --target web --out-dir pkg

# Use Node.js for the frontend build
FROM node:18-alpine as node-builder

WORKDIR /app

# Copy package files
COPY package*.json ./

# Install dependencies
RUN npm ci

# Copy source files first
COPY index.html ./
COPY main.js ./
COPY vite.config.js ./

# Copy WASM build from rust-builder stage
COPY --from=rust-builder /app/pkg ./pkg

# Build the frontend (skip wasm build since we already have pkg/)
RUN npx vite build

# Production stage with simple HTTP server
FROM node:18-alpine

WORKDIR /app

# Copy built files
COPY --from=node-builder /app/dist ./dist

# Create simple server script
RUN echo 'const http = require("http");' > server.js && \
    echo 'const fs = require("fs");' >> server.js && \
    echo 'const path = require("path");' >> server.js && \
    echo 'const port = process.env.PORT || 3000;' >> server.js && \
    echo 'const server = http.createServer((req, res) => {' >> server.js && \
    echo '  let filePath = path.join(__dirname, "dist", req.url === "/" ? "index.html" : req.url);' >> server.js && \
    echo '  const ext = path.extname(filePath);' >> server.js && \
    echo '  const contentType = {' >> server.js && \
    echo '    ".html": "text/html", ".js": "application/javascript", ".css": "text/css",' >> server.js && \
    echo '    ".wasm": "application/wasm", ".json": "application/json"' >> server.js && \
    echo '  }[ext] || "text/plain";' >> server.js && \
    echo '  fs.readFile(filePath, (err, data) => {' >> server.js && \
    echo '    if (err) { res.writeHead(404); res.end("Not found"); return; }' >> server.js && \
    echo '    res.writeHead(200, { "Content-Type": contentType });' >> server.js && \
    echo '    res.end(data);' >> server.js && \
    echo '  });' >> server.js && \
    echo '});' >> server.js && \
    echo 'server.listen(port, "0.0.0.0", () => console.log(`Server running on port ${port}`));' >> server.js

# Expose port
EXPOSE 3000

# Start command
CMD ["node", "server.js"] 