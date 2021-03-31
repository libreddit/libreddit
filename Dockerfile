FROM rust:alpine as builder
WORKDIR /usr/src/libreddit
COPY . .
RUN apk add --no-cache g++
RUN cargo install --path .

FROM alpine:latest
RUN apk add --no-cache curl
COPY --from=builder /usr/local/cargo/bin/libreddit /usr/local/bin/libreddit
EXPOSE 8080
HEALTHCHECK --interval=5m --timeout=3s CMD curl -f http://localhost:8080/settings || exit 1
CMD ["libreddit"]