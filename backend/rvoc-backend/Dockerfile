FROM rust:latest

COPY src /src
COPY wither /wither
COPY Cargo.toml Cargo.lock /

ARG RELEASE_FLAG="--release"
ENV RELEASE_FLAG_ENV=$RELEASE_FLAG

RUN cargo build $RELEASE_FLAG_ENV --bins

ENTRYPOINT cargo run "$RELEASE_FLAG_ENV"