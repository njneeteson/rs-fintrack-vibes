# syntax=docker/dockerfile:1
#
# FinTrack server image.
#
#   docker build -t fintrack .
#
# Stage 1 compiles the server; stage 2 is a minimal runtime (no shell, no
# package manager) that runs as an unprivileged user. Household data lives in
# the /data volume.

FROM rust:1-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock rustfmt.toml ./
COPY crates crates
# Cache the crate registry and build artefacts between builds.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked -p fintrack-server \
    && cp target/release/fintrack-server /fintrack-server
# An empty data directory, owned by the runtime user, so a fresh named volume
# starts out writable.
RUN mkdir /data

FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=build /fintrack-server /usr/local/bin/fintrack-server
COPY --from=build --chown=nonroot:nonroot /data /data

ENV FINTRACK_DATA_DIR=/data \
    FINTRACK_BIND=0.0.0.0:8080
VOLUME /data
EXPOSE 8080
USER nonroot

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD ["/usr/local/bin/fintrack-server", "healthcheck"]
ENTRYPOINT ["/usr/local/bin/fintrack-server"]
CMD ["serve"]
