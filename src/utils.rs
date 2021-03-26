//
// CRATES
//
use crate::{client::json, esc, server::RequestExt};
use askama::Template;
use cookie::Cookie;
use hyper::{Body, Request, Response};
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use time::{Duration, OffsetDateTime};
use url::Url;

// Post flair with content, background color and foreground color
pub struct Flair {
	pub flair_parts: Vec<FlairPart>,
	pub text: String,
	pub background_color: String,
	pub foreground_color: String,
}

// Part of flair, either emoji or text
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
								"text" => value("t").to_string(),
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
					value: text.to_string(),
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

pub struct Media {
	pub url: String,
	pub width: i64,
	pub height: i64,
	pub poster: String,
}

impl Media {
	pub async fn parse(data: &Value) -> (String, Self, Vec<GalleryMedia>) {
		let mut gallery = Vec::new();

		// If post is a video, return the video
		let (post_type, url_val) = if data["preview"]["reddit_video_preview"]["fallback_url"].is_string() {
			// Return reddit video
			("video", &data["preview"]["reddit_video_preview"]["fallback_url"])
		} else if data["secure_media"]["reddit_video"]["fallback_url"].is_string() {
			// Return reddit video
			("video", &data["secure_media"]["reddit_video"]["fallback_url"])
		} else if data["post_hint"].as_str().unwrap_or("") == "image" {
			// Handle images, whether GIFs or pics
			let preview = &data["preview"]["images"][0];
			let mp4 = &preview["variants"]["mp4"];

			if mp4.is_object() {
				// Return the mp4 if the media is a gif
				("gif", &mp4["source"]["url"])
			} else {
				// Return the picture if the media is an image
				if data["domain"] == "i.redd.it" {
					("image", &data["url"])
				} else {
					("image", &preview["source"]["url"])
				}
			}
		} else if data["is_self"].as_bool().unwrap_or_default() {
			// If type is self, return permalink
			("self", &data["permalink"])
		} else if data["is_gallery"].as_bool().unwrap_or_default() {
			// If this post contains a gallery of images
			gallery = GalleryMedia::parse(&data["gallery_data"]["items"], &data["media_metadata"]);

			("gallery", &data["url"])
		} else {
			// If type can't be determined, return url
			("link", &data["url"])
		};

		let source = &data["preview"]["images"][0]["source"];

		let url = if post_type == "self" || post_type == "link" {
			url_val.as_str().unwrap_or_default().to_string()
		} else {
			format_url(url_val.as_str().unwrap_or_default())
		};

		(
			post_type.to_string(),
			Self {
				url,
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
	pub awards: Awards
}

impl Post {
	// Fetch posts of a user or subreddit and return a vector of posts and the "after" value
	pub async fn fetch(path: &str, fallback_title: String) -> Result<(Vec<Self>, String), String> {
		let res;
		let post_list;

		// Send a request to the url
		match json(path.to_string()).await {
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
			let (post_type, media, gallery) = Media::parse(&data).await;
			let mut awards = Awards::new();
			
			awards.parse(&data["all_awardings"]).await;

			posts.push(Self {
				id: val(post, "id"),
				title: esc!(if title.is_empty() { fallback_title.to_owned() } else { title }),
				community: val(post, "subreddit"),
				body: rewrite_urls(&val(post, "body_html")),
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
				awards
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
}


#[derive(Debug)]
pub struct Award {
	pub name: String,
	pub icon_url: String,
	pub description: String,
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
		self.iter().fold(Ok(()), |result, award| {
			result.and_then(|_| writeln!(f, "{}", award))
		})
	}
}

impl Awards {
	pub fn new() -> Self {
		let awards: Vec<Award> = Vec::new();
		Self(awards)
	}

	pub async fn parse(&mut self, items: &Value) -> &mut Self {

		if let Some(array_items) = items.as_array() {
			for item in array_items.iter() {
				let name = item["name"].as_str().unwrap_or_default().to_string();
				let icon_url = item["icon_url"].as_str().unwrap_or_default().to_string();
				let description = item["description"].as_str().unwrap_or_default().to_string();

				self.0.push(
					Award {
						name,
						icon_url,
						description,
					}
				)
			}
			
			self
		} else {
			self
		}
	}
}

#[derive(Template)]
#[template(path = "error.html", escape = "none")]
pub struct ErrorTemplate {
	pub msg: String,
	pub prefs: Preferences,
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
	pub comment_sort: String,
	pub subscriptions: Vec<String>,
}

impl Preferences {
	// Build preferences from cookies
	pub fn new(req: Request<Body>) -> Self {
		Self {
			theme: cookie(&req, "theme"),
			front_page: cookie(&req, "front_page"),
			layout: cookie(&req, "layout"),
			wide: cookie(&req, "wide"),
			show_nsfw: cookie(&req, "show_nsfw"),
			comment_sort: cookie(&req, "comment_sort"),
			subscriptions: cookie(&req, "subscriptions").split('+').map(String::from).filter(|s| !s.is_empty()).collect(),
		}
	}
}

//
// FORMATTING
//

// Grab a query parameter from a url
pub fn param(path: &str, value: &str) -> String {
	match Url::parse(format!("https://libredd.it/{}", path).as_str()) {
		Ok(url) => url.query_pairs().into_owned().collect::<HashMap<_, _>>().get(value).unwrap_or(&String::new()).to_owned(),
		_ => String::new(),
	}
}

// Parse a cookie value from request
pub fn cookie(req: &Request<Body>, name: &str) -> String {
	let cookie = req.cookie(name).unwrap_or_else(|| Cookie::named(name));
	cookie.value().to_string()
}

// Direct urls to proxy if proxy is enabled
pub fn format_url(url: &str) -> String {
	if url.is_empty() || url == "self" || url == "default" || url == "nsfw" || url == "spoiler" {
		String::new()
	} else {
		match Url::parse(url) {
			Ok(parsed) => {
				let domain = parsed.domain().unwrap_or_default();

				let capture = |regex: &str, format: &str, segments: i16| {
					Regex::new(regex)
						.map(|re| match re.captures(url) {
							Some(caps) => match segments {
								1 => [format, &caps[1]].join(""),
								2 => [format, &caps[1], "/", &caps[2]].join(""),
								_ => String::new(),
							},
							None => String::new(),
						})
						.unwrap_or_default()
				};

				match domain {
					"v.redd.it" => capture(r"https://v\.redd\.it/(.*)/DASH_([0-9]{2,4}(\.mp4|$))", "/vid/", 2),
					"i.redd.it" => capture(r"https://i\.redd\.it/(.*)", "/img/", 1),
					"a.thumbs.redditmedia.com" => capture(r"https://a\.thumbs\.redditmedia\.com/(.*)", "/thumb/a/", 1),
					"b.thumbs.redditmedia.com" => capture(r"https://b\.thumbs\.redditmedia\.com/(.*)", "/thumb/b/", 1),
					"emoji.redditmedia.com" => capture(r"https://emoji\.redditmedia\.com/(.*)/(.*)", "/emoji/", 2),
					"preview.redd.it" => capture(r"https://preview\.redd\.it/(.*)\?(.*)", "/preview/pre/", 2),
					"external-preview.redd.it" => capture(r"https://external\-preview\.redd\.it/(.*)\?(.*)", "/preview/external-pre/", 2),
					"styles.redditmedia.com" => capture(r"https://styles\.redditmedia\.com/(.*)", "/style/", 1),
					"www.redditstatic.com" => capture(r"https://www\.redditstatic\.com/(.*)", "/static/", 1),
					_ => String::new(),
				}
			}
			Err(_) => String::new(),
		}
	}
}

// Rewrite Reddit links to Libreddit in body of text
pub fn rewrite_urls(text: &str) -> String {
	match Regex::new(r#"href="(https|http|)://(www.|old.|np.|amp.|)(reddit).(com)/"#) {
		Ok(re) => re.replace_all(text, r#"href="/"#).to_string(),
		Err(_) => String::new(),
	}
}

// Append `m` and `k` for millions and thousands respectively
pub fn format_num(num: i64) -> (String, String) {
	let truncated = if num >= 1_000_000 || num <= -1_000_000 {
		format!("{}m", num / 1_000_000)
	} else if num >= 1000 || num <= -1000 {
		format!("{}k", num / 1_000)
	} else {
		num.to_string()
	};

	(truncated, num.to_string())
}

// Parse a relative and absolute time from a UNIX timestamp
pub fn time(created: f64) -> (String, String) {
	let time = OffsetDateTime::from_unix_timestamp(created.round() as i64);
	let time_delta = OffsetDateTime::now_utc() - time;

	// If the time difference is more than a month, show full date
	let rel_time = if time_delta > Duration::days(30) {
		time.format("%b %d '%y")
	// Otherwise, show relative date/time
	} else if time_delta.whole_days() > 0 {
		format!("{}d ago", time_delta.whole_days())
	} else if time_delta.whole_hours() > 0 {
		format!("{}h ago", time_delta.whole_hours())
	} else {
		format!("{}m ago", time_delta.whole_minutes())
	};

	(rel_time, time.format("%b %d %Y, %H:%M:%S UTC"))
}

// val() function used to parse JSON from Reddit APIs
pub fn val(j: &Value, k: &str) -> String {
	j["data"][k].as_str().unwrap_or_default().to_string()
}

#[macro_export]
macro_rules! esc {
	($f:expr) => {
		$f.replace('<', "&lt;").replace('>', "&gt;")
	};
	($j:expr, $k:expr) => {
		$j["data"][$k].as_str().unwrap_or_default().to_string().replace('<', "&lt;").replace('>', "&gt;")
	};
}

// Escape < and > to accurately render HTML
// pub fn esc(j: &Value, k: &str) -> String {
// 	val(j,k)
// 		// .replace('&', "&amp;")
// 		.replace('<', "&lt;")
// 		.replace('>', "&gt;")
// 		// .replace('"', "&quot;")
// 		// .replace('\'', "&#x27;")
// 		// .replace('/', "&#x2f;")
// }

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
	let body = ErrorTemplate {
		msg,
		prefs: Preferences::new(req),
	}
	.render()
	.unwrap_or_default();

	Ok(Response::builder().status(404).header("content-type", "text/html").body(body.into()).unwrap_or_default())
}
