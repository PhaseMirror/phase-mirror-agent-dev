# syntax=docker/dockerfile:1
FROM rust:1.78 AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12
COPY --from=builder /app/target/release/phase-mirror-mcp /usr/local/bin/phase-mirror-mcp
ENTRYPOINT ["/usr/local/bin/phase-mirror-mcp"]
