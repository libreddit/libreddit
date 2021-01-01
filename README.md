# Libreddit

> An alternative private front-end to Reddit 

Libre + Reddit = [Libreddit](https://libredd.it)

- 🚀 Fast: written in Rust for blazing fast speeds and safety
- ☁️ Light: no JavaScript, no ads, no tracking
- 🕵 Private: all requests are proxied through the server, including media
- 🦺 Safe: does not rely on Reddit OAuth or require a Reddit API Key 
- 🔒 Secure: strong [Content Security Policy](https://developer.mozilla.org/en-US/docs/Web/HTTP/CSP) prevents browser requests to Reddit

Like [Invidious](https://github.com/iv-org/invidious) but for Reddit. Browse the coldest takes of [r/unpopularopinion](https://libredd.it/r/unpopularopinion) without being [tracked](#reddit).

## Contents
- [Screenshot](#screenshot)
- [Instances](#instances)
- [About](#about)
  - [Elsewhere](#elsewhere)
  - [Info](#info)
  - [Teddit Comparison](#how-does-it-compare-to-teddit)
- [Comparison](#comparison)
  - [Speed](#speed)
  - [Privacy](#privacy)
- [Installation](#installation)
  - [Cargo](#a-cargo)
  - [Docker](#b-docker)
  - [AUR](#c-aur)
  - [GitHub Releases](#d-github-releases)
  - [Repl.it](#e-replit)
- Developing
  - [Deployment](#deployment)
  - [Building](#building)

## Screenshot

![](https://i.ibb.co/vLhBdL4/libreddit-rust.png)

## Instances

Feel free to [open an issue](https://github.com/spikecodes/libreddit/issues/new) to have your [selfhosted instance](#deployment) listed here!

| Website | Country | Cloudflare |
|-|-|-|
| [libredd.it](https://libredd.it) (official) | 🇺🇸 US  |  |
| [libreddit.spike.codes](https://libreddit.spike.codes) (official) | 🇺🇸 US  |  |
| [libreddit.dothq.co](https://libreddit.dothq.co) | 🇺🇸 US  | ✅ |
| [libreddit.insanity.wtf](https://libreddit.insanity.wtf) | 🇺🇸 US  | ✅ |
| [libreddit.kavin.rocks](https://libreddit.kavin.rocks) | 🇮🇳 IN  | ✅ |
| [spjmllawtheisznfs7uryhxumin26ssv2draj7oope3ok3wuhy43eoyd.onion](http://spjmllawtheisznfs7uryhxumin26ssv2draj7oope3ok3wuhy43eoyd.onion) | 🇮🇳 IN  |  |

A checkmark in the "Cloudflare" category here refers to the use of the reverse proxy, [Cloudflare](https://cloudflare). The checkmark will not be listed for a site which uses Cloudflare DNS but rather the proxying service which grants Cloudflare the ability to monitor traffic to the website.

## About

### Elsewhere
Find Libreddit on...
- 💬 Matrix: [#libreddit:matrix.org](https://matrix.to/#/#libreddit:matrix.org)
- 🐋 Docker: [spikecodes/libreddit](https://hub.docker.com/r/spikecodes/libreddit)
- :octocat: GitHub: [spikecodes/libreddit](https://github.com/spikecodes/libreddit)
- 🦊 GitLab: [spikecodes/libreddit](https://gitlab.com/spikecodes/libreddit)

### Info
Libreddit hopes to provide an easier way to browse Reddit, without the ads, trackers, and bloat. Libreddit was inspired by other alternative front-ends to popular services such as [Invidious](https://github.com/iv-org/invidious) for YouTube, [Nitter](https://github.com/zedeus/nitter) for Twitter, and [Bibliogram](https://sr.ht/~cadence/bibliogram/) for Instagram.

Libreddit currently implements most of Reddit's functionalities but still lacks a few features that are being worked on below.

### How does it compare to Teddit?

Teddit is another awesome open source project designed to provide an alternative frontend to Reddit. There is no connection between the two and you're welcome to use whichever one you favor. Competition fosters innovation and Teddit's release has motivated me to build Libreddit into an even more polished product.

If you are looking to compare, the biggest differences I have noticed are:
- Libreddit is themed around Reddit's redesign whereas Teddit appears to stick much closer to Reddit's old design. This may suit some users better as design is always subjective.
- Libreddit is written in [Rust](https://www.rust-lang.org) for speed and memory safety. It uses [Actix Web](https://actix.rs), which was [benchmarked as the fastest web server for single queries](https://www.techempower.com/benchmarks/#hw=ph&test=db).

## Comparison

This section outlines how Libreddit compares to Reddit.

### Speed

Lasted tested December 21, 2020.

Results from Google Lighthouse ([Libreddit Report](https://lighthouse-dot-webdotdevsite.appspot.com/lh/html?url=https%3A%2F%2Flibredd.it), [Reddit Report](https://lighthouse-dot-webdotdevsite.appspot.com/lh/html?url=https%3A%2F%2Fwww.reddit.com%2F)).

|                     | Libreddit     | Reddit    |
|---------------------|---------------|-----------|
| Requests            | 22            | 70        |
| Resource Size       | 135 KiB       | 2,222 KiB |
| Time to Interactive | **1.7 s**     | **11.5 s**|

### Privacy

#### Reddit

**Logging:** According to Reddit's [privacy policy](https://www.redditinc.com/policies/privacy-policy), they "may [automatically] log information" including:
- IP address
- User-agent string
- Browser type
- Operating system
- Referral URLs
- Device information (e.g., device IDs)
- Device settings
- Pages visited
- Links clicked
- The requested URL
- Search terms

**Location:** The same privacy policy goes on to describe location data may be collected through the use of:
- GPS (consensual)
- Bluetooth (consensual)
- Content associated with a location (consensual)
- Your IP Address

**Cookies:** Reddit's [cookie notice](https://www.redditinc.com/policies/cookies) documents the array of cookies used by Reddit including/regarding:
- Authentication
- Functionality
- Analytics and Performance
- Advertising
- Third-Party Cookies
- Third-Party Site

#### Libreddit

For transparency, I hope to describe all the ways Libreddit handles user privacy.

**Logging:** In production (when running the binary, hosting with docker, or using the official instances), Libreddit logs nothing. When debugging (running from source without `--release`), Libreddit logs post IDs and URL paths fetched to aid troubleshooting but nothing else.

**DNS:** Both official domains (`libredd.it` and `libreddit.spike.codes`) use Cloudflare as the DNS resolver. Though, the sites are not proxied through Cloudflare meaning Cloudflare doesn't have access to user traffic.

**Cookies:** Libreddit uses no cookies currently but eventually, I plan to add a configuration page where users can store an optional cookie to save their preferred theme, default sorting algorithm, or default layout.

**Hosting:** The official instances (`libredd.it` and `libreddit.spike.codes`) are hosted on [Repl.it](https://repl.it/) which monitors usage to prevent abuse. I can understand if this invalidates certain users' threat models and therefore, selfhosting and browsing through Tor are welcomed.

## Installation

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

For ArchLinux users, Libreddit is available from the AUR as [`libreddit-git`](https://aur.archlinux.org/packages/libreddit-git).

Install:
```
yay -S libreddit-git
```

### D) GitHub Releases

If you're on Linux and none of these methods work for you, you can grab a Linux binary from [the newest release](https://github.com/spikecodes/libreddit/releases/latest).
Currently, Libreddit does not have Windows or macOS binaries but those will be available soon.

### E) Repl.it

**Note:** Repl.it is a free option but they are *not* private and are monitor server usage to prevent abuse. If you really need a free and easy setup, this method may work best for you.

1. Create a Repl.it account (see note above)
2. Visit [the official Repl](https://repl.it/@spikethecoder/libreddit) and fork it
3. Hit the run button to download the latest Libreddit version and start it

In the web preview (defaults to top right), you should see your instance hosted where you can assign a [custom domain](https://docs.repl.it/repls/web-hosting#custom-domains).

## Deployment

Once installed, deploy Libreddit (unless you're using Docker) by running:

```
libreddit
```

Specify a custom address for the server by passing the `-a` or `--address` argument:
```
libreddit --address=0.0.0.0:8111
```

To disable the media proxy built into Libreddit, run:
```
libreddit --no-default-features
```

## Building

```
git clone https://github.com/spikecodes/libreddit
cd libreddit
cargo run
```
