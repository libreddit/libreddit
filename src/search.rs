// CRATES
use crate::utils::{cookie, error, fetch_posts, param, Post};
use actix_web::{HttpRequest, HttpResponse};
use askama::Template;

// STRUCTS
struct SearchParams {
	q: String,
	sort: String,
	t: String,
	before: String,
	after: String,
	restrict_sr: String,
}

#[derive(Template)]
#[template(path = "search.html", escape = "none")]
struct SearchTemplate {
	posts: Vec<Post>,
	sub: String,
	params: SearchParams,
	layout: String,
}

// SERVICES
pub async fn find(req: HttpRequest) -> HttpResponse {
	let path = format!("{}.json?{}", req.path(), req.query_string());
	let sort = if param(&path, "sort").is_empty() {
		"relevance".to_string()
	} else {
		param(&path, "sort")
	};
	let sub = req.match_info().get("sub").unwrap_or("").to_string();

	match fetch_posts(&path, String::new()).await {
		Ok((posts, after)) => HttpResponse::Ok().content_type("text/html").body(
			SearchTemplate {
				posts,
				sub,
				params: SearchParams {
					q: param(&path, "q"),
					sort,
					t: param(&path, "t"),
					before: param(&path, "after"),
					after,
					restrict_sr: param(&path, "restrict_sr"),
				},
				layout: cookie(req, "layout"),
			}
			.render()
			.unwrap(),
		),
		Err(msg) => error(msg.to_string()).await,
	}
}
