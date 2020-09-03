# Libreddit

Libre + Reddit = Libreddit

![](https://i.ibb.co/CsYPXJ8/image.png)

## Features

- 🚀 Fast: written in Rust for blazing fast speeds and safety
- ☁️ Light: uses no javascript or ads
- 🕵 Private: ~~all~~ most requests are proxied through the server (images are still loaded from Reddit)
- 🔒 Safe: does not rely on Reddit's closed APIs 
- 📱 Responsive: works great on mobile!

Think Invidious but for Reddit. Watch your cat videos without being watched.

**Note**: Libreddit is still a WIP.

## Instances

- [libreddit.spike.codes](https://libreddit.spike.codes) 🇺🇸

## Deploy an Instance

### A) Manual

Make sure you have [Rust](https://rust-lang.org) installed first or else `cargo` commands won't work.Libreddit uses Rocket for the web server and as of now, Rocket requires Rust Nightly.

```
rustup default nightly
```

Deploy using these commands:

```
git clone https://github.com/spikecodes/libreddit.git
cd libreddit
cargo run
```

<!-- ### B) Repl.it

[![Run on Repl.it](https://repl.it/badge/github/spikecodes/libreddit)](https://repl.it/github/spikecodes/libreddit)

Provides:
- Free deployment of app (can be ran without account)
- Free HTTPS url (https://\<app name\>.\<username\>\.repl\.co)
    - Supports custom domains
- Downtime after periods of inactivity \([solution 1](https://repl.it/talk/ask/use-this-pingmat1replco-just-enter/28821/101298), [solution 2](https://repl.it/talk/learn/How-to-use-and-setup-UptimeRobot/9003)\) -->
