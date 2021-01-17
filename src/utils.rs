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

// Part of flair, either emoji or text
pub struct FlairPart {
	pub flair_part_type: String,
	pub value: String,
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
}

// Post containing content, metadata and media
pub struct Post {
	pub id: String,
	pub title: String,
	pub community: String,
	pub body: String,
	pub author: Author,
	pub permalink: String,
	pub score: String,
	pub upvote_ratio: i64,
	pub post_type: String,
	pub flair: Flair,
	pub flags: Flags,
	pub thumbnail: Media,
	pub media: Media,
	pub domain: String,
	pub rel_time: String,
	pub created: String,
	pub comments: String,
}

// Comment with content, post, score and data/time that it was posted
pub struct Comment {
	pub id: String,
	pub body: String,
	pub author: Author,
	pub score: String,
	pub rel_time: String,
	pub created: String,
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
	pub msg: String,
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
	match Url::parse(format!("https://libredd.it/{}", path).as_str()) {
		Ok(url) => url.query_pairs().into_owned().collect::<HashMap<_, _>>().get(value).unwrap_or(&String::new()).to_owned(),
		_ => String::new(),
	}
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
	if num >= 1_000_000 {
		format!("{}m", num / 1_000_000)
	} else if num >= 1000 {
		format!("{}k", num / 1_000)
	} else {
		num.to_string()
	}
}

