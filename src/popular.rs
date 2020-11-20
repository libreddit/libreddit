// CRATES
use actix_web::{get, web, HttpResponse, Result};
use askama::Template;

#[path = "subreddit.rs"]
mod subreddit;
use subreddit::{posts, Post};

#[path = "utils.rs"]
mod utils;
use utils::{ErrorTemplate, Params};

// STRUCTS
#[derive(Template)]
#[template(path = "popular.html", escape = "none")]
struct PopularTemplate {
	posts: Vec<Post>,
	sort: String,
	ends: (String, String),
}

// RENDER
async fn render(sub_name: String, sort: Option<String>, ends: (Option<String>, Option<String>)) -> Result<HttpResponse> {
	let sorting = sort.unwrap_or("hot".to_string());
	let before = ends.1.clone().unwrap_or(String::new()); // If there is an after, there must be a before

	// Build the Reddit JSON API url
	let url = match ends.0 {
		Some(val) => format!("https://www.reddit.com/r/{}/{}.json?before={}&count=25", sub_name, sorting, val),
		None => match ends.1 {
			Some(val) => format!("https://www.reddit.com/r/{}/{}.json?after={}&count=25", sub_name, sorting, val),
			None => format!("https://www.reddit.com/r/{}/{}.json", sub_name, sorting),
		},
	};

	let items_result = posts(url).await;

	if items_result.is_err() {
		let s = ErrorTemplate {
			message: items_result.err().unwrap().to_string(),
		}
		.render()
		.unwrap();
		Ok(HttpResponse::Ok().content_type("text/html").body(s))
	} else {
		let items = items_result.unwrap();

		let s = PopularTemplate {
			posts: items.0,
			sort: sorting,
			ends: (before, items.1),
		}
		.render()
		.unwrap();
		Ok(HttpResponse::Ok().content_type("text/html").body(s))
	}
}

// SERVICES
#[get("/")]
pub async fn page(params: web::Query<Params>) -> Result<HttpResponse> {
	render("popular".to_string(), params.sort.clone(), (params.before.clone(), params.after.clone())).await
}
