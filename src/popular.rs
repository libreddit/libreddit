// CRATES
use actix_web::{get, web, HttpResponse, Result};
use askama::Template;
use serde::Deserialize;

#[path = "subreddit.rs"]
mod subreddit;
use subreddit::{posts, Post};

// STRUCTS
#[derive(Template)]
#[template(path = "popular.html", escape = "none")]
struct PopularTemplate {
	posts: Vec<Post>,
	sort: String,
}

#[derive(Deserialize)]
pub struct Params {
	sort: Option<String>,
}

// RENDER
async fn render(sub_name: String, sort: String) -> Result<HttpResponse> {
	let posts: Vec<Post> = posts(sub_name, &sort).await;

	let s = PopularTemplate { posts: posts, sort: sort }.render().unwrap();
	Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

// SERVICES
#[get("/")]
pub async fn page(params: web::Query<Params>) -> Result<HttpResponse> {
	match &params.sort {
		Some(sort) => render("popular".to_string(), sort.to_string()).await,
		None => render("popular".to_string(), "hot".to_string()).await,
	}
}
