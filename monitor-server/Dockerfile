# syntax=docker/dockerfile:1.6
FROM rust:1.77-slim-bookworm AS builder

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# 复制 workspace 清单以利用构建缓存
COPY Cargo.toml ./
COPY migration/Cargo.toml migration/Cargo.toml

# 预先创建占位源码文件以让 cargo 能够 fetch deps
RUN mkdir -p src migration/src \
    && echo "fn main() {}" > src/main.rs \
    && echo "// placeholder" > src/lib.rs \
    && echo "fn main() {}" > migration/src/main.rs \
    && echo "// placeholder" > migration/src/lib.rs \
    && cargo fetch || true

# 复制真实源码并构建
COPY . .
RUN cargo build --release --bin monitor-server

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/monitor-server /usr/local/bin/monitor-server
EXPOSE 8080 8081
CMD ["monitor-server"]
