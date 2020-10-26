// CRATES
use actix_web::{get, web, HttpResponse, Result};
use askama::Template;
use chrono::{TimeZone, Utc};

// STRUCTS
#[derive(Template)]
#[template(path = "user.html", escape = "none")]
struct UserTemplate {
	user: User,
	posts: Vec<Post>,
	sort: String,
}

pub struct Post {
	pub title: String,
	pub community: String,
	pub author: String,
	pub score: String,
	pub image: String,
	pub url: String,
	pub time: String,
}

pub struct User {
	pub name: String,
	pub icon: String,
	pub karma: i64,
	pub banner: String,
	pub description: String,
}

async fn render(username: String, sort: String) -> Result<HttpResponse> {
	let user: User = user(&username).await;
	let posts: Vec<Post> = posts(username, &sort).await;

	let s = UserTemplate { user: user, posts: posts, sort: sort }.render().unwrap();
	Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

// SERVICES
#[get("/u/{username}")]
async fn page(web::Path(username): web::Path<String>) -> Result<HttpResponse> {
	render(username, "hot".to_string()).await
}

#[get("/u/{username}/{sort}")]
async fn sorted(web::Path((username, sort)): web::Path<(String, String)>) -> Result<HttpResponse> {
	render(username, sort).await
}

// UTILITIES
async fn user_val(j: &serde_json::Value, k: &str) -> String {
	String::from(j["data"]["subreddit"][k].as_str().unwrap())
}
async fn post_val(j: &serde_json::Value, k: &str) -> String {
	String::from(j["data"][k].as_str().unwrap_or("Comment"))
}

// USER
async fn user(name: &String) -> User {
	let url: String = format!("https://www.reddit.com/user/{}/about.json", name);
	let resp: String = reqwest::get(&url).await.unwrap().text().await.unwrap();

	let data: serde_json::Value = serde_json::from_str(resp.as_str()).expect("Failed to parse JSON");

	User {
		name: name.to_string(),
		icon: user_val(&data, "icon_img").await,
		karma: data["data"]["total_karma"].as_i64().unwrap(),
		banner: user_val(&data, "banner_img").await,
		description: user_val(&data, "public_description").await,
	}
}

// POSTS
async fn posts(sub: String, sort: &String) -> Vec<Post> {
	let url: String = format!("https://www.reddit.com/u/{}/.json?sort={}", sub, sort);
	let resp: String = reqwest::get(&url).await.unwrap().text().await.unwrap();

	let popular: serde_json::Value = serde_json::from_str(resp.as_str()).expect("Failed to parse JSON");
	let post_list = popular["data"]["children"].as_array().unwrap();

	let mut posts: Vec<Post> = Vec::new();

	for post in post_list.iter() {
		let img = if post_val(post, "thumbnail").await.starts_with("https:/") {
			post_val(post, "thumbnail").await
		} else {
			String::new()
		};
		let unix_time: i64 = post["data"]["created_utc"].as_f64().unwrap().round() as i64;
		let score = post["data"]["score"].as_i64().unwrap();
		posts.push(Post {
			title: post_val(post, "title").await,
			community: post_val(post, "subreddit").await,
			author: post_val(post, "author").await,
			score: if score > 1000 { format!("{}k", score / 1000) } else { score.to_string() },
			image: img,
			url: post_val(post, "permalink").await,
			time: Utc.timestamp(unix_time, 0).format("%b %e '%y").to_string(),
		});
	}

	posts
}
