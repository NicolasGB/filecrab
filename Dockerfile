# chef image with cargo
FROM alpine:edge AS chef
RUN apk update
RUN apk --no-cache add cargo rust openssl-dev
RUN cargo install cargo-chef

# Planner layer
FROM chef AS planner
WORKDIR /app
COPY . /app
RUN cargo chef prepare --recipe-path recipe.json

# Build layer
From chef AS builder
WORKDIR /app
# Copy over the recipes
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --package filecrab --recipe-path recipe.json
COPY . /app
# Build the application
RUN cargo build --package filecrab 


# RUNTIME IMAGE
FROM alpine:edge as runtime
RUN apk update
RUN apk --no-cache add libgcc openssl ca-certificates
COPY --from=builder /app/target/debug/filecrab /filecrab

CMD ["/filecrab", "server"]


