FROM rust:latest

COPY wither /wither
COPY Cargo.toml Cargo.lock /

ARG RELEASE_FLAG="--release"
ENV RELEASE_FLAG_ENV=$RELEASE_FLAG

# First build the dependencies, such that we do not need to rebuild them whenever any of our code changes.
RUN mkdir -p /src && touch /src/lib.rs
RUN cargo build $RELEASE_FLAG_ENV --lib

# Now copy in our code, and build it without rebuilding dependencies.
RUN rm -rf /src
COPY src /src
RUN cargo build $RELEASE_FLAG_ENV --bins

ENTRYPOINT cargo run $RELEASE_FLAG_ENV