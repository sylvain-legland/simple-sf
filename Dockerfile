# Stage 1: Build Rust
FROM rust:1.82-slim AS rust-builder
WORKDIR /app
COPY SFEngine/ ./SFEngine/
RUN cd SFEngine && cargo build --release

# Stage 2: Build Server
FROM rust:1.82-slim AS server-builder
WORKDIR /app
COPY SimpleSFServer/ ./SimpleSFServer/
RUN cd SimpleSFServer && cargo build --release

# Stage 3: Runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=server-builder /app/SimpleSFServer/target/release/simple-sf-server /usr/local/bin/
EXPOSE 8099
ENV JWT_SECRET=changeme
ENV CORS_ORIGINS=http://localhost:3000
CMD ["simple-sf-server"]
