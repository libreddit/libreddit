//
// CRATES
//
use crate::{client::json, esc, server::RequestExt};
use askama::Template;
use cookie::Cookie;
use hyper::{Body, Request, Response};
use regex::Regex;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use time::{Duration, OffsetDateTime, macros::format_description};
use url::Url;

// Post flair with content, background color and foreground color
pub struct Flair {
	pub flair_parts: Vec<FlairPart>,
	pub text: String,
	pub background_color: String,
	pub foreground_color: String,
}

// Part of flair, either emoji or text
#[derive(Clone)]
pub struct FlairPart {
	pub flair_part_type: String,
	pub value: String,
}

impl FlairPart {
	pub fn parse(flair_type: &str, rich_flair: Option<&Vec<Value>>, text_flair: Option<&str>) -> Vec<Self> {
		// Parse type of flair
		match flair_type {
			// If flair contains emojis and text
			"richtext" => match rich_flair {
				Some(rich) => rich
					.iter()
					// For each part of the flair, extract text and emojis
					.map(|part| {
						let value = |name: &str| part[name].as_str().unwrap_or_default();
						Self {
							flair_part_type: value("e").to_string(),
							value: match value("e") {
								"text" => esc!(value("t")),
								"emoji" => format_url(value("u")),
								_ => String::new(),
							},
						}
					})
					.collect::<Vec<Self>>(),
				None => Vec::new(),
			},
			// If flair contains only text
			"text" => match text_flair {
				Some(text) => vec![Self {
					flair_part_type: "text".to_string(),
					value: esc!(text),
				}],
				None => Vec::new(),
			},
			_ => Vec::new(),
		}
	}
}

pub struct Author {
	pub name: String,
	pub flair: Flair,
	pub distinguished: String,
}

// Post flags with nsfw and stickied
pub struct Flags {
	pub nsfw: bool,
	pub stickied: bool,
}

#[derive(Debug)]
pub struct Media {
	pub url: String,
	pub alt_url: String,
	pub width: i64,
	pub height: i64,
	pub poster: String,
}

impl Media {
	pub async fn parse(data: &Value) -> (String, Self, Vec<GalleryMedia>) {
		let mut gallery = Vec::new();

		// Define the various known places that Reddit might put video URLs.
		let data_preview = &data["preview"]["reddit_video_preview"];
		let secure_media = &data["secure_media"]["reddit_video"];
		let crosspost_parent_media = &data["crosspost_parent_list"][0]["secure_media"]["reddit_video"];

		// If post is a video, return the video
		let (post_type, url_val, alt_url_val) = if data_preview["fallback_url"].is_string() {
			(
				if data_preview["is_gif"].as_bool().unwrap_or(false) { "gif" } else { "video" },
				&data_preview["fallback_url"],
				Some(&data_preview["hls_url"]),
			)
		} else if secure_media["fallback_url"].is_string() {
			(
				if secure_media["is_gif"].as_bool().unwrap_or(false) { "gif" } else { "video" },
				&secure_media["fallback_url"],
				Some(&secure_media["hls_url"]),
			)
		} else if crosspost_parent_media["fallback_url"].is_string() {
			(
				if crosspost_parent_media["is_gif"].as_bool().unwrap_or(false) { "gif" } else { "video" },
				&crosspost_parent_media["fallback_url"],
				Some(&crosspost_parent_media["hls_url"]),
			)
		} else if data["post_hint"].as_str().unwrap_or("") == "image" {
			// Handle images, whether GIFs or pics
			let preview = &data["preview"]["images"][0];
			let mp4 = &preview["variants"]["mp4"];

			if mp4.is_object() {
				// Return the mp4 if the media is a gif
				("gif", &mp4["source"]["url"], None)
			} else {
				// Return the picture if the media is an image
				if data["domain"] == "i.redd.it" {
					("image", &data["url"], None)
				} else {
					("image", &preview["source"]["url"], None)
				}
			}
		} else if data["is_self"].as_bool().unwrap_or_default() {
			// If type is self, return permalink
			("self", &data["permalink"], None)
		} else if data["is_gallery"].as_bool().unwrap_or_default() {
			// If this post contains a gallery of images
			gallery = GalleryMedia::parse(&data["gallery_data"]["items"], &data["media_metadata"]);

			("gallery", &data["url"], None)
		} else {
			// If type can't be determined, return url
			("link", &data["url"], None)
		};

		let source = &data["preview"]["images"][0]["source"];

		let alt_url = alt_url_val.map_or(String::new(), |val| format_url(val.as_str().unwrap_or_default()));

		(
			post_type.to_string(),
			Self {
				url: format_url(url_val.as_str().unwrap_or_default()),
				alt_url,
				width: source["width"].as_i64().unwrap_or_default(),
				height: source["height"].as_i64().unwrap_or_default(),
				poster: format_url(source["url"].as_str().unwrap_or_default()),
			},
			gallery,
		)
	}
}

