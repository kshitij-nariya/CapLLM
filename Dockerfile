# syntax=docker/dockerfile:1

FROM rust:alpine AS builder
WORKDIR /app

# Install build dependencies for Alpine/musl
RUN apk add --no-cache musl-dev

# Copy the entire workspace
COPY . .

# Build the server optimized for release
RUN cargo build --release -p capllm-server

# Strip the binary to reduce size
RUN strip target/release/capllm-server

# ── Runtime Stage ─────────────────────────────────────────────────────────────
FROM alpine:latest
WORKDIR /app

# Install CA certificates for HTTPS requests to upstream providers
RUN apk add --no-cache ca-certificates

# Copy the stripped binary
COPY --from=builder /app/target/release/capllm-server /usr/local/bin/capllm-server

EXPOSE 3000
CMD ["capllm-server"]
