// CRATES
use actix_web::{get, web, HttpResponse, Result};
use askama::Template;
use chrono::{TimeZone, Utc};

// STRUCTS
#[derive(Template)]
#[template(path = "subreddit.html", escape = "none")]
struct SubredditTemplate {
	sub: Subreddit,
	posts: Vec<Post>,
	sort: String,
}

// Post flair with text, background color and foreground color
pub struct Flair(pub String, pub String, pub String);

pub struct Post {
	pub title: String,
	pub community: String,
	pub author: String,
	pub score: String,
	pub image: String,
	pub url: String,
	pub time: String,
	pub flair: Flair,
}

pub struct Subreddit {
	pub name: String,
	pub title: String,
	pub description: String,
	pub icon: String,
}

async fn render(sub_name: String, sort: String) -> Result<HttpResponse> {
	let mut sub: Subreddit = subreddit(&sub_name).await;
	let posts: Vec<Post> = posts(sub_name, &sort).await;

	sub.icon = if sub.icon != "" {
		format!(r#"<img class="subreddit_icon" src="{}">"#, sub.icon)
	} else {
		String::new()
	};

	let s = SubredditTemplate { sub: sub, posts: posts, sort: sort }.render().unwrap();
	Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

// SERVICES
#[allow(dead_code)]
#[get("/r/{sub}")]
async fn page(web::Path(sub): web::Path<String>) -> Result<HttpResponse> {
	render(sub, String::from("hot")).await
}

#[allow(dead_code)]
#[get("/r/{sub}/{sort}")]
async fn sorted(web::Path((sub, sort)): web::Path<(String, String)>) -> Result<HttpResponse> {
	render(sub, sort).await
}

// UTILITIES
async fn val(j: &serde_json::Value, k: &str) -> String {
	String::from(j["data"][k].as_str().unwrap_or(""))
}

// SUBREDDIT
async fn subreddit(sub: &String) -> Subreddit {
	let url: String = format!("https://www.reddit.com/r/{}/about.json", sub);
	let resp: String = reqwest::get(&url).await.unwrap().text().await.unwrap();

	let data: serde_json::Value = serde_json::from_str(resp.as_str()).expect("Failed to parse JSON");

	let icon: String = String::from(data["data"]["community_icon"].as_str().unwrap()); //val(&data, "community_icon");
	let icon_split: std::str::Split<&str> = icon.split("?");
	let icon_parts: Vec<&str> = icon_split.collect();

	Subreddit {
		name: val(&data, "display_name").await,
		title: val(&data, "title").await,
		description: val(&data, "public_description").await,
		icon: String::from(icon_parts[0]),
	}
}

// POSTS
pub async fn posts(sub: String, sort: &String) -> Vec<Post> {
	let url: String = format!("https://www.reddit.com/r/{}/{}.json", sub, sort);
	let resp: String = reqwest::get(&url).await.unwrap().text().await.unwrap();

	let popular: serde_json::Value = serde_json::from_str(resp.as_str()).expect("Failed to parse JSON");
	let post_list = popular["data"]["children"].as_array().unwrap();

	let mut posts: Vec<Post> = Vec::new();

	for post in post_list.iter() {
		let img = if val(post, "thumbnail").await.starts_with("https:/") {
			val(post, "thumbnail").await
		} else {
			String::new()
		};
		let unix_time: i64 = post["data"]["created_utc"].as_f64().unwrap().round() as i64;
		let score = post["data"]["score"].as_i64().unwrap();
		posts.push(Post {
			title: val(post, "title").await,
			community: val(post, "subreddit").await,
			author: val(post, "author").await,
			score: if score > 1000 { format!("{}k", score / 1000) } else { score.to_string() },
			image: img,
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
	posts
}
