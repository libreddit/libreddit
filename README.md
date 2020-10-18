# Libreddit

> An alternative private front-end to Reddit 

Libre + Reddit = Libreddit

- 🚀 Fast: written in Rust for blazing fast speeds and safety
- ☁️ Light: no javascript, no ads, no tracking
- 🕵 Private: ~~all~~ most requests are proxied through the server (images are still loaded from Reddit)
- 🔒 Safe: does not rely on Reddit's closed APIs 
- 📱 Responsive: works great on mobile!

Think Invidious but for Reddit. Watch your cat videos without being watched.

## Screenshot

![](https://i.ibb.co/SfFHDhh/image.png)

## Status

- [x] Hosting
  - [x] Instances
    - [x] Clearnet instance
    - [ ] .onion instance
  - [x] Cargo deployment
  - [x] Docker deployment
- [x] Subreddits
  - [x] Title
  - [x] Description
  - [x] Posts
    - [x] Post sorting
- [x] Posts
  - [x] Comments
    - [x] Comment sorting
    - [ ] Nested comments
  - [x] UTC post date
  - [x] Image thumbnails
  - [x] Embedded images
    - [ ] Proxied images 
  - [x] Reddit-hosted video
    - [ ] Proxied video
  - [ ] Localized post date
- [x] Users
  - [x] Username
  - [x] Karma
  - [x] Description
  - [x] Post history
    - [x] Post sorting
  - [ ] Comment history
    - [ ] Comment sorting

- [ ] Search
  - [ ] Post aggregating
  - [ ] Comment aggregating
  - [ ] Result sorting

## Instances

- [libredd.it](https://libredd.it) 🇺🇸 (Thank you to [YeapGuy](https://github.com/YeapGuy)!)
- [libreddit.spike.codes](https://libreddit.spike.codes) 🇺🇸

## Deploy an Instance

### A) Manual

Make sure you have [Rust](https://rust-lang.org) installed first or else `cargo` commands won't work. Libreddit uses Rocket for the web server and as of now, Rocket requires Rust Nightly.

```
rustup default nightly
```

Deploy using these commands:

```
git clone https://github.com/spikecodes/libreddit.git
cd libreddit
cargo run
```

### B) Docker

Deploy the Docker image of Libreddit:
```
docker run -d --name libreddit -p 8000:8000 spikecodes/libreddit
```

Deploy using a different port (in this case, port 80):
```
docker run -d --name libreddit -p 80:8000 spikecodes/libreddit
```