pub struct GalleryMedia {
	pub url: String,
	pub width: i64,
	pub height: i64,
	pub caption: String,
	pub outbound_url: String,
}

impl GalleryMedia {
	fn parse(items: &Value, metadata: &Value) -> Vec<Self> {
		items
			.as_array()
			.unwrap_or(&Vec::new())
			.iter()
			.map(|item| {
				// For each image in gallery
				let media_id = item["media_id"].as_str().unwrap_or_default();
				let image = &metadata[media_id]["s"];

				// Construct gallery items
				Self {
					url: format_url(image["u"].as_str().unwrap_or_default()),
					width: image["x"].as_i64().unwrap_or_default(),
					height: image["y"].as_i64().unwrap_or_default(),
					caption: item["caption"].as_str().unwrap_or_default().to_string(),
					outbound_url: item["outbound_url"].as_str().unwrap_or_default().to_string(),
				}
			})
			.collect::<Vec<Self>>()
	}
}

// Post containing content, metadata and media
pub struct Post {
	pub id: String,
	pub title: String,
	pub community: String,
	pub body: String,
	pub author: Author,
	pub permalink: String,
	pub score: (String, String),
	pub upvote_ratio: i64,
	pub post_type: String,
	pub flair: Flair,
	pub flags: Flags,
	pub thumbnail: Media,
	pub media: Media,
	pub domain: String,
	pub rel_time: String,
	pub created: String,
	pub comments: (String, String),
	pub gallery: Vec<GalleryMedia>,
	pub awards: Awards,
}

