# Build stage
FROM rust:1.75 as builder

# Install wasm-pack for building the WASM client
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Install Node.js for frontend build
RUN curl -fsSL https://deb.nodesource.com/setup_18.x | bash - && \
    apt-get install -y nodejs

WORKDIR /app

# Copy all source files
COPY . .

# Build the WASM client
RUN wasm-pack build --target web --out-dir pkg

# Install frontend dependencies and build
RUN npm ci
RUN npm run build

# Build the Rust server
RUN cargo build --release --features server --bin server

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the built server binary
COPY --from=builder /app/target/release/server /usr/local/bin/server

# Copy the built frontend files
COPY --from=builder /app/dist ./dist

# Set environment variables
ENV STATIC_PATH=./dist

# Expose the port (Railway will set PORT env var)
EXPOSE 8080

# Start the server
CMD ["server"] 