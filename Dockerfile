FROM rust:latest as builder

WORKDIR /usr/src/libreddit
COPY . .
RUN cargo install --path .


FROM debian:buster-slim

RUN apt-get update && apt-get install -y libcurl4 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/libreddit /usr/local/bin/libreddit
RUN useradd --system --user-group --home-dir /nonexistent --no-create-home --shell /usr/sbin/nologin libreddit
USER libreddit

EXPOSE 8080

CMD ["libreddit"]
