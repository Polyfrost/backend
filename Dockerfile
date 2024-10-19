FROM clux/muslrust:stable AS chef

USER root
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner

COPY . .

RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder

COPY --from=planner /app/recipe.json recipe.json

RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json

COPY . .

RUN rustup target add x86_64-unknown-linux-musl && cargo build --release --target x86_64-unknown-linux-musl

FROM alpine AS runtime
RUN addgroup -S polyfrost && adduser -S polyfrost -G polyfrost
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/backend /usr/local/bin/
USER polyfrost
CMD ["/usr/local/bin/backend"]