impl Post {
	// Fetch posts of a user or subreddit and return a vector of posts and the "after" value
	pub async fn fetch(path: &str, quarantine: bool) -> Result<(Vec<Self>, String), String> {
		let res;
		let post_list;

		// Send a request to the url
		match json(path.to_string(), quarantine).await {
			// If success, receive JSON in response
			Ok(response) => {
				res = response;
			}
			// If the Reddit API returns an error, exit this function
			Err(msg) => return Err(msg),
		}

		// Fetch the list of posts from the JSON response
		match res["data"]["children"].as_array() {
			Some(list) => post_list = list,
			None => return Err("No posts found".to_string()),
		}

		let mut posts: Vec<Self> = Vec::new();

		// For each post from posts list
		for post in post_list {
			let data = &post["data"];

			let (rel_time, created) = time(data["created_utc"].as_f64().unwrap_or_default());
			let score = data["score"].as_i64().unwrap_or_default();
			let ratio: f64 = data["upvote_ratio"].as_f64().unwrap_or(1.0) * 100.0;
			let title = esc!(post, "title");

			// Determine the type of media along with the media URL
			let (post_type, media, gallery) = Media::parse(data).await;
			let awards = Awards::parse(&data["all_awardings"]);

			// selftext_html is set for text posts when browsing.
			let mut body = rewrite_urls(&val(post, "selftext_html"));
			if body.is_empty() {
				body = rewrite_urls(&val(post, "body_html"));
			}

			posts.push(Self {
				id: val(post, "id"),
				title,
				community: val(post, "subreddit"),
				body,
				author: Author {
					name: val(post, "author"),
					flair: Flair {
						flair_parts: FlairPart::parse(
							data["author_flair_type"].as_str().unwrap_or_default(),
							data["author_flair_richtext"].as_array(),
							data["author_flair_text"].as_str(),
						),
						text: esc!(post, "link_flair_text"),
						background_color: val(post, "author_flair_background_color"),
						foreground_color: val(post, "author_flair_text_color"),
					},
					distinguished: val(post, "distinguished"),
				},
				score: if data["hide_score"].as_bool().unwrap_or_default() {
					("\u{2022}".to_string(), "Hidden".to_string())
				} else {
					format_num(score)
				},
				upvote_ratio: ratio as i64,
				post_type,
				thumbnail: Media {
					url: format_url(val(post, "thumbnail").as_str()),
					alt_url: String::new(),
					width: data["thumbnail_width"].as_i64().unwrap_or_default(),
					height: data["thumbnail_height"].as_i64().unwrap_or_default(),
					poster: "".to_string(),
				},
				media,
				domain: val(post, "domain"),
				flair: Flair {
					flair_parts: FlairPart::parse(
						data["link_flair_type"].as_str().unwrap_or_default(),
						data["link_flair_richtext"].as_array(),
						data["link_flair_text"].as_str(),
					),
					text: esc!(post, "link_flair_text"),
					background_color: val(post, "link_flair_background_color"),
					foreground_color: if val(post, "link_flair_text_color") == "dark" {
						"black".to_string()
					} else {
						"white".to_string()
					},
				},
				flags: Flags {
					nsfw: data["over_18"].as_bool().unwrap_or_default(),
					stickied: data["stickied"].as_bool().unwrap_or_default(),
				},
				permalink: val(post, "permalink"),
				rel_time,
				created,
				comments: format_num(data["num_comments"].as_i64().unwrap_or_default()),
				gallery,
				awards,
			});
		}

		Ok((posts, res["data"]["after"].as_str().unwrap_or_default().to_string()))
	}
}

#[derive(Template)]
#[template(path = "comment.html", escape = "none")]
// Comment with content, post, score and data/time that it was posted
pub struct Comment {
	pub id: String,
	pub kind: String,
	pub parent_id: String,
	pub parent_kind: String,
	pub post_link: String,
	pub post_author: String,
	pub body: String,
	pub author: Author,
	pub score: (String, String),
	pub rel_time: String,
	pub created: String,
	pub edited: (String, String),
	pub replies: Vec<Comment>,
	pub highlighted: bool,
	pub awards: Awards,
	pub collapsed: bool,
	pub is_filtered: bool,
}

#[derive(Default, Clone)]
pub struct Award {
	pub name: String,
	pub icon_url: String,
	pub description: String,
	pub count: i64,
}

impl std::fmt::Display for Award {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{} {} {}", self.name, self.icon_url, self.description)
	}
}

pub struct Awards(pub Vec<Award>);

impl std::ops::Deref for Awards {
	type Target = Vec<Award>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl std::fmt::Display for Awards {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		self.iter().fold(Ok(()), |result, award| result.and_then(|_| writeln!(f, "{}", award)))
	}
}

// Convert Reddit awards JSON to Awards struct
impl Awards {
	pub fn parse(items: &Value) -> Self {
		let parsed = items.as_array().unwrap_or(&Vec::new()).iter().fold(Vec::new(), |mut awards, item| {
			let name = item["name"].as_str().unwrap_or_default().to_string();
			let icon_url = format_url(item["resized_icons"][0]["url"].as_str().unwrap_or_default());
			let description = item["description"].as_str().unwrap_or_default().to_string();
			let count: i64 = i64::from_str(&item["count"].to_string()).unwrap_or(1);

			awards.push(Award {
				name,
				icon_url,
				description,
				count,
			});

			awards
		});

		Self(parsed)
	}
}

#[derive(Template)]
#[template(path = "error.html", escape = "none")]
pub struct ErrorTemplate {
	pub msg: String,
	pub prefs: Preferences,
	pub url: String,
}

#[derive(Default)]
// User struct containing metadata about user
pub struct User {
	pub name: String,
	pub title: String,
	pub icon: String,
	pub karma: i64,
	pub created: String,
	pub banner: String,
	pub description: String,
}

