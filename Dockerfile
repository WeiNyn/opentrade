# Stage 1: Build the Rust application
FROM rust:1.88-slim-bookworm AS builder

# Install necessary dependencies
RUN apt-get update && \
    apt-get install -y \
    libssl-dev \
    pkg-config \
    build-essential \
    postgresql-client \
    && rm -rf /var/lib/apt/lists/*

# Set the working directory inside the container
WORKDIR /app

# Now copy the actual source code
COPY . .

# Build the release binary
RUN DATABASE_URL=postgres://postgres:password@localhost/postgres cargo build --release --package opentrade-pipeline

# Stage 2: Create the final image
# Use a minimal base image to reduce size
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y \
    openssl \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/streaming_klines /app/streaming_klines
COPY --from=builder /app/target/release/backfill_klines /app/backfill_klines

CMD [ "/app/streaming_klines" ]