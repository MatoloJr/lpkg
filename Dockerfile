# syntax=docker/dockerfile:1

FROM rust:1-bookworm AS builder
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY data ./data
COPY tests ./tests
RUN cargo build --release

FROM debian:bookworm-slim
LABEL org.opencontainers.image.source="https://github.com/MatoloJr/lpkg"
LABEL org.opencontainers.image.url="https://github.com/MatoloJr/lpkg"
LABEL org.opencontainers.image.description="A winget-like package orchestrator for Ubuntu/Debian Linux"
LABEL org.opencontainers.image.licenses="MIT"
LABEL org.opencontainers.image.title="lpkg"

RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/*

COPY --from=builder /src/target/release/lpkg /usr/local/bin/lpkg
COPY --from=builder /src/data/aliases.toml /usr/local/share/lpkg/aliases.toml
COPY --from=builder /src/data/direct-deb-index.json /usr/local/share/lpkg/direct-deb-index.json

ENTRYPOINT ["lpkg"]
CMD ["--help"]
