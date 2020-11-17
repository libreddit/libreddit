// CRATES
use actix_web::{get, web, HttpResponse, Result};
use askama::Template;
use chrono::{TimeZone, Utc};
use pulldown_cmark::{html, Options, Parser};

#[path = "utils.rs"]
mod utils;
use utils::{Comment, Flair, Post, val};

// STRUCTS
#[derive(Template)]
#[template(path = "post.html", escape = "none")]
struct PostTemplate {
	comments: Vec<Comment>,
	post: Post,
	sort: String,
}

async fn render(id: String, sort: String) -> Result<HttpResponse> {
	println!("id: {}", id);
	let post: Post = fetch_post(&id).await;
	let comments: Vec<Comment> = fetch_comments(id, &sort).await.unwrap();

	let s = PostTemplate {
		comments: comments,
		post: post,
		sort: sort,
	}
	.render()
	.unwrap();

	// println!("{}", s);

	Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

// SERVICES
#[get("/{id}")]
async fn short(web::Path(id): web::Path<String>) -> Result<HttpResponse> {
	render(id.to_string(), "confidence".to_string()).await
}

#[get("/r/{sub}/comments/{id}/{title}/")]
async fn page(web::Path((_sub, id)): web::Path<(String, String)>) -> Result<HttpResponse> {
	render(id.to_string(), "confidence".to_string()).await
}

#[get("/r/{sub}/comments/{id}/{title}/{sort}")]
async fn sorted(web::Path((_sub, id, _title, sort)): web::Path<(String, String, String, String)>) -> Result<HttpResponse> {
	render(id.to_string(), sort).await
}

// UTILITIES
async fn media(data: &serde_json::Value) -> String {
	let post_hint: &str = data["data"]["post_hint"].as_str().unwrap_or("");
	let has_media: bool = data["data"]["media"].is_object();

	let media: String = if !has_media {
		format!(r#"<h4 class="post_body"><a href="{u}">{u}</a></h4>"#, u = data["data"]["url"].as_str().unwrap())
	} else {
		format!(r#"<img class="post_image" src="{}.png"/>"#, data["data"]["url"].as_str().unwrap())
	};

	match post_hint {
		"hosted:video" => format!(
			r#"<video class="post_image" src="{}" controls/>"#,
			data["data"]["media"]["reddit_video"]["fallback_url"].as_str().unwrap()
		),
		"image" => format!(r#"<img class="post_image" src="{}"/>"#, data["data"]["url"].as_str().unwrap()),
		"self" => String::from(""),
		_ => media,
	}
}

async fn markdown_to_html(md: &str) -> String {
	let mut options = Options::empty();
	options.insert(Options::ENABLE_TABLES);
	options.insert(Options::ENABLE_FOOTNOTES);
	options.insert(Options::ENABLE_STRIKETHROUGH);
	options.insert(Options::ENABLE_TASKLISTS);
	let parser = Parser::new_ext(md, options);

	// Write to String buffer.
	let mut html_output = String::new();
	html::push_html(&mut html_output, parser);
	html_output
}

// POSTS
async fn fetch_post(id: &String) -> Post {
	let url: String = format!("https://reddit.com/{}.json", id);
	let resp: String = reqwest::get(&url).await.unwrap().text().await.unwrap();

	let data: serde_json::Value = serde_json::from_str(resp.as_str()).expect("Failed to parse JSON");

	let post_data: &serde_json::Value = &data[0]["data"]["children"][0];

	let unix_time: i64 = post_data["data"]["created_utc"].as_f64().unwrap().round() as i64;
	let score = post_data["data"]["score"].as_i64().unwrap();

	Post {
		title: val(post_data, "title").await,
		community: val(post_data, "subreddit").await,
		body: markdown_to_html(post_data["data"]["selftext"].as_str().unwrap()).await,
		author: val(post_data, "author").await,
		url: val(post_data, "permalink").await,
		score: if score > 1000 { format!("{}k", score / 1000) } else { score.to_string() },
		media: media(post_data).await,
		time: Utc.timestamp(unix_time, 0).format("%b %e %Y %H:%M UTC").to_string(),
		flair: Flair(
			val(post_data, "link_flair_text").await,
			val(post_data, "link_flair_background_color").await,
			if val(post_data, "link_flair_text_color").await == "dark" {
				"black".to_string()
			} else {
				"white".to_string()
			},
		),
	}
}

// COMMENTS
async fn fetch_comments(id: String, sort: &String) -> Result<Vec<Comment>, Box<dyn std::error::Error>> {
	let url: String = format!("https://reddit.com/{}.json?sort={}", id, sort);
	let resp: String = reqwest::get(&url).await?.text().await?;

	let data: serde_json::Value = serde_json::from_str(resp.as_str())?;

	let comment_data = data[1]["data"]["children"].as_array().unwrap();

	let mut comments: Vec<Comment> = Vec::new();

	for comment in comment_data.iter() {
		let unix_time: i64 = comment["data"]["created_utc"].as_f64().unwrap_or(0.0).round() as i64;
		let score = comment["data"]["score"].as_i64().unwrap_or(0);
		let body = markdown_to_html(comment["data"]["body"].as_str().unwrap_or("")).await;

		// println!("{}", body);

		comments.push(Comment {
			body: body,
			author: val(comment, "author").await,
			score: if score > 1000 { format!("{}k", score / 1000) } else { score.to_string() },
			time: Utc.timestamp(unix_time, 0).format("%b %e %Y %H:%M UTC").to_string(),
		});
	}

	Ok(comments)
}
