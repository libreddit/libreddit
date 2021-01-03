// use std::collections::HashMap;

//
// CRATES
//
use actix_web::{HttpResponse, Result};
use askama::Template;
use base64::encode;
use chrono::{TimeZone, Utc};
use regex::Regex;
use serde_json::from_str;
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
	pub title: String,
	pub community: String,
	pub body: String,
	pub author: String,
	pub author_flair: Flair,
	pub url: String,
	pub score: String,
	pub post_type: String,
	pub flair: Flair,
	pub flags: Flags,
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
}

//
// FORMATTING
//

// Grab a query param from a url
pub fn param(path: &str, value: &str) -> String {
	let url = Url::parse(format!("https://reddit.com/{}", path).as_str()).unwrap();
	let pairs: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
	pairs.get(value).unwrap_or(&String::new()).to_owned()
}

// Cookies from request
// pub fn cookies(req: HttpRequest) -> HashMap<String, String> {
// 	let mut result: HashMap<String, String> = HashMap::new();

// 	let cookies: Vec<Cookie> = req
// 		.headers()
// 		.get_all("Cookie")
// 		.map(|value| value.to_str().unwrap())
// 		.map(|unparsed| Cookie::parse(unparsed).unwrap())
// 		.collect();

// 	for cookie in cookies {
// 		result.insert(cookie.name().to_string(), cookie.value().to_string());
// 	}

// 	result
// }

// Direct urls to proxy if proxy is enabled
pub fn format_url(url: String) -> String {
	if url.is_empty() {
		return String::new();
	};

	format!("/proxy/{}", encode(url).as_str())
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

// Fetch posts of a user or subreddit
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

	for post in post_list {
		let img = if val(post, "thumbnail").starts_with("https:/") {
			format_url(val(post, "thumbnail"))
		} else {
			String::new()
		};
		let unix_time: i64 = post["data"]["created_utc"].as_f64().unwrap_or_default().round() as i64;
		let score = post["data"]["score"].as_i64().unwrap_or_default();
		let title = val(post, "title");

		posts.push(Post {
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
			post_type: "link".to_string(),
			media: img,
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
				nsfw: post["data"]["over_18"].as_bool().unwrap_or(false),
				stickied: post["data"]["stickied"].as_bool().unwrap_or(false),
			},
			url: val(post, "permalink"),
			time: Utc.timestamp(unix_time, 0).format("%b %e '%y").to_string(),
		});
	}

	Ok((posts, res["data"]["after"].as_str().unwrap_or("").to_string()))
}

//
// NETWORKING
//

pub async fn error(message: String) -> HttpResponse {
	let msg = if message.is_empty() { "Page not found".to_string() } else { message };
	let body = ErrorTemplate { message: msg }.render().unwrap_or_default();
	HttpResponse::NotFound().content_type("text/html").body(body)
}

// Make a request to a Reddit API and parse the JSON response
pub async fn request(path: &str) -> Result<serde_json::Value, &'static str> {
	let url = format!("https://www.reddit.com/{}", path);

	// --- actix-web::client ---
	// let client = actix_web::client::Client::default();
	// let res = client
	// 	.get(url)
	// 	.send()
	// 	.await?
	// 	.body()
	// 	.limit(1000000)
	// 	.await?;

	// let body = std::str::from_utf8(res.as_ref())?; // .as_ref converts Bytes to [u8]

	// --- surf ---
	// let req = get(&url).header("User-Agent", "libreddit");
	// let client = client().with(Redirect::new(5));
	// let mut res = client.send(req).await.unwrap();
	// let success = res.status().is_success();
	// let body = res.body_string().await.unwrap();

	// --- reqwest ---
	let res = reqwest::get(&url).await.unwrap();
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
		false => {
			#[cfg(debug_assertions)]
			dbg!(format!("{} - Page not found", url));
			Err("Page not found")
		}
	}
}
