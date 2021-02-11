FROM rust:latest

WORKDIR /usr/src/libreddit
COPY . .

RUN cargo install --path .

CMD ["libreddit"]
