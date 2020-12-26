//
// CRATES
//
use chrono::{TimeZone, Utc};
use serde_json::{from_str, Value};
// use surf::{client, get, middleware::Redirect};

#[cfg(feature = "proxy")]
use base64::encode;

//
// STRUCTS
//
// Post flair with text, background color and foreground color
pub struct Flair(pub String, pub String, pub String);

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
	pub nsfw: bool,
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
	pub icon: String,
	pub karma: i64,
	pub created: String,
	pub banner: String,
	pub description: String,
}

// Subreddit struct containing metadata about community
pub struct Subreddit {
	pub name: String,
	pub title: String,
	pub description: String,
	pub icon: String,
	pub members: String,
	pub active: String,
}

// Parser for query params, used in sorting (eg. /r/rust/?sort=hot)
#[derive(serde::Deserialize)]
pub struct Params {
	pub sort: Option<String>,
	pub after: Option<String>,
	pub before: Option<String>,
}

// Error template
#[derive(askama::Template)]
#[template(path = "error.html", escape = "none")]
pub struct ErrorTemplate {
	pub message: String,
}

//
// FORMATTING
//

// Direct urls to proxy if proxy is enabled
pub async fn format_url(url: String) -> String {
	if url.is_empty() {
		return String::new();
	};

	#[cfg(feature = "proxy")]
	return "/proxy/".to_string() + encode(url).as_str();

	#[cfg(not(feature = "proxy"))]
	return url.to_string();
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
pub async fn val(j: &serde_json::Value, k: &str) -> String {
	String::from(j["data"][k].as_str().unwrap_or(""))
}

// nested_val() function used to parse JSON from Reddit APIs
pub async fn nested_val(j: &serde_json::Value, n: &str, k: &str) -> String {
	String::from(j["data"][n][k].as_str().unwrap())
}

// Fetch posts of a user or subreddit
pub async fn fetch_posts(url: String, fallback_title: String) -> Result<(Vec<Post>, String), &'static str> {
	// Send a request to the url, receive JSON in response
	let req = request(url).await;

	// If the Reddit API returns an error, exit this function
	if req.is_err() {
		return Err(req.err().unwrap());
	}

	// Otherwise, grab the JSON output from the request
	let res = req.unwrap();

	// Fetch the list of posts from the JSON response
	let post_list = res["data"]["children"].as_array().unwrap();

	let mut posts: Vec<Post> = Vec::new();

	for post in post_list {
		let img = if val(post, "thumbnail").await.starts_with("https:/") {
			format_url(val(post, "thumbnail").await).await
		} else {
			String::new()
		};
		let unix_time: i64 = post["data"]["created_utc"].as_f64().unwrap().round() as i64;
		let score = post["data"]["score"].as_i64().unwrap();
		let title = val(post, "title").await;

		posts.push(Post {
			title: if title.is_empty() { fallback_title.to_owned() } else { title },
			community: val(post, "subreddit").await,
			body: val(post, "body").await,
			author: val(post, "author").await,
			author_flair: Flair(
				val(post, "author_flair_text").await,
				val(post, "author_flair_background_color").await,
				val(post, "author_flair_text_color").await,
			),
			score: format_num(score),
			post_type: "link".to_string(),
			media: img,
			flair: Flair(
				val(post, "link_flair_text").await,
				val(post, "link_flair_background_color").await,
				if val(post, "link_flair_text_color").await == "dark" {
					"black".to_string()
				} else {
					"white".to_string()
				},
			),
			nsfw: post["data"]["over_18"].as_bool().unwrap_or(false),
			url: val(post, "permalink").await,
			time: Utc.timestamp(unix_time, 0).format("%b %e '%y").to_string(),
		});
	}

	Ok((posts, res["data"]["after"].as_str().unwrap_or("").to_string()))
}

//
// NETWORKING
//

// Make a request to a Reddit API and parse the JSON response
pub async fn request(mut url: String) -> Result<serde_json::Value, &'static str> {
	url = format!("https://www.reddit.com/{}", url);

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
	let success = res.status().is_success();
	// Read the body of the response
	let body = res.text().await.unwrap();

	// Parse the response from Reddit as JSON
	let json: Value = from_str(body.as_str()).unwrap_or(Value::Null);

	if !success {
		println!("! {} - {}", url, "Page not found");
		Err("Page not found")
	} else if json == Value::Null {
		println!("! {} - {}", url, "Failed to parse page JSON data");
		Err("Failed to parse page JSON data")
	} else {
		Ok(json)
	}
}
