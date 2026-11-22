FROM rust:1.85.0

ARG STELLAR_CLI_VERSION=22.8.1

RUN rustup target add wasm32v1-none && \
    cargo install --locked stellar-cli --version ${STELLAR_CLI_VERSION} && \
    apt-get update && apt-get install -y curl

WORKDIR /workspace

CMD ["bash", "-c", "cargo build --release --target wasm32v1-none"]
