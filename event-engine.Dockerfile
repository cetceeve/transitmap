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
COPY ./rust-crates/event-engine/ ./rust-crates/event-engine/
RUN touch ./rust-crates/event-engine/src/main.rs

# build, with dependency cache
RUN --mount=type=cache,target=/usr/local/cargo/registry,id="reg-${TARGETPLATFORM}" \
    --mount=type=cache,target=target,id="target-event-engine-${TARGETPLATFORM}" \
    cargo build --release && \
    mkdir bin && \
    mv target/release/event-engine bin/event-engine

# our final base
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl-dev && rm -rf /var/lib/apt/lists/*
COPY --from=build /workspace/bin/event-engine /usr/local/bin/event-engine
CMD event-engine
