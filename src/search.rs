// CRATES
use crate::utils::{error, fetch_posts, param, Post};
use actix_web::{HttpRequest, HttpResponse};
use askama::Template;

// STRUCTS
#[derive(Template)]
#[template(path = "search.html", escape = "none")]
struct SearchTemplate {
	posts: Vec<Post>,
	query: String,
	sub: String,
	sort: (String, String),
	ends: (String, String),
}

// SERVICES
pub async fn find(req: HttpRequest) -> HttpResponse {
	let path = format!("{}.json?{}", req.path(), req.query_string());
	let q = param(&path, "q");
	let sort = if param(&path, "sort").is_empty() {
		"relevance".to_string()
	} else {
		param(&path, "sort")
	};
	let sub = req.match_info().get("sub").unwrap_or("").to_string();

	match fetch_posts(&path, String::new()).await {
		Ok(posts) => HttpResponse::Ok().content_type("text/html").body(
			SearchTemplate {
				posts: posts.0,
				query: q,
				sub,
				sort: (sort, param(&path, "t")),
				ends: (param(&path, "after"), posts.1),
			}
			.render()
			.unwrap(),
		),
		Err(msg) => error(msg.to_string()).await,
	}
}
