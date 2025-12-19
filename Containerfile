# Stage 1: Builder
FROM rust:1.83-bookworm AS builder

# Install Trunk and WASM target
RUN rustup target add wasm32-unknown-unknown && \
    cargo install trunk

# Create app directory
WORKDIR /usr/src/app

# Copy source code
COPY . .

# Build Frontend
WORKDIR /usr/src/app/frontend
# Release build for frontend (generates ./dist)
RUN trunk build --release

# Build Backend
WORKDIR /usr/src/app
# Release build for backend
RUN cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim

# Install runtime dependencies (OpenSSL, SQLite)
RUN apt-get update && \
    apt-get install -y libsqlite3-0 ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Set working directory for the application (DB will be created here)
WORKDIR /app

# Copy Backend Binary
COPY --from=builder /usr/src/app/target/release/kana-tutor /usr/local/bin/kana-tutor

# Copy Frontend Assets
COPY --from=builder /usr/src/app/frontend/dist /app/frontend/dist

# Environment configuration
ENV RUST_LOG=info
# Port to expose
EXPOSE 3000

# Command to run the application in web mode
CMD ["kana-tutor", "--web"]
