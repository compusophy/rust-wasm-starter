# Use official Rust image as base
FROM rust:1.75 as rust-builder

# Install wasm-pack
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Set working directory
WORKDIR /app

# Copy Rust files
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/

# Build WASM
RUN wasm-pack build --target web --out-dir pkg

# Use Node.js for the frontend build
FROM node:18-alpine as node-builder

WORKDIR /app

# Copy package files
COPY package*.json ./

# Install dependencies
RUN npm ci

# Copy source files and WASM build from previous stage
COPY . .
COPY --from=rust-builder /app/pkg ./pkg

# Build the frontend
RUN npm run build

# Production stage with simple HTTP server
FROM node:18-alpine

WORKDIR /app

# Install serve globally
RUN npm install -g serve

# Copy built files
COPY --from=node-builder /app/dist ./dist

# Expose port
EXPOSE 3000

# Start command
CMD ["serve", "-s", "dist", "-l", "3000"] 