pub async fn media(data: &Value) -> (String, Media) {
	let post_type: &str;
	// If post is a video, return the video
	let url = if data["preview"]["reddit_video_preview"]["fallback_url"].is_string() {
		post_type = "video";
		format_url(data["preview"]["reddit_video_preview"]["fallback_url"].as_str().unwrap_or_default())
	} else if data["secure_media"]["reddit_video"]["fallback_url"].is_string() {
		post_type = "video";
		format_url(data["secure_media"]["reddit_video"]["fallback_url"].as_str().unwrap_or_default())
	// Handle images, whether GIFs or pics
	} else if data["post_hint"].as_str().unwrap_or("") == "image" {
		let preview = data["preview"]["images"][0].clone();
		match preview["variants"]["mp4"].as_object() {
			// Return the mp4 if the media is a gif
			Some(gif) => {
				post_type = "gif";
				format_url(gif["source"]["url"].as_str().unwrap_or_default())
			}
			// Return the picture if the media is an image
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

	(
		post_type.to_string(),
		Media {
			url,
			width: data["preview"]["images"][0]["source"]["width"].as_i64().unwrap_or_default(),
			height: data["preview"]["images"][0]["source"]["height"].as_i64().unwrap_or_default(),
		},
	)
}

pub fn parse_rich_flair(flair_type: String, rich_flair: Option<&Vec<Value>>, text_flair: Option<&str>) -> Vec<FlairPart> {
	// Parse type of flair
	match flair_type.as_str() {
		// If flair contains emojis and text
		"richtext" => match rich_flair {
			Some(rich) => rich
				.iter()
				// For each part of the flair, extract text and emojis
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
		// If flair contains only text
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

	(rel_time, time.format("%b %d %Y, %H:%M UTC"))
}

//
// JSON PARSING
//

// val() function used to parse JSON from Reddit APIs
pub fn val(j: &Value, k: &str) -> String {
	String::from(j["data"][k].as_str().unwrap_or_default())
}

// Fetch posts of a user or subreddit and return a vector of posts and the "after" value
pub async fn fetch_posts(path: &str, fallback_title: String) -> Result<(Vec<Post>, String), String> {
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
		None => return Err("No posts found".to_string()),
	}

	let mut posts: Vec<Post> = Vec::new();

	// For each post from posts list
	for post in post_list {
		let (rel_time, created) = time(post["data"]["created_utc"].as_f64().unwrap_or_default());
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
			author: Author {
				name: val(post, "author"),
				flair: Flair {
					flair_parts: parse_rich_flair(
						val(post, "author_flair_type"),
						post["data"]["author_flair_richtext"].as_array(),
						post["data"]["author_flair_text"].as_str(),
					),
					background_color: val(post, "author_flair_background_color"),
					foreground_color: val(post, "author_flair_text_color"),
				},
				distinguished: val(post, "distinguished"),
			},
			score: format_num(score),
			upvote_ratio: ratio as i64,
			post_type,
			thumbnail: Media {
				url: format_url(val(post, "thumbnail").as_str()),
				width: post["data"]["thumbnail_width"].as_i64().unwrap_or_default(),
				height: post["data"]["thumbnail_height"].as_i64().unwrap_or_default(),
			},
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
			rel_time,
			created,
			comments: format_num(post["data"]["num_comments"].as_i64().unwrap_or_default()),
		});
	}

	Ok((posts, res["data"]["after"].as_str().unwrap_or_default().to_string()))
}

//
// NETWORKING
//

pub async fn error(msg: String) -> HttpResponse {
	let body = ErrorTemplate {
		msg,
		prefs: Preferences::default(),
	}
	.render()
	.unwrap_or_default();
	HttpResponse::NotFound().content_type("text/html").body(body)
}

// Make a request to a Reddit API and parse the JSON response
pub async fn request(path: &str) -> Result<Value, String> {
	let url = format!("https://www.reddit.com{}", path);
	let user_agent = format!("web:libreddit:{}", env!("CARGO_PKG_VERSION"));

	// Send request using awc
	// async fn send(url: &str) -> Result<String, (bool, String)> {
	// 	let client = actix_web::client::Client::default();
	// 	let response = client.get(url).header("User-Agent", format!("web:libreddit:{}", env!("CARGO_PKG_VERSION"))).send().await;

	// 	match response {
	// 		Ok(mut payload) => {
	// 			// Get first number of response HTTP status code
	// 			match payload.status().to_string().chars().next() {
	// 				// If success
	// 				Some('2') => Ok(String::from_utf8(payload.body().limit(20_000_000).await.unwrap_or_default().to_vec()).unwrap_or_default()),
	// 				// If redirection
	// 				Some('3') => match payload.headers().get("location") {
	// 					Some(location) => Err((true, location.to_str().unwrap_or_default().to_string())),
	// 					None => Err((false, "Page not found".to_string())),
	// 				},
	// 				// Otherwise
	// 				_ => Err((false, "Page not found".to_string())),
	// 			}
	// 		}
	// 		Err(e) => { dbg!(e); Err((false, "Couldn't send request to Reddit, this instance may be being rate-limited. Try another.".to_string())) },
	// 	}
	// }

	// // Print error if debugging then return error based on error message
	// fn err(url: String, msg: String) -> Result<Value, String> {
	// 	// #[cfg(debug_assertions)]
	// 	dbg!(format!("{} - {}", url, msg));
	// 	Err(msg)
	// };

	// // Parse JSON from body. If parsing fails, return error
	// fn json(url: String, body: String) -> Result<Value, String> {
	// 	match from_str(body.as_str()) {
	// 		Ok(json) => Ok(json),
	// 		Err(_) => err(url, "Failed to parse page JSON data".to_string()),
	// 	}
	// }

	// // Make request to Reddit using send function
	// match send(&url).await {
	// 	// If success, parse and return body
	// 	Ok(body) => json(url, body),
	// 	// Follow any redirects
	// 	Err((true, location)) => match send(location.as_str()).await {
	// 		// If success, parse and return body
	// 		Ok(body) => json(url, body),
	// 		// Follow any redirects again
	// 		Err((true, location)) => err(url, location),
	// 		// Return errors if request fails
	// 		Err((_, msg)) => err(url, msg),
	// 	},
	// 	// Return errors if request fails
	// 	Err((_, msg)) => err(url, msg),
	// }

	// Send request using ureq
	match ureq::get(&url).set("User-Agent", user_agent.as_str()).call() {
		// If response is success
		Ok(response) => {
			// Parse the response from Reddit as JSON
			match from_str(&response.into_string().unwrap()) {
				Ok(json) => Ok(json),
				Err(_) => {
					#[cfg(debug_assertions)]
					dbg!(format!("{} - Failed to parse page JSON data", url));
					Err("Failed to parse page JSON data".to_string())
				}
			}
		}
		// If response is error
		Err(ureq::Error::Status(_, _)) => {
			#[cfg(debug_assertions)]
			dbg!(format!("{} - Page not found", url));
			Err("Page not found".to_string())
		}
		// If failed to send request
		Err(e) => {
			#[cfg(debug_assertions)]
			dbg!(format!("{} - {}", url, e));
			Err("Couldn't send request to Reddit, this instance may be being rate-limited. Try another.".to_string())
		}
	}
}
