# The SS2RS dedicated server (docs/DEDICATED_SERVER.md).
#
#   docker build -t ss2-server .
#   docker run --rm -p 27960:27960/udp -v "$PWD/server.toml:/config/server.toml:ro" ss2-server --config /config/server.toml
#
# The image holds the server binary and nothing else: **no game data is ever copied into it**. Put the
# player's own Serious Sam 2 install in a volume and point `data_dir` at it.

FROM rust:1.99.1-slim AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates ./crates
COPY xtask ./xtask
COPY tools ./tools
# The reproducible release flags of scripts/dist-flags.sh are for the release archives; an image
# build only needs the locked dependency versions.
RUN cargo build --profile dist --locked -p ss2-server

FROM debian:bookworm-slim
RUN useradd --system --create-home --home-dir /home/ss2 --shell /usr/sbin/nologin ss2 \
    && mkdir -p /config /data /logs \
    && chown ss2:ss2 /data /logs
COPY --from=build /src/target/dist/ss2-server /usr/local/bin/ss2-server
USER ss2
WORKDIR /home/ss2
# The default listening port (UDP for the game once the transport exists) and the optional metrics
# port. Nothing listens until the configuration asks for it; see the documentation.
EXPOSE 27960/udp 9100/tcp
VOLUME ["/data", "/logs"]
ENTRYPOINT ["/usr/local/bin/ss2-server"]
