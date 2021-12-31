FROM rust:latest

COPY src /src
COPY wither /wither
COPY Cargo.toml Cargo.lock /

RUN cargo build --release --bins

ENTRYPOINT ["cargo", "run", "--release"]