FROM rust:latest as builder

LABEL maintainer="Fabien Bellanger <valentil@gmail.com>"

RUN apt-get update \
    && apt-get -y install ca-certificates cmake libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy
# ----
COPY ./src src
COPY ./Cargo.toml Cargo.toml

# Build
# -----
ENV PKG_CONFIG_ALLOW_CROSS=1

# RUN cargo build
RUN cargo build

# =============================================================================

FROM gcr.io/distroless/cc AS runtime

WORKDIR /app

COPY --from=builder /app/target/debug/rust-opentelemetry .

EXPOSE 3333
ENTRYPOINT ["./rust-opentelemetry"]
