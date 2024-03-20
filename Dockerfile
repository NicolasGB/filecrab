## CHEF
FROM alpine:edge AS chef
RUN apk update
RUN apk --no-cache add cargo rust openssl-dev clang16-libclang 
RUN apk add --update --no-cache build-base cmake 
RUN apk add --update --no-cache  linux-headers snappy-dev
RUN cargo install cargo-chef

## PLANNER
FROM chef AS planner
WORKDIR /app
COPY . /app
RUN cargo chef prepare --recipe-path recipe.json

## BUILDER
FROM chef AS builder
ARG BUILD_MODE
ARG FEATURES
WORKDIR /app
# Copy over the recipes
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN if [ "$BUILD_MODE" = "release" ]; then \
    cargo chef cook --package filecrab-server --release --features="${FEATURES}" --recipe-path recipe.json; \
else \
    cargo chef cook --package filecrab-server --features="${FEATURES}" --recipe-path recipe.json; \
fi
COPY . /app
# Build the application
RUN if [ "$BUILD_MODE" = "release" ]; then \
    cargo build --package filecrab-server --features="${FEATURES}" --release; \
else \
    cargo build --package filecrab-server --features="${FEATURES}"; \
fi

## RUNTIME
FROM alpine:edge AS runtime
ARG BUILD_MODE
# Install the runtime dependencies
RUN apk update
RUN apk --no-cache add libgcc openssl ca-certificates snappy 
COPY --from=builder /app/target/${BUILD_MODE}/filecrab-server /filecrab
CMD ["/filecrab", "server"]
