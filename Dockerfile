FROM alpine:3.20.3 AS builder

WORKDIR /usr/src/polyfrost-api

# Install rustup
RUN wget -O - https://sh.rustup.rs | sh -s -- -y --default-toolchain none
ENV PATH="$PATH:/root/.cargo/bin"

# Use sparse registry because it is significantly faster and this already requires nightly
ENV CARGO_UNSTABLE_SPARSE_REGISTRY=true

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