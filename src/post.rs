// CRATES
use crate::utils::{format_num, format_url, request, val, Comment, ErrorTemplate, Flair, Params, Post};
use actix_web::{http::StatusCode, web, HttpResponse, Result};

use async_recursion::async_recursion;

use askama::Template;
use chrono::{TimeZone, Utc};
use pulldown_cmark::{html, Options, Parser};

// STRUCTS
#[derive(Template)]
#[template(path = "post.html", escape = "none")]
struct PostTemplate {
	comments: Vec<Comment>,
	post: Post,
	sort: String,
}

async fn render(id: String, sort: String) -> Result<HttpResponse> {
	// Log the post ID being fetched
	dbg!(&id);

	// Build the Reddit JSON API url
	let url: String = format!("https://reddit.com/{}.json?sort={}", id, sort);

	// Send a request to the url, receive JSON in response
	let req = request(url).await;

	// If the Reddit API returns an error, exit and send error page to user
	if req.is_err() {
		let s = ErrorTemplate {
			message: req.err().unwrap().to_string(),
		}
		.render()
		.unwrap();
		return Ok(HttpResponse::Ok().status(StatusCode::NOT_FOUND).content_type("text/html").body(s));
	}

	// Otherwise, grab the JSON output from the request
	let res = req.unwrap();

	// Parse the JSON into Post and Comment structs
	let post = parse_post(res[0].clone()).await;
	let comments = parse_comments(res[1].clone()).await;

	// Use the Post and Comment structs to generate a website to show users
	let s = PostTemplate {
		comments: comments.unwrap(),
		post: post.unwrap(),
		sort: sort,
	}
	.render()
	.unwrap();
	Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

// SERVICES
pub async fn short(web::Path(id): web::Path<String>, params: web::Query<Params>) -> Result<HttpResponse> {
	match &params.sort {
		Some(sort) => render(id, sort.to_string()).await,
		None => render(id, "confidence".to_string()).await,
	}
}

pub async fn page(web::Path((_sub, id)): web::Path<(String, String)>, params: web::Query<Params>) -> Result<HttpResponse> {
	match &params.sort {
		Some(sort) => render(id, sort.to_string()).await,
		None => render(id, "confidence".to_string()).await,
	}
}

// UTILITIES
async fn media(data: &serde_json::Value) -> (String, String) {
	let post_type: &str;
	let url = if !data["preview"]["reddit_video_preview"]["fallback_url"].is_null() {
		post_type = "video";
		format_url(data["preview"]["reddit_video_preview"]["fallback_url"].as_str().unwrap()).await
	} else if !data["secure_media"]["reddit_video"]["fallback_url"].is_null() {
		post_type = "video";
		format_url(data["secure_media"]["reddit_video"]["fallback_url"].as_str().unwrap()).await
	} else if data["post_hint"].as_str().unwrap_or("") == "image" {
		post_type = "image";
		format_url(data["preview"]["images"][0]["source"]["url"].as_str().unwrap()).await
	} else {
		post_type = "link";
		data["url"].as_str().unwrap().to_string()
	};

	(post_type.to_string(), url)
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
async fn parse_post(json: serde_json::Value) -> Result<Post, &'static str> {
	let post_data: &serde_json::Value = &json["data"]["children"][0];

	let unix_time: i64 = post_data["data"]["created_utc"].as_f64().unwrap().round() as i64;
	let score = post_data["data"]["score"].as_i64().unwrap();

	let media = media(&post_data["data"]).await;

	let post = Post {
		title: val(post_data, "title").await,
		community: val(post_data, "subreddit").await,
		body: markdown_to_html(post_data["data"]["selftext"].as_str().unwrap()).await,
		author: val(post_data, "author").await,
		url: val(post_data, "permalink").await,
		score: format_num(score),
		post_type: media.0,
		media: media.1,
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
#[async_recursion]
async fn parse_comments(json: serde_json::Value) -> Result<Vec<Comment>, &'static str> {
	// Separate the comment JSON into a Vector of comments
	let comment_data = json["data"]["children"].as_array().unwrap();

	let mut comments: Vec<Comment> = Vec::new();

	// For each comment, retrieve the values to build a Comment object
	for comment in comment_data.iter() {
		let unix_time: i64 = comment["data"]["created_utc"].as_f64().unwrap_or(0.0).round() as i64;
		if unix_time == 0 {
			continue;
		}

		let score = comment["data"]["score"].as_i64().unwrap_or(0);
		let body = markdown_to_html(comment["data"]["body"].as_str().unwrap_or("")).await;

		let replies: Vec<Comment> = if comment["data"]["replies"].is_object() {
			parse_comments(comment["data"]["replies"].clone()).await.unwrap_or(Vec::new())
		} else {
			Vec::new()
		};

		comments.push(Comment {
			body: body,
			author: val(comment, "author").await,
			score: format_num(score),
			time: Utc.timestamp(unix_time, 0).format("%b %e %Y %H:%M UTC").to_string(),
			replies: replies,
		});
	}

	Ok(comments)
}
