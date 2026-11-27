FROM rust:bullseye AS builder
WORKDIR /opt/hdmapdl
COPY . .
RUN cargo build --release

FROM debian:bullseye-slim
WORKDIR /opt/hdmapdl
COPY --from=builder /opt/hdmapdl/target/release/hdmapdl /usr/local/bin/

ARG DATA_SOURCES_YAML=/opt/hdmapdl/data-sources.yaml
ENV DATA_SOURCES_YAML=$DATA_SOURCES_YAML

RUN apt-get update && apt-get install -y \
    libfontconfig1-dev \
    libxcursor1-dev \
    libxinerama-dev \
    libxrandr-dev \
    libasound2-dev \
    libudev-dev \
    libxi-dev \
    libgl1-mesa-dev \
    libegl1-mesa-dev \
    libdbus-1-dev \
    libxkbcommon-dev \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

EXPOSE 8080
CMD ["hdmapdl"]
