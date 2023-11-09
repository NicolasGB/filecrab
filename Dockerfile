From rust:alpine AS builder

WORKDIR /usr/src/filecrab

RUN apk add --no-cache \
    openssh-client \
    ca-certificates \
    musl-dev \
    tzdata \
    openssl-dev 

COPY . .

RUN RUSTFLAGS=-Ctarget-feature=-crt-static cargo build --release && \
    mv target/release/filecrab /filecrab && \
    cargo clean
EXPOSE 8080
CMD ["/filecrab"]


