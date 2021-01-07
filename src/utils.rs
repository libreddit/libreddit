// use std::collections::HashMap;

use std::collections::HashMap;

//
// CRATES
//
use actix_web::{cookie::Cookie, HttpResponse, Result};
use askama::Template;
use base64::encode;
use regex::Regex;
use serde_json::from_str;
use time::OffsetDateTime;
use url::Url;
// use surf::{client, get, middleware::Redirect};

//
// STRUCTS
//
// Post flair with text, background color and foreground color
pub struct Flair(pub String, pub String, pub String);
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
	pub layout: String,
}

//
// FORMATTING
//

// Grab a query param from a url
pub fn param(path: &str, value: &str) -> String {
	let url = Url::parse(format!("https://libredd.it/{}", path).as_str()).unwrap();
	let pairs: HashMap<_, _> = url.query_pairs().into_owned().collect();
	pairs.get(value).unwrap_or(&String::new()).to_owned()
}

// Parse Cookie value from request
pub fn cookie(req: actix_web::HttpRequest, name: &str) -> String {
	actix_web::HttpMessage::cookie(&req, name).unwrap_or_else(|| Cookie::new(name, "")).value().to_string()
}

// Direct urls to proxy if proxy is enabled
pub fn format_url(url: String) -> String {
	if url.is_empty() || url == "self" || url == "default" || url == "nsfw" {
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
	if num > 1000000 {
		format!("{}m", num / 1000000)
	} else if num > 1000 {
		format!("{}k", num / 1000)
	} else {
		num.to_string()
	}
}

pub async fn media(data: &serde_json::Value) -> (String, String) {
	let post_type: &str;
	let url = if !data["preview"]["reddit_video_preview"]["fallback_url"].is_null() {
		post_type = "video";
		format_url(data["preview"]["reddit_video_preview"]["fallback_url"].as_str().unwrap_or_default().to_string())
	} else if !data["secure_media"]["reddit_video"]["fallback_url"].is_null() {
		post_type = "video";
		format_url(data["secure_media"]["reddit_video"]["fallback_url"].as_str().unwrap_or_default().to_string())
	} else if data["post_hint"].as_str().unwrap_or("") == "image" {
		post_type = "image";
		format_url(data["preview"]["images"][0]["source"]["url"].as_str().unwrap_or_default().to_string())
	} else {
		post_type = "link";
		data["url"].as_str().unwrap_or_default().to_string()
	};

	(post_type.to_string(), url)
}

//
// JSON PARSING
//

// val() function used to parse JSON from Reddit APIs
pub fn val(j: &serde_json::Value, k: &str) -> String {
	String::from(j["data"][k].as_str().unwrap_or_default())
}

// nested_val() function used to parse JSON from Reddit APIs
pub fn nested_val(j: &serde_json::Value, n: &str, k: &str) -> String {
	String::from(j["data"][n][k].as_str().unwrap_or_default())
}

// Fetch posts of a user or subreddit and return a vector of posts and the "after" value
pub async fn fetch_posts(path: &str, fallback_title: String) -> Result<(Vec<Post>, String), &'static str> {
	let res;
	let post_list;

	// Send a request to the url
	match request(&path).await {
		// If success, receive JSON in response
		Ok(response) => { res = response;	}
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
			author_flair: Flair(
				val(post, "author_flair_text"),
				val(post, "author_flair_background_color"),
				val(post, "author_flair_text_color"),
			),
			score: format_num(score),
			upvote_ratio: ratio as i64,
			post_type,
			thumbnail: format_url(val(post, "thumbnail")),
			media,
			flair: Flair(
				val(post, "link_flair_text"),
				val(post, "link_flair_background_color"),
				if val(post, "link_flair_text_color") == "dark" {
					"black".to_string()
				} else {
					"white".to_string()
				},
			),
			flags: Flags {
				nsfw: post["data"]["over_18"].as_bool().unwrap_or_default(),
				stickied: post["data"]["stickied"].as_bool().unwrap_or_default(),
			},
			permalink: val(post, "permalink"),
			time: OffsetDateTime::from_unix_timestamp(unix_time).format("%b %d '%y"), // %b %e '%y
		});
	}

	Ok((posts, res["data"]["after"].as_str().unwrap_or_default().to_string()))
}

//
// NETWORKING
//

pub async fn error(msg: String) -> HttpResponse {
	let body = ErrorTemplate {
		message: msg,
		layout: String::new(),
	}
	.render()
	.unwrap_or_default();
	HttpResponse::NotFound().content_type("text/html").body(body)
}

// Make a request to a Reddit API and parse the JSON response
pub async fn request(path: &str) -> Result<serde_json::Value, &'static str> {
	let url = format!("https://www.reddit.com/{}", path);

	// Send request using reqwest
	match reqwest::get(&url).await {
		Ok(res) => {
			// Read the status from the response
			match res.status().is_success() {
				true => {
					// Parse the response from Reddit as JSON
					match from_str(res.text().await.unwrap_or_default().as_str()) {
						Ok(json) => Ok(json),
						Err(_) => {
							#[cfg(debug_assertions)]
							dbg!(format!("{} - Failed to parse page JSON data", url));
							Err("Failed to parse page JSON data")
						}
					}
				}
				// If Reddit returns error, tell user Page Not Found
				false => {
					#[cfg(debug_assertions)]
					dbg!(format!("{} - Page not found", url));
					Err("Page not found")
				}
			}
		}
		// If can't send request to Reddit, return this to user
		Err(e) => {
			#[cfg(debug_assertions)]
			dbg!(format!("{} - {}", url, e));
			Err("Couldn't send request to Reddit")
		}
	}
}
