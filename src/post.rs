// CRATES
use actix_web::{get, web, HttpResponse, Result};
use askama::Template;
use chrono::{TimeZone, Utc};
use pulldown_cmark::{html, Options, Parser};

#[path = "utils.rs"]
mod utils;
use utils::{request, val, Comment, ErrorTemplate, Flair, Params, Post};

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
	let post = fetch_post(&id).await;
	let comments = fetch_comments(id, &sort).await;

	if post.is_err() || comments.is_err() {
		let s = ErrorTemplate {
			message: post.err().unwrap().to_string(),
		}
		.render()
		.unwrap();
		Ok(HttpResponse::Ok().content_type("text/html").body(s))
	} else {
		let s = PostTemplate {
			comments: comments.unwrap(),
			post: post.unwrap(),
			sort: sort,
		}
		.render()
		.unwrap();
		Ok(HttpResponse::Ok().content_type("text/html").body(s))
	}
}

// SERVICES
#[get("/{id}")]
async fn short(web::Path(id): web::Path<String>) -> Result<HttpResponse> {
	render(id.to_string(), "confidence".to_string()).await
}

#[get("/r/{sub}/comments/{id}/{title}/")]
async fn page(web::Path((_sub, id)): web::Path<(String, String)>, params: web::Query<Params>) -> Result<HttpResponse> {
	match &params.sort {
		Some(sort) => render(id, sort.to_string()).await,
		None => render(id, "confidence".to_string()).await,
	}
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
async fn fetch_post(id: &String) -> Result<Post, &'static str> {
	// Build the Reddit JSON API url
	let url: String = format!("https://reddit.com/{}.json", id);

	// Send a request to the url, receive JSON in response
	let req = request(url).await;

	// If the Reddit API returns an error, exit this function
	if req.is_err() {
		return Err(req.err().unwrap());
	}

	// Otherwise, grab the JSON output from the request
	let res = req.unwrap();

	let post_data: &serde_json::Value = &res[0]["data"]["children"][0];

	let unix_time: i64 = post_data["data"]["created_utc"].as_f64().unwrap().round() as i64;
	let score = post_data["data"]["score"].as_i64().unwrap();

	let post = Post {
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
	};

	Ok(post)
}

// COMMENTS
async fn fetch_comments(id: String, sort: &String) -> Result<Vec<Comment>, &'static str> {
	// Build the Reddit JSON API url
	let url: String = format!("https://reddit.com/{}.json?sort={}", id, sort);

	// Send a request to the url, receive JSON in response
	let req = request(url).await;

	// If the Reddit API returns an error, exit this function
	if req.is_err() {
		return Err(req.err().unwrap());
	}

	// Otherwise, grab the JSON output from the request
	let res = req.unwrap();

	let comment_data = res[1]["data"]["children"].as_array().unwrap();

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
