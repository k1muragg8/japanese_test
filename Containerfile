# Stage 1: Chef - Compute recipe
FROM rust:1.83-slim-bookworm AS chef
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
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: Builder - Build dependencies and application
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --bin kana-tutor

# Stage 4: Runtime - Minimal image
# gcr.io/distroless/cc-debian12 contains glibc and libssl/openssl needed for runtime
FROM gcr.io/distroless/cc-debian12
COPY --from=builder /app/target/release/kana-tutor /app/kana-tutor
WORKDIR /app

# Ensure /app is a volume for persistence (kana.db created here)
VOLUME ["/app"]

ENTRYPOINT ["./kana-tutor"]
