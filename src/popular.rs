// CRATES
use crate::utils::{fetch_posts, ErrorTemplate, Params, Post};
use actix_web::{http::StatusCode, web, HttpResponse, Result};
use askama::Template;

// STRUCTS
#[derive(Template)]
#[template(path = "popular.html", escape = "none")]
struct PopularTemplate {
	posts: Vec<Post>,
	sort: (String, String),
	ends: (String, String),
}

// RENDER
async fn render(sort: Option<String>, t: Option<String>, ends: (Option<String>, Option<String>)) -> Result<HttpResponse> {
	let sorting = sort.unwrap_or("hot".to_string());
	let before = ends.1.clone().unwrap_or(String::new()); // If there is an after, there must be a before

	let timeframe = match &t { Some(val) => format!("&t={}", val), None => String::new() };

	// Build the Reddit JSON API url
	let url = match ends.0 {
		Some(val) => format!("r/popular/{}.json?before={}&count=25{}", sorting, val, timeframe),
		None => match ends.1 {
			Some(val) => format!("r/popular/{}.json?after={}&count=25{}", sorting, val, timeframe),
			None => format!("r/popular/{}.json?{}", sorting, timeframe),
		},
	};

	let items_result = fetch_posts(url, String::new()).await;

	if items_result.is_err() {
		let s = ErrorTemplate {
			message: items_result.err().unwrap().to_string(),
		}
		.render()
		.unwrap();
		Ok(HttpResponse::Ok().status(StatusCode::NOT_FOUND).content_type("text/html").body(s))
	} else {
		let items = items_result.unwrap();

		let s = PopularTemplate {
			posts: items.0,
			sort: (sorting, t.unwrap_or(String::new())),
			ends: (before, items.1),
		}
		.render()
		.unwrap();
		Ok(HttpResponse::Ok().content_type("text/html").body(s))
	}
}

// SERVICES
pub async fn page(params: web::Query<Params>) -> Result<HttpResponse> {
	render(params.sort.clone(), params.t.clone(), (params.before.clone(), params.after.clone())).await
}
