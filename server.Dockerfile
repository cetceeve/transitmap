FROM rust:bookworm AS build
ARG TARGETPLATFORM

WORKDIR /workspace/

# copy over workspace manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# copy local dependencies
COPY ./rust-crates/types/ ./rust-crates/types/
RUN touch ./rust-crates/types/src/lib.rs

# copy the crate we're building
COPY ./rust-crates/server/ ./rust-crates/server/
RUN touch ./rust-crates/server/src/main.rs
COPY ./website/ ./website/

# build, with dependency cache
RUN --mount=type=cache,target=/usr/local/cargo/registry,id="reg-${TARGETPLATFORM}" \
    --mount=type=cache,target=target,id="target-server-${TARGETPLATFORM}" \
    cargo build --release && \
    mkdir bin && \
    mv target/release/server bin/server

# our final base
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl-dev && rm -rf /var/lib/apt/lists/*
COPY --from=build /workspace/bin/server /usr/local/bin/server
COPY ./website/ ./website/
CMD server
