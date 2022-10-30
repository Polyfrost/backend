FROM rustlang/rust:nightly-alpine as builder

WORKDIR /usr/src/polyfrost-api

# Install build tools
RUN apk add --no-cache g++

# Copy source files
COPY . .

# Build actual binary
RUN cargo build --release --locked --all-features

# ---------------------------------------------------------------------------------------------

FROM alpine:3

COPY --from=builder /usr/src/polyfrost-api/target/release/polyfrost-api /usr/local/bin/polyfrost-api

# Use an unprivileged user
RUN adduser --home /nonexistent --no-create-home --disabled-password polyfrost-api
USER polyfrost-api

HEALTHCHECK --interval=10s --timeout=3s --retries=5 CMD wget --spider --q http://localhost:$PORT/ || exit 1

CMD ["polyfrost-api"]