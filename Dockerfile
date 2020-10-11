FROM rustlang/rust:nightly

WORKDIR /usr/src/libreddit

COPY . .

RUN cargo install --path .

CMD ["libreddit"]
