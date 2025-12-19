# Stage 1: Chef - Compute recipe
FROM rust:1.92.0-slim-bookworm AS chef
RUN cargo install cargo-chef
WORKDIR /app

# Install system dependencies required for building (C compiler, pkg-config, etc.)
# Even with bundled SQLite, some crates might need system tools or SSL.
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Stage 2: Planner - Create lockfile
FROM chef AS planner
COPY . .
# Prepare backend recipe
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: Builder - Build dependencies and application
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build backend dependencies
RUN cargo chef cook --release --recipe-path recipe.json
# Build backend application
COPY . .
RUN cargo build --release --bin kana-tutor

# Stage 4: Frontend Builder
FROM rust:1.92.0-slim-bookworm AS frontend-builder
# Install trunk
RUN cargo install --locked trunk
# Install wasm-bindgen-cli
RUN cargo install --locked wasm-bindgen-cli
# Add wasm target
RUN rustup target add wasm32-unknown-unknown
WORKDIR /app
# Copy frontend source
COPY ./frontend ./frontend
# Build frontend
RUN cd frontend && trunk build --release

# Stage 5: Runtime - Minimal image
FROM gcr.io/distroless/cc-debian12
WORKDIR /app

# Copy backend executable
COPY --from=builder /app/target/release/kana-tutor /app/kana-tutor
# Copy frontend assets
COPY --from=frontend-builder /app/frontend/dist /app/frontend/dist
# Copy database
COPY ./kana.db /app/kana.db


# Ensure /app is a volume for persistence (kana.db created here)
VOLUME ["/app"]

ENTRYPOINT ["./kana-tutor"]
