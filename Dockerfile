# syntax=docker/dockerfile:1

FROM rust:1.89-bookworm AS builder
WORKDIR /app

# Only add the minimum build deps likely needed for reqwest/native-tls.
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Cache dependency resolution first.
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release --locked --bin myanonamouse-mcp

FROM debian:bookworm-slim
WORKDIR /app

# Runtime only needs CA roots for outbound HTTPS to MyAnonamouse.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/myanonamouse-mcp /usr/local/bin/myanonamouse-mcp

EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/myanonamouse-mcp"]
CMD ["--transport", "http", "--http-bind", "0.0.0.0:8080"]
