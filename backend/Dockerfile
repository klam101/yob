# syntax=docker/dockerfile:1
# Builds one backend binary, picked with the BIN build arg (api or worker).

FROM rust:1.100-slim-trixie AS build
ARG BIN
WORKDIR /app
COPY . .
# Cache downloaded crates and build output between builds.
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/app/target,sharing=locked \
    cargo build --release --locked --bin "${BIN}" \
 && cp "target/release/${BIN}" /usr/local/bin/app

FROM debian:trixie-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/*
COPY --from=build /usr/local/bin/app /usr/local/bin/app
USER nobody
CMD ["app"]
