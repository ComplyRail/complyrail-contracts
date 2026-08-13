FROM rust:latest

RUN rustup target add wasm32-unknown-unknown && \
    cargo install --locked stellar-cli && \
    apt-get update && apt-get install -y curl

WORKDIR /workspace

CMD ["bash", "-c", "cargo build --release --target wasm32-unknown-unknown"]