#[derive(Default)]
// Subreddit struct containing metadata about community
pub struct Subreddit {
	pub name: String,
	pub title: String,
	pub description: String,
	pub info: String,
	// pub moderators: Vec<String>,
	pub icon: String,
	pub members: (String, String),
	pub active: (String, String),
	pub wiki: bool,
}

// Parser for query params, used in sorting (eg. /r/rust/?sort=hot)
#[derive(serde::Deserialize)]
pub struct Params {
	pub t: Option<String>,
	pub q: Option<String>,
	pub sort: Option<String>,
	pub after: Option<String>,
	pub before: Option<String>,
}

#[derive(Default)]
pub struct Preferences {
	pub theme: String,
	pub front_page: String,
	pub layout: String,
	pub wide: String,
	pub show_nsfw: String,
	pub hide_hls_notification: String,
	pub use_hls: String,
	pub autoplay_videos: String,
	pub comment_sort: String,
	pub post_sort: String,
	pub subscriptions: Vec<String>,
	pub filters: Vec<String>,
}

impl Preferences {
	// Build preferences from cookies
	pub fn new(req: Request<Body>) -> Self {
		Self {
			theme: setting(&req, "theme"),
			front_page: setting(&req, "front_page"),
			layout: setting(&req, "layout"),
			wide: setting(&req, "wide"),
			show_nsfw: setting(&req, "show_nsfw"),
			use_hls: setting(&req, "use_hls"),
			hide_hls_notification: setting(&req, "hide_hls_notification"),
			autoplay_videos: setting(&req, "autoplay_videos"),
			comment_sort: setting(&req, "comment_sort"),
			post_sort: setting(&req, "post_sort"),
			subscriptions: setting(&req, "subscriptions").split('+').map(String::from).filter(|s| !s.is_empty()).collect(),
			filters: setting(&req, "filters").split('+').map(String::from).filter(|s| !s.is_empty()).collect(),
		}
	}
}

/// Gets a `HashSet` of filters from the cookie in the given `Request`.
pub fn get_filters(req: &Request<Body>) -> HashSet<String> {
	setting(req, "filters").split('+').map(String::from).filter(|s| !s.is_empty()).collect::<HashSet<String>>()
}

/// Filters a `Vec<Post>` by the given `HashSet` of filters (each filter being a subreddit name or a user name). If a
/// `Post`'s subreddit or author is found in the filters, it is removed. Returns `true` if _all_ posts were filtered
/// out, or `false` otherwise.
pub fn filter_posts(posts: &mut Vec<Post>, filters: &HashSet<String>) -> bool {
	if posts.is_empty() {
		false
	} else {
		posts.retain(|p| !filters.contains(&p.community) && !filters.contains(&["u_", &p.author.name].concat()));
		posts.is_empty()
	}
}

//
// FORMATTING
//

// Grab a query parameter from a url
pub fn param(path: &str, value: &str) -> Option<String> {
	Some(
		Url::parse(format!("https://libredd.it/{}", path).as_str())
			.ok()?
			.query_pairs()
			.into_owned()
			.collect::<HashMap<_, _>>()
			.get(value)?
			.clone(),
	)
}

// Retrieve the value of a setting by name
pub fn setting(req: &Request<Body>, name: &str) -> String {
	// Parse a cookie value from request
	req
		.cookie(name)
		.unwrap_or_else(|| {
			// If there is no cookie for this setting, try receiving a default from an environment variable
			if let Ok(default) = std::env::var(format!("LIBREDDIT_DEFAULT_{}", name.to_uppercase())) {
				Cookie::new(name, default)
			} else {
				Cookie::named(name)
			}
		})
		.value()
		.to_string()
}

// Detect and redirect in the event of a random subreddit
pub async fn catch_random(sub: &str, additional: &str) -> Result<Response<Body>, String> {
	if sub == "random" || sub == "randnsfw" {
		let new_sub = json(format!("/r/{}/about.json?raw_json=1", sub), false).await?["data"]["display_name"]
			.as_str()
			.unwrap_or_default()
			.to_string();
		Ok(redirect(format!("/r/{}{}", new_sub, additional)))
	} else {
		Err("No redirect needed".to_string())
	}
}

