# Libreddit

> An alternative private front-end to Reddit 

![screenshot](https://i.ibb.co/QYbqTQt/libreddit-rust.png)

---

**10 second pitch:** Libreddit is a portmanteau of "libre" (meaning freedom) and "Reddit". It is a private front-end like [Invidious](https://github.com/iv-org/invidious) but for Reddit. Browse the coldest takes of [r/unpopularopinion](https://libreddit.spike.codes/r/unpopularopinion) without being [tracked](#reddit).

- 🚀 Fast: written in Rust for blazing fast speeds and memory safety
- ☁️ Light: no JavaScript, no ads, no tracking, no bloat
- 🕵 Private: all requests are proxied through the server, including media
- 🔒 Secure: strong [Content Security Policy](https://developer.mozilla.org/en-US/docs/Web/HTTP/CSP) prevents browser requests to Reddit

---

**BTC:** bc1qwyxjnafpu3gypcpgs025cw9wa7ryudtecmwa6y

**XMR:** 45FJrEuFPtG2o7QZz2Nps77TbHD4sPqxViwbdyV9A6ktfHiWs47UngG5zXPcLoDXAc8taeuBgeNjfeprwgeXYXhN3C9tVSR

---

# Instances

Feel free to [open an issue](https://github.com/spikecodes/libreddit/issues/new) to have your [selfhosted instance](#deployment) listed here!

| Website | Country | Cloudflare |
|-|-|-|
| [libredd.it](https://libredd.it) (official) | 🇺🇸 US |  |
| [libreddit.spike.codes](https://libreddit.spike.codes) (official) | 🇺🇸 US |  |
| [libreddit.dothq.co](https://libreddit.dothq.co) | 🇺🇸 US |  |
| [libreddit.kavin.rocks](https://libreddit.kavin.rocks) | 🇮🇳 IN | ✅ |
| [libreddit.bcow.xyz](https://libreddit.bcow.xyz) | 🇺🇸 US |  |
| [libreddit.40two.app](https://libreddit.40two.app) | 🇳🇱 NL |  |
| [reddit.invak.id](https://reddit.invak.id) | 🇧🇬 BG |  |
| [reddit.phii.me](https://reddit.phii.me) | 🇺🇸 US |  |
| [lr.riverside.rocks](https://lr.riverside.rocks) | 🇺🇸 US |  |
| [libreddit.silkky.cloud](https://libreddit.silkky.cloud) | 🇫🇮 FI |  |
| [libreddit.database.red](https://libreddit.database.red) | 🇺🇸 US | ✅ |
| [libreddit.exonip.de](https://libreddit.exonip.de) | 🇩🇪 DE  |  |
| [libreddit.domain.glass](https://libreddit.domain.glass) | 🇺🇸 US | ✅ |
| [spjmllawtheisznfs7uryhxumin26ssv2draj7oope3ok3wuhy43eoyd.onion](http://spjmllawtheisznfs7uryhxumin26ssv2draj7oope3ok3wuhy43eoyd.onion) | 🇮🇳 IN  |  |
| [fwhhsbrbltmrct5hshrnqlqygqvcgmnek3cnka55zj4y7nuus5muwyyd.onion](http://fwhhsbrbltmrct5hshrnqlqygqvcgmnek3cnka55zj4y7nuus5muwyyd.onion) | 🇩🇪 DE  |  |
| [dflv6yjt7il3n3tggf4qhcmkzbti2ppytqx3o7pjrzwgntutpewscyid.onion](http://dflv6yjt7il3n3tggf4qhcmkzbti2ppytqx3o7pjrzwgntutpewscyid.onion/) | 🇺🇸 US |  |
| [kphht2jcflojtqte4b4kyx7p2ahagv4debjj32nre67dxz7y57seqwyd.onion](http://kphht2jcflojtqte4b4kyx7p2ahagv4debjj32nre67dxz7y57seqwyd.onion/) | 🇳🇱 NL |  |

A checkmark in the "Cloudflare" category here refers to the use of the reverse proxy, [Cloudflare](https://cloudflare). The checkmark will not be listed for a site which uses Cloudflare DNS but rather the proxying service which grants Cloudflare the ability to monitor traffic to the website.

---

# About

Find Libreddit on 💬 [Matrix](https://matrix.to/#/#libreddit:kde.org), 🐋 [Docker](https://hub.docker.com/r/spikecodes/libreddit), :octocat: [GitHub](https://github.com/spikecodes/libreddit), and 🦊 [GitLab](https://gitlab.com/spikecodes/libreddit).

## Built with

- [Rust](https://www.rust-lang.org/) - Programming language
- [Hyper](https://github.com/hyperium/hyper) - HTTP server and client
- [Askama](https://github.com/djc/askama) - Templating engine
- [Rustls](https://github.com/ctz/rustls) - TLS library

## Info
Libreddit hopes to provide an easier way to browse Reddit, without the ads, trackers, and bloat. Libreddit was inspired by other alternative front-ends to popular services such as [Invidious](https://github.com/iv-org/invidious) for YouTube, [Nitter](https://github.com/zedeus/nitter) for Twitter, and [Bibliogram](https://sr.ht/~cadence/bibliogram/) for Instagram.

Libreddit currently implements most of Reddit's (signed-out) functionalities but still lacks [a few features](https://github.com/spikecodes/libreddit/issues).

## How does it compare to Teddit?

Teddit is another awesome open source project designed to provide an alternative frontend to Reddit. There is no connection between the two and you're welcome to use whichever one you favor. Competition fosters innovation and Teddit's release has motivated me to build Libreddit into an even more polished product.

If you are looking to compare, the biggest differences I have noticed are:
- Libreddit is themed around Reddit's redesign whereas Teddit appears to stick much closer to Reddit's old design. This may suit some users better as design is always subjective.
- Libreddit is written in [Rust](https://www.rust-lang.org) for speed and memory safety. It uses [Hyper](https://hyper.rs), a speedy and lightweight HTTP server/client implementation.

---

# Comparison

This section outlines how Libreddit compares to Reddit.

## Speed

Lasted tested Jan 17, 2021.

Results from Google Lighthouse ([Libreddit Report](https://lighthouse-dot-webdotdevsite.appspot.com/lh/html?url=https%3A%2F%2Flibredd.it), [Reddit Report](https://lighthouse-dot-webdotdevsite.appspot.com/lh/html?url=https%3A%2F%2Fwww.reddit.com%2F)).

|                        | Libreddit     | Reddit     |
|------------------------|---------------|------------|
| Requests               | 20            | 70         |
| Resource Size (card ui)| 1,224 KiB     | 1,690 KiB  |
| Time to Interactive    | **1.5 s**     | **11.2 s** |

## Privacy

### Reddit

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

### Libreddit

For transparency, I hope to describe all the ways Libreddit handles user privacy.

**Logging:** In production (when running the binary, hosting with docker, or using the official instances), Libreddit logs when Reddit is ratelimiting Libreddit and when Reddit's JSON responses can't be parsed. When debugging (running from source without `--release`), Libreddit logs post IDs and URL paths fetched to aid with troubleshooting.

**DNS:** Both official domains (`libredd.it` and `libreddit.spike.codes`) use Cloudflare as the DNS resolver. Though, the sites are not proxied through Cloudflare meaning Cloudflare doesn't have access to user traffic.

**Cookies:** Libreddit uses optional cookies to store any configured settings in [the settings menu](https://libreddit.spike.codes/settings). This is not a cross-site cookie and the cookie holds no personal data, only a value of the possible layout.

**Hosting:** The official instances are hosted on [Replit](https://replit.com/) which monitors usage to prevent abuse. I can understand if this invalidates certain users' threat models and therefore, selfhosting and browsing through Tor are welcomed.

---

# Installation

## 1) Cargo

Make sure Rust stable is installed along with `cargo`, Rust's package manager.

```
cargo install libreddit
```

## 2) Docker

Deploy the [Docker image](https://hub.docker.com/r/spikecodes/libreddit) of Libreddit:
```
docker pull spikecodes/libreddit
docker run -d --name libreddit -p 8080:8080 spikecodes/libreddit
```

Deploy using a different port (in this case, port 80):
```
docker pull spikecodes/libreddit
docker run -d --name libreddit -p 80:8080 spikecodes/libreddit
```

To deploy on `arm64` platforms, simply replace `spikecodes/libreddit` in the commands above with `spikecodes/libreddit:arm`.

## 3) AUR

For ArchLinux users, Libreddit is available from the AUR as [`libreddit-git`](https://aur.archlinux.org/packages/libreddit-git).

```
yay -S libreddit-git
```

## 4) GitHub Releases

If you're on Linux and none of these methods work for you, you can grab a Linux binary from [the newest release](https://github.com/spikecodes/libreddit/releases/latest).

## 5) Replit

**Note:** Replit is a free option but they are *not* private and will monitor server usage to prevent abuse. If you need a free and easy setup, this method may work best for you.

1. Create a Replit account (see note above)
2. Visit [the official Repl](https://replit.com/@spikethecoder/libreddit) and fork it
3. Hit the run button to download the latest Libreddit version and start it

In the web preview (defaults to top right), you should see your instance hosted where you can assign a [custom domain](https://docs.replit.com/repls/web-hosting#custom-domains).

---

# Deployment

Once installed, deploy Libreddit to `0.0.0.0:8080` by running:

```
libreddit
```

## Proxying using NGINX

**NOTE** If you're [proxying Libreddit through a NGINX Reverse Proxy](https://github.com/spikecodes/libreddit/issues/122#issuecomment-782226853), add
```nginx
proxy_http_version 1.1;
```
to your NGINX configuration file above your `proxy_pass` line.

## Building

```
git clone https://github.com/spikecodes/libreddit
cd libreddit
cargo run
```
