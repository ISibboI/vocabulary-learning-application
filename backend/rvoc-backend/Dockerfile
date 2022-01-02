FROM rust:latest

COPY src /src
COPY wither /wither
COPY Cargo.toml Cargo.lock /

ARG RELEASE_FLAG="--release"

RUN cargo build $RELEASE_FLAG --bins

ENTRYPOINT ["cargo", "run", "$RELEASE_FLAG"]