// Direct urls to proxy if proxy is enabled
pub fn format_url(url: &str) -> String {
	if url.is_empty() || url == "self" || url == "default" || url == "nsfw" || url == "spoiler" {
		String::new()
	} else {
		Url::parse(url).map_or(url.to_string(), |parsed| {
			let domain = parsed.domain().unwrap_or_default();

			let capture = |regex: &str, format: &str, segments: i16| {
				Regex::new(regex).map_or(String::new(), |re| {
					re.captures(url).map_or(String::new(), |caps| match segments {
						1 => [format, &caps[1]].join(""),
						2 => [format, &caps[1], "/", &caps[2]].join(""),
						_ => String::new(),
					})
				})
			};

			macro_rules! chain {
				() => {
					{
						String::new()
					}
				};

				( $first_fn:expr, $($other_fns:expr), *) => {
					{
						let result = $first_fn;
						if result.is_empty() {
							chain!($($other_fns,)*)
						}
						else
						{
							result
						}
					}
				};
			}

			match domain {
				"www.reddit.com" => capture(r"https://www\.reddit\.com/(.*)", "/", 1),
				"old.reddit.com" => capture(r"https://old\.reddit\.com/(.*)", "/", 1),
				"np.reddit.com" => capture(r"https://np\.reddit\.com/(.*)", "/", 1),
				"reddit.com" => capture(r"https://reddit\.com/(.*)", "/", 1),
				"v.redd.it" => chain!(
					capture(r"https://v\.redd\.it/(.*)/DASH_([0-9]{2,4}(\.mp4|$|\?source=fallback))", "/vid/", 2),
					capture(r"https://v\.redd\.it/(.+)/(HLSPlaylist\.m3u8.*)$", "/hls/", 2)
				),
				"i.redd.it" => capture(r"https://i\.redd\.it/(.*)", "/img/", 1),
				"a.thumbs.redditmedia.com" => capture(r"https://a\.thumbs\.redditmedia\.com/(.*)", "/thumb/a/", 1),
				"b.thumbs.redditmedia.com" => capture(r"https://b\.thumbs\.redditmedia\.com/(.*)", "/thumb/b/", 1),
				"emoji.redditmedia.com" => capture(r"https://emoji\.redditmedia\.com/(.*)/(.*)", "/emoji/", 2),
				"preview.redd.it" => capture(r"https://preview\.redd\.it/(.*)", "/preview/pre/", 1),
				"external-preview.redd.it" => capture(r"https://external\-preview\.redd\.it/(.*)", "/preview/external-pre/", 1),
				"styles.redditmedia.com" => capture(r"https://styles\.redditmedia\.com/(.*)", "/style/", 1),
				"www.redditstatic.com" => capture(r"https://www\.redditstatic\.com/(.*)", "/static/", 1),
				_ => url.to_string(),
			}
		})
	}
}

