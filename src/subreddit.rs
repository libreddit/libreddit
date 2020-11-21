// CRATES
use actix_web::{get, web, HttpResponse, Result};
use askama::Template;

#[path = "utils.rs"]
mod utils;
pub use utils::{request, val, fetch_posts, ErrorTemplate, Flair, Params, Post, Subreddit};

// STRUCTS
#[derive(Template)]
#[template(path = "subreddit.html", escape = "none")]
struct SubredditTemplate {
	sub: Subreddit,
	posts: Vec<Post>,
	sort: String,
	ends: (String, String),
}

// SERVICES
#[allow(dead_code)]
#[get("/r/{sub}")]
async fn page(web::Path(sub): web::Path<String>, params: web::Query<Params>) -> Result<HttpResponse> {
	render(sub, params.sort.clone(), (params.before.clone(), params.after.clone())).await
}

pub async fn render(sub_name: String, sort: Option<String>, ends: (Option<String>, Option<String>)) -> Result<HttpResponse> {
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

	let sub_result = subreddit(&sub_name).await;
	let items_result = fetch_posts(url, String::new()).await;

	if sub_result.is_err() || items_result.is_err() {
		let s = ErrorTemplate {
			message: sub_result.err().unwrap().to_string(),
		}
		.render()
		.unwrap();
		Ok(HttpResponse::Ok().status(actix_web::http::StatusCode::NOT_FOUND).content_type("text/html").body(s))
	} else {
		let mut sub = sub_result.unwrap();
		let items = items_result.unwrap();

		sub.icon = if sub.icon != "" {
			format!(r#"<img class="subreddit_icon" src="{}">"#, sub.icon)
		} else {
			String::new()
		};

		let s = SubredditTemplate {
			sub: sub,
			posts: items.0,
			sort: sorting,
			ends: (before, items.1),
		}
		.render()
		.unwrap();
		Ok(HttpResponse::Ok().content_type("text/html").body(s))
	}
}

// SUBREDDIT
async fn subreddit(sub: &String) -> Result<Subreddit, &'static str> {
	// Build the Reddit JSON API url
	let url: String = format!("https://www.reddit.com/r/{}/about.json", sub);

	// Send a request to the url, receive JSON in response
	let req = request(url).await;

	// If the Reddit API returns an error, exit this function
	if req.is_err() {
		return Err(req.err().unwrap());
	}

	// Otherwise, grab the JSON output from the request
	let res = req.unwrap();

	let icon: String = String::from(res["data"]["community_icon"].as_str().unwrap()); //val(&data, "community_icon");
	let icon_split: std::str::Split<&str> = icon.split("?");
	let icon_parts: Vec<&str> = icon_split.collect();

	let sub = Subreddit {
		name: val(&res, "display_name").await,
		title: val(&res, "title").await,
		description: val(&res, "public_description").await,
		icon: String::from(icon_parts[0]),
	};

	Ok(sub)
}