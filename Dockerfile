FROM --platform=$BUILDPLATFORM rust:1.58-slim-buster AS builder

RUN apt-get update -qq && apt-get -qqy install pkg-config libssl-dev && rm -rf /var/cache/apt/* /var/lib/apt/*

WORKDIR /work

COPY . .

RUN cargo build --release

FROM debian:buster-slim AS release

RUN apt-get update -qq && apt-get -qqy install openssl && rm -rf /var/cache/apt/* /var/lib/apt/*

COPY --from=builder /work/target/release/adsb_exporter /usr/local/bin/adsb_exporter

EXPOSE 9190/tcp

ENTRYPOINT ["/usr/local/bin/adsb_exporter"]
