# syntax=docker/dockerfile:1
# phase-mirror-agent — multi-stage build.
# Build context: repository root (so the internal Prime/packages/rust pirtm crates resolve).
#   docker build -t phase-mirror-agent -f packages/phase-mirror-agent/Dockerfile .

FROM rust:1.85-slim AS builder
WORKDIR /build

# The agent crate is its own workspace root; pin the target dir so the binary
# always lands at /build/target regardless of workspace layout.
ENV CARGO_TARGET_DIR=/build/target

# Copy manifests + lockfile first to cache dependency compilation.
COPY packages/phase-mirror-agent/Cargo.toml packages/phase-mirror-agent/
COPY packages/phase-mirror-agent/Cargo.lock packages/phase-mirror-agent/
COPY packages/phase-mirror-agent/src packages/phase-mirror-agent/src

# Internal pirtm crate chain (pirtm-apps -> registry/dist/invariants -> stdlib).
COPY Prime/packages/rust/pirtm-apps/Cargo.toml Prime/packages/rust/pirtm-apps/
COPY Prime/packages/rust/pirtm-apps/src Prime/packages/rust/pirtm-apps/src
COPY Prime/packages/rust/pirtm-registry/Cargo.toml Prime/packages/rust/pirtm-registry/
COPY Prime/packages/rust/pirtm-registry/src Prime/packages/rust/pirtm-registry/src
COPY Prime/packages/rust/pirtm-dist/Cargo.toml Prime/packages/rust/pirtm-dist/
COPY Prime/packages/rust/pirtm-dist/src Prime/packages/rust/pirtm-dist/src
COPY Prime/packages/rust/pirtm-invariants/Cargo.toml Prime/packages/rust/pirtm-invariants/
COPY Prime/packages/rust/pirtm-invariants/src Prime/packages/rust/pirtm-invariants/src
COPY Prime/packages/rust/pirtm-stdlib/Cargo.toml Prime/packages/rust/pirtm-stdlib/
COPY Prime/packages/rust/pirtm-stdlib/src Prime/packages/rust/pirtm-stdlib/src

# Pre-fetch dependencies so source changes don't recompile the whole graph.
RUN cargo fetch --manifest-path packages/phase-mirror-agent/Cargo.toml

# Build the release binary.
RUN cargo build --release --manifest-path packages/phase-mirror-agent/Cargo.toml -p phase-mirror-agent

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates tzdata curl \
    && rm -rf /var/lib/apt/lists/*

# Non-root runtime user (ADR-009 §2.1 least privilege).
RUN groupadd --gid 65532 nonroot \
    && useradd --uid 65532 --gid 65532 --no-create-home --shell /usr/sbin/nologin nonroot

# Durable runtime state lives here; must be writable by the non-root user.
RUN mkdir -p /var/lib/phase-mirror/state \
    && chown -R nonroot:nonroot /var/lib/phase-mirror
WORKDIR /var/lib/phase-mirror

COPY --from=builder /build/target/release/phase-mirror-agent /usr/local/bin/phase-mirror-agent

USER nonroot

# HTTP API + operator WebSocket broadcast.
EXPOSE 8080 3030

# Readiness gate (ADR-007 §2.3). Overridden in compose/systemd when TLS is
# enabled (the check must then use https).
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD curl -fsS http://localhost:8080/ready || exit 1

# Graceful stop: SIGTERM triggers the ADR-007 drain (HTTP -> WS -> WAL flush)
# before the container manager falls back to SIGKILL.
STOPSIGNAL SIGTERM

# TLS support: mount a PEM cert/key and set PHASE_MIRROR_TLS_CERT/KEY
# (or pass --tls-cert/--tls-key). In production mode plain HTTP requires
# an explicit PHASE_MIRROR_NO_TLS=true / --no-tls.
ENTRYPOINT ["phase-mirror-agent"]
