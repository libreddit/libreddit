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
#[allow(dead_code)]
// Post flair with text, background color and foreground color
pub struct Flair(pub String, pub String, pub String);

#[allow(dead_code)]
// Post containing content, metadata and media
pub struct Post {
	pub title: String,
	pub community: String,
	pub body: String,
	pub author: String,
	pub url: String,
	pub score: String,
	pub post_type: String,
	pub media: String,
	pub time: String,
	pub flair: Flair,
}

#[allow(dead_code)]
// Comment with content, post, score and data/time that it was posted
pub struct Comment {
	pub body: String,
	pub author: String,
	pub score: String,
	pub time: String,
}

#[allow(dead_code)]
// User struct containing metadata about user
pub struct User {
	pub name: String,
	pub icon: String,
	pub karma: i64,
	pub banner: String,
	pub description: String,
}

#[allow(dead_code)]
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
// URL HANDLING
//

pub async fn format_url(url: &str) -> String {
	#[cfg(feature = "proxy")]
	return "/imageproxy/".to_string() + encode(url).as_str();

	#[cfg(not(feature = "proxy"))]
	return url.to_string();
}

//
// JSON PARSING
//

#[allow(dead_code)]
// val() function used to parse JSON from Reddit APIs
pub async fn val(j: &serde_json::Value, k: &str) -> String {
	String::from(j["data"][k].as_str().unwrap_or(""))
}

#[allow(dead_code)]
// nested_val() function used to parse JSON from Reddit APIs
pub async fn nested_val(j: &serde_json::Value, n: &str, k: &str) -> String {
	String::from(j["data"][n][k].as_str().unwrap())
}

#[allow(dead_code)]
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

	for post in post_list.iter() {
		let img = if val(post, "thumbnail").await.starts_with("https:/") {
			format_url(val(post, "thumbnail").await.as_str()).await
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
			score: if score > 1000 { format!("{}k", score / 1000) } else { score.to_string() },
			post_type: "link".to_string(),
			media: img,
			url: val(post, "permalink").await,
			time: Utc.timestamp(unix_time, 0).format("%b %e '%y").to_string(),
			flair: Flair(
				val(post, "link_flair_text").await,
				val(post, "link_flair_background_color").await,
				if val(post, "link_flair_text_color").await == "dark" {
					"black".to_string()
				} else {
					"white".to_string()
				},
			),
		});
	}

	Ok((posts, res["data"]["after"].as_str().unwrap_or("").to_string()))
}

//
// NETWORKING
//

// Make a request to a Reddit API and parse the JSON response
#[allow(dead_code)]
pub async fn request(url: String) -> Result<serde_json::Value, &'static str> {
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

	dbg!(url.clone());

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
