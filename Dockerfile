####################################################################################################
## Builder
####################################################################################################
FROM rust:alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /libbacon

COPY . .

RUN cargo build --target x86_64-unknown-linux-musl --release

####################################################################################################
## Final image
####################################################################################################
FROM alpine:latest

# Import ca-certificates from builder
COPY --from=builder /usr/share/ca-certificates /usr/share/ca-certificates
COPY --from=builder /etc/ssl/certs /etc/ssl/certs

# Copy our build
COPY --from=builder /libbacon/target/x86_64-unknown-linux-musl/release/libbacon /usr/local/bin/libbacon

# Use an unprivileged user.
RUN adduser --home /nonexistent --no-create-home --disabled-password libbacon
USER libbacon

# Tell Docker to expose port 8080
EXPOSE 8080

# Run a healthcheck every minute to make sure libbacon is functional
HEALTHCHECK --interval=1m --timeout=3s CMD wget --spider --q http://localhost:8080/settings || exit 1

CMD ["libbacon"]
