//
// CRATES
//
use actix_web::{cookie::Cookie, HttpRequest, HttpResponse, Result};
use askama::Template;
use base64::encode;
use regex::Regex;
use serde_json::{from_str, Value};
use std::collections::HashMap;
use time::{Duration, OffsetDateTime};
use url::Url;

//
// STRUCTS
//
// Post flair with content, background color and foreground color
pub struct Flair {
	pub flair_parts: Vec<FlairPart>,
	pub background_color: String,
	pub foreground_color: String,
}

pub struct FlairPart {
	pub flair_part_type: String,
	pub value: String,
}

// Post flags with nsfw and stickied
pub struct Flags {
	pub nsfw: bool,
	pub stickied: bool,
}

// Post containing content, metadata and media
pub struct Post {
	pub id: String,
	pub title: String,
	pub community: String,
	pub body: String,
	pub author: String,
	pub author_flair: Flair,
	pub permalink: String,
	pub score: String,
	pub upvote_ratio: i64,
	pub post_type: String,
	pub flair: Flair,
	pub flags: Flags,
	pub thumbnail: String,
	pub media: String,
	pub domain: String,
	pub time: String,
}

// Comment with content, post, score and data/time that it was posted
pub struct Comment {
	pub id: String,
	pub body: String,
	pub author: String,
	pub flair: Flair,
	pub score: String,
	pub time: String,
	pub replies: Vec<Comment>,
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
	pub members: String,
	pub active: String,
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

// Error template
#[derive(Template)]
#[template(path = "error.html", escape = "none")]
pub struct ErrorTemplate {
	pub message: String,
	pub prefs: Preferences,
}

#[derive(Default)]
pub struct Preferences {
	pub theme: String,
	pub front_page: String,
	pub layout: String,
	pub wide: String,
	pub hide_nsfw: String,
	pub comment_sort: String,
}

//
// FORMATTING
//

// Build preferences from cookies
pub fn prefs(req: HttpRequest) -> Preferences {
	Preferences {
		theme: cookie(&req, "theme"),
		front_page: cookie(&req, "front_page"),
		layout: cookie(&req, "layout"),
		wide: cookie(&req, "wide"),
		hide_nsfw: cookie(&req, "hide_nsfw"),
		comment_sort: cookie(&req, "comment_sort"),
	}
}

// Grab a query param from a url
pub fn param(path: &str, value: &str) -> String {
	let url = Url::parse(format!("https://libredd.it/{}", path).as_str()).unwrap();
	let pairs: HashMap<_, _> = url.query_pairs().into_owned().collect();
	pairs.get(value).unwrap_or(&String::new()).to_owned()
}

// Parse Cookie value from request
pub fn cookie(req: &HttpRequest, name: &str) -> String {
	actix_web::HttpMessage::cookie(req, name).unwrap_or_else(|| Cookie::new(name, "")).value().to_string()
}

// Direct urls to proxy if proxy is enabled
pub fn format_url(url: &str) -> String {
	if url.is_empty() || url == "self" || url == "default" || url == "nsfw" || url == "spoiler" {
		String::new()
	} else {
		format!("/proxy/{}", encode(url).as_str())
	}
}

// Rewrite Reddit links to Libreddit in body of text
pub fn rewrite_url(text: &str) -> String {
	let re = Regex::new(r#"href="(https://|http://|)(www.|)(reddit).(com)/"#).unwrap();
	re.replace_all(text, r#"href="/"#).to_string()
}

// Append `m` and `k` for millions and thousands respectively
pub fn format_num(num: i64) -> String {
	if num > 1_000_000 {
		format!("{}m", num / 1_000_000)
	} else if num > 1000 {
		format!("{}k", num / 1_000)
	} else {
		num.to_string()
	}
}

pub async fn media(data: &serde_json::Value) -> (String, String) {
	let post_type: &str;
	let url = if !data["preview"]["reddit_video_preview"]["fallback_url"].is_null() {
		post_type = "video";
		format_url(data["preview"]["reddit_video_preview"]["fallback_url"].as_str().unwrap_or_default())
	} else if !data["secure_media"]["reddit_video"]["fallback_url"].is_null() {
		post_type = "video";
		format_url(data["secure_media"]["reddit_video"]["fallback_url"].as_str().unwrap_or_default())
	} else if data["post_hint"].as_str().unwrap_or("") == "image" {
		let preview = data["preview"]["images"][0].clone();
		match preview["variants"]["mp4"].as_object() {
			Some(gif) => {
				post_type = "gif";
				format_url(gif["source"]["url"].as_str().unwrap_or_default())
			}
			None => {
				post_type = "image";
				format_url(preview["source"]["url"].as_str().unwrap_or_default())
			}
		}
	} else if data["is_self"].as_bool().unwrap_or_default() {
		post_type = "self";
		data["permalink"].as_str().unwrap_or_default().to_string()
	} else {
		post_type = "link";
		data["url"].as_str().unwrap_or_default().to_string()
	};

	(post_type.to_string(), url)
}

pub fn parse_rich_flair(flair_type: String, rich_flair: Option<&Vec<Value>>, text_flair: Option<&str>) -> Vec<FlairPart> {
	match flair_type.as_str() {
		"richtext" => match rich_flair {
			Some(rich) => rich
				.iter()
				.map(|part| {
					let value = |name: &str| part[name].as_str().unwrap_or_default();
					FlairPart {
						flair_part_type: value("e").to_string(),
						value: match value("e") {
							"text" => value("t").to_string(),
							"emoji" => format_url(value("u")),
							_ => String::new(),
						},
					}
				})
				.collect::<Vec<FlairPart>>(),
			None => Vec::new(),
		},
		"text" => match text_flair {
			Some(text) => vec![FlairPart {
				flair_part_type: "text".to_string(),
				value: text.to_string(),
			}],
			None => Vec::new(),
		},
		_ => Vec::new(),
	}
}

pub fn time(unix_time: i64) -> String {
	let time = OffsetDateTime::from_unix_timestamp(unix_time);
	let time_delta = OffsetDateTime::now_utc() - time;
	if time_delta > Duration::days(30) {
		time.format("%b %d '%y") // %b %e '%y
	} else if time_delta.whole_days() > 0 {
		format!("{}d ago", time_delta.whole_days())
	} else if time_delta.whole_hours() > 0 {
		format!("{}h ago", time_delta.whole_hours())
	} else {
		format!("{}m ago", time_delta.whole_minutes())
	}
}

//
// JSON PARSING
//

// val() function used to parse JSON from Reddit APIs
pub fn val(j: &serde_json::Value, k: &str) -> String {
	String::from(j["data"][k].as_str().unwrap_or_default())
}

// Fetch posts of a user or subreddit and return a vector of posts and the "after" value
pub async fn fetch_posts(path: &str, fallback_title: String) -> Result<(Vec<Post>, String), &'static str> {
	let res;
	let post_list;

	// Send a request to the url
	match request(&path).await {
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
		None => return Err("No posts found"),
	}

	let mut posts: Vec<Post> = Vec::new();

	// For each post from posts list
	for post in post_list {
		let unix_time: i64 = post["data"]["created_utc"].as_f64().unwrap_or_default().round() as i64;
		let score = post["data"]["score"].as_i64().unwrap_or_default();
		let ratio: f64 = post["data"]["upvote_ratio"].as_f64().unwrap_or(1.0) * 100.0;
		let title = val(post, "title");

		// Determine the type of media along with the media URL
		let (post_type, media) = media(&post["data"]).await;

		posts.push(Post {
			id: val(post, "id"),
			title: if title.is_empty() { fallback_title.to_owned() } else { title },
			community: val(post, "subreddit"),
			body: rewrite_url(&val(post, "body_html")),
			author: val(post, "author"),
			author_flair: Flair {
				flair_parts: parse_rich_flair(
					val(post, "author_flair_type"),
					post["data"]["author_flair_richtext"].as_array(),
					post["data"]["author_flair_text"].as_str(),
				),
				background_color: val(post, "author_flair_background_color"),
				foreground_color: val(post, "author_flair_text_color"),
			},
			score: format_num(score),
			upvote_ratio: ratio as i64,
			post_type,
			thumbnail: format_url(val(post, "thumbnail").as_str()),
			media,
			domain: val(post, "domain"),
			flair: Flair {
				flair_parts: parse_rich_flair(
					val(post, "link_flair_type"),
					post["data"]["link_flair_richtext"].as_array(),
					post["data"]["link_flair_text"].as_str(),
				),
				background_color: val(post, "link_flair_background_color"),
				foreground_color: if val(post, "link_flair_text_color") == "dark" {
					"black".to_string()
				} else {
					"white".to_string()
				},
			},
			flags: Flags {
				nsfw: post["data"]["over_18"].as_bool().unwrap_or_default(),
				stickied: post["data"]["stickied"].as_bool().unwrap_or_default(),
			},
			permalink: val(post, "permalink"),
			time: time(unix_time),
		});
	}

	Ok((posts, res["data"]["after"].as_str().unwrap_or_default().to_string()))
}

//
// NETWORKING
//

pub async fn error(msg: &str) -> HttpResponse {
	let body = ErrorTemplate {
		message: msg.to_string(),
		prefs: Preferences::default(),
	}
	.render()
	.unwrap_or_default();
	HttpResponse::NotFound().content_type("text/html").body(body)
}

// Make a request to a Reddit API and parse the JSON response
pub async fn request(path: &str) -> Result<serde_json::Value, &'static str> {
	let url = format!("https://www.reddit.com{}", path);

	// Send request using ureq
	match ureq::get(&url).call() {
		// If response is success
		Ok(response) => {
			// Parse the response from Reddit as JSON
			match from_str(&response.into_string().unwrap()) {
				Ok(json) => Ok(json),
				Err(_) => {
					#[cfg(debug_assertions)]
					dbg!(format!("{} - Failed to parse page JSON data", url));
					Err("Failed to parse page JSON data")
				}
			}
		}
		// If response is error
		Err(ureq::Error::Status(_, _)) => {
			#[cfg(debug_assertions)]
			dbg!(format!("{} - Page not found", url));
			Err("Page not found")
		}
		// If failed to send request
		Err(e) => {
			#[cfg(debug_assertions)]
			dbg!(e);
			Err("Couldn't send request to Reddit")
		}
	}
}