// Rewrite Reddit links to Libreddit in body of text
pub fn rewrite_urls(input_text: &str) -> String {

	let text1 =
		Regex::new(r#"href="(https|http|)://(www\.|old\.|np\.|amp\.|)(reddit\.com|redd\.it)/"#)
			.map_or(String::new(), |re| re.replace_all(input_text, r#"href="/"#).to_string())
			// Remove (html-encoded) "\" from URLs.
			.replace("%5C", "").replace(r"\", "");

	// Rewrite external media previews to Libreddit
	Regex::new(r"https://external-preview\.redd\.it(.*)[^?]").map_or(String::new(), |re| {
		if re.is_match(&text1) {
			re.replace_all(&text1, format_url(re.find(&text1).map(|x| x.as_str()).unwrap_or_default())).to_string()
		} else {
			text1
		}
	})
}

// Format vote count to a string that will be displayed.
// Append `m` and `k` for millions and thousands respectively, and
// round to the nearest tenth.
pub fn format_num(num: i64) -> (String, String) {
	let truncated = if num >= 1_000_000 || num <= -1_000_000 {
		format!("{:.1}m", num as f64 / 1_000_000.0)
	} else if num >= 1000 || num <= -1000 {
		format!("{:.1}k", num as f64 / 1_000.0)
	} else {
		num.to_string()
	};

	(truncated, num.to_string())
}

// Parse a relative and absolute time from a UNIX timestamp
pub fn time(created: f64) -> (String, String) {
	let time = OffsetDateTime::from_unix_timestamp(created.round() as i64).unwrap_or(OffsetDateTime::UNIX_EPOCH);
	let time_delta = OffsetDateTime::now_utc() - time;

	// If the time difference is more than a month, show full date
	let rel_time = if time_delta > Duration::days(30) {
		time.format(format_description!("[month repr:short] [day] '[year repr:last_two]")).unwrap_or_default()
	// Otherwise, show relative date/time
	} else if time_delta.whole_days() > 0 {
		format!("{}d ago", time_delta.whole_days())
	} else if time_delta.whole_hours() > 0 {
		format!("{}h ago", time_delta.whole_hours())
	} else {
		format!("{}m ago", time_delta.whole_minutes())
	};

	(rel_time, time.format(format_description!("[month repr:short] [day] [year], [hour]:[minute]:[second] UTC")).unwrap_or_default())
}

// val() function used to parse JSON from Reddit APIs
pub fn val(j: &Value, k: &str) -> String {
	j["data"][k].as_str().unwrap_or_default().to_string()
}

// Escape < and > to accurately render HTML
#[macro_export]
macro_rules! esc {
	($f:expr) => {
		$f.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
	};
	($j:expr, $k:expr) => {
		$j["data"][$k].as_str().unwrap_or_default().to_string().replace('<', "&lt;").replace('>', "&gt;")
	};
}

//
// NETWORKING
//

pub fn template(t: impl Template) -> Result<Response<Body>, String> {
	Ok(
		Response::builder()
			.status(200)
			.header("content-type", "text/html")
			.body(t.render().unwrap_or_default().into())
			.unwrap_or_default(),
	)
}

pub fn redirect(path: String) -> Response<Body> {
	Response::builder()
		.status(302)
		.header("content-type", "text/html")
		.header("Location", &path)
		.body(format!("Redirecting to <a href=\"{0}\">{0}</a>...", path).into())
		.unwrap_or_default()
}

pub async fn error(req: Request<Body>, msg: String) -> Result<Response<Body>, String> {
	let url = req.uri().to_string();
	let body = ErrorTemplate {
		msg,
		prefs: Preferences::new(req),
		url,
	}
	.render()
	.unwrap_or_default();

	Ok(Response::builder().status(404).header("content-type", "text/html").body(body.into()).unwrap_or_default())
}

pub fn get_saved_posts(req: &Request<Body>) -> Vec<String> {
    match req.cookie("saved_posts") {
        Some(cookie) => cookie.value().split('+').map(String::from).collect(),
        None => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
	use super::format_num;
	use super::rewrite_urls;

	#[test]
	fn format_num_works() {
		assert_eq!(format_num(567), ("567".to_string(), "567".to_string()));
		assert_eq!(format_num(1234), ("1.2k".to_string(), "1234".to_string()));
		assert_eq!(format_num(1999), ("2.0k".to_string(), "1999".to_string()));
		assert_eq!(format_num(1001), ("1.0k".to_string(), "1001".to_string()));
		assert_eq!(format_num(1_999_999), ("2.0m".to_string(), "1999999".to_string()));
	}

	#[test]
	fn rewrite_urls_removes_backslashes() {
		let comment_body_html = r#"<a href=\"https://www.reddit.com/r/linux%5C_gaming/comments/x/just%5C_a%5C_test%5C/\">https://www.reddit.com/r/linux\\_gaming/comments/x/just\\_a\\_test/</a>"#;
		assert_eq!(
			rewrite_urls(comment_body_html),
			r#"<a href="https://www.reddit.com/r/linux_gaming/comments/x/just_a_test/">https://www.reddit.com/r/linux_gaming/comments/x/just_a_test/</a>"#
		)
	}
}
