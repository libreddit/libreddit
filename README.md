# Libreddit

> An alternative private front-end to Reddit 

Libre + Reddit = Libreddit

- 🚀 Fast: written in Rust for blazing fast speeds and safety
- ☁️ Light: no javascript, no ads, no tracking
- 🕵 Private: ~~all~~ most requests are proxied through the server (images are still loaded from Reddit)
- 🔒 Safe: does not rely on Reddit's OAuth-requiring APIs 
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
  - [x] Flairs
  - [x] Comments
    - [x] Comment sorting
    - [ ] Nested comments
  - [x] UTC post date
  - [x] Image thumbnails
  - [x] Embedded images
    - [ ] Proxied images 
  - [x] Reddit-hosted video
    - [ ] Proxied video
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

Once installed, deploy Libreddit (unless you're using Docker) by running:

```
libreddit
```

Specify a custom address for the server by passing the `-a` or `--address` argument:
```
libreddit --address=0.0.0.0:8111
```

### A) Cargo

Make sure Rust stable is installed along with `cargo`, Rust's package manager.

```
cargo install libreddit
```

### B) Docker

Deploy the Docker image of Libreddit:
```
docker run -d --name libreddit -p 8080:8080 spikecodes/libreddit
```

Deploy using a different port (in this case, port 80):
```
docker run -d --name libreddit -p 80:8080 spikecodes/libreddit
```

### C) AUR

Libreddit is available from the Arch User Repository as [`libreddit-git`](https://aur.archlinux.org/packages/libreddit-git).

Install:
```
yay -S libreddit-git
```

## Building

```
git clone https://github.com/spikecodes/libreddit
cd libreddit
cargo run
```