FROM rust:alpine as builder
WORKDIR /usr/src/libreddit
COPY . .
RUN apk add --no-cache g++ openssl-dev
RUN cargo install --path .

FROM alpine:latest
COPY --from=builder /usr/local/cargo/bin/libreddit /usr/local/bin/libreddit
CMD ["libreddit"]