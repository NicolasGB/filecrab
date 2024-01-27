# chef image with cargo
FROM rust:1.75 AS chef
RUN cargo install cargo-chef

# Planner layer
FROM chef AS planner
WORKDIR /app
COPY . /app
RUN cargo chef prepare --recipe-path recipe.json

# Build layer
From chef AS builder
WORKDIR /app
# Copy over the recipies
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
COPY . /app
# Build the application
RUN cargo build --release 


# RUNTIME IMAGE
FROM gcr.io/distroless/cc-debian12 as runtime
COPY --from=builder /app/target/release/filecrab /filecrab

CMD ["/filecrab"]


