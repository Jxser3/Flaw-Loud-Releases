FROM rust:1-bookworm AS builder
WORKDIR /app
COPY source.tar.gz /tmp/source.tar.gz
RUN tar -xzf /tmp/source.tar.gz -C /app && cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/flaw-loud-platform /usr/local/bin/flaw-loud-platform
ENV FLAW_DATA_DIR=/data
VOLUME ["/data"]
EXPOSE 8787
CMD ["flaw-loud-platform"]
