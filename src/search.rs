// CRATES
use crate::utils::{error, fetch_posts, param, Post};
use actix_web::{HttpRequest, HttpResponse, Result};
use askama::Template;

// STRUCTS
#[derive(Template)]
#[allow(dead_code)]
#[template(path = "search.html", escape = "none")]
struct SearchTemplate {
	posts: Vec<Post>,
	query: String,
	sub: String,
	sort: (String, String),
	ends: (String, String),
}

// SERVICES
pub async fn find(req: HttpRequest) -> Result<HttpResponse> {
	let path = format!("{}.json?{}", req.path(), req.query_string());
	let q = param(&path, "q").await;
	let sort = if param(&path, "sort").await.is_empty() {
		"relevance".to_string()
	} else {
		param(&path, "sort").await
	};
	let sub = req.match_info().get("sub").unwrap_or("").to_string();

	let posts = fetch_posts(path.clone(), String::new()).await;

	if posts.is_err() {
		error(posts.err().unwrap().to_string()).await
	} else {
		let items = posts.unwrap();

		let s = SearchTemplate {
			posts: items.0,
			query: q,
			sub: sub,
			sort: (sort, param(&path, "t").await),
			ends: (param(&path, "after").await, items.1),
		}
		.render()
		.unwrap();
		Ok(HttpResponse::Ok().content_type("text/html").body(s))
	}
}
