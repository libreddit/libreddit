// CRATES
use crate::utils::{fetch_posts, format_url, nested_val, request, ErrorTemplate, Params, Post, User};
use actix_web::{http::StatusCode, web, HttpResponse, Result};
use askama::Template;
use chrono::{TimeZone, Utc};

// STRUCTS
#[derive(Template)]
#[template(path = "user.html", escape = "none")]
struct UserTemplate {
	user: User,
	posts: Vec<Post>,
	sort: (String, String),
	ends: (String, String),
}

async fn render(username: String, sort: Option<String>, t: Option<String>, ends: (Option<String>, Option<String>)) -> Result<HttpResponse> {
	let sorting = sort.unwrap_or("new".to_string());

	let before = ends.1.clone().unwrap_or(String::new()); // If there is an after, there must be a before

	let timeframe = match &t { Some(val) => format!("&t={}", val), None => String::new() };

	// Build the Reddit JSON API url
	let url = match ends.0 {
		Some(val) => format!("user/{}/.json?sort={}&before={}&count=25&raw_json=1{}", username, sorting, val, timeframe),
		None => match ends.1 {
			Some(val) => format!("user/{}/.json?sort={}&after={}&count=25&raw_json=1{}", username, sorting, val, timeframe),
			None => format!("user/{}/.json?sort={}&raw_json=1{}", username, sorting, timeframe),
		},
	};

	let user = user(&username).await;
	let posts = fetch_posts(url, "Comment".to_string()).await;

	if user.is_err() || posts.is_err() {
		let s = ErrorTemplate {
			message: user.err().unwrap().to_string(),
		}
		.render()
		.unwrap();
		Ok(HttpResponse::Ok().status(StatusCode::NOT_FOUND).content_type("text/html").body(s))
	} else {
		let posts_unwrapped = posts.unwrap();
		
		let s = UserTemplate {
			user: user.unwrap(),
			posts: posts_unwrapped.0,
			sort: (sorting, t.unwrap_or(String::new())),
			ends: (before, posts_unwrapped.1)
		}
		.render()
		.unwrap();
		Ok(HttpResponse::Ok().content_type("text/html").body(s))
	}
}

// SERVICES
pub async fn page(web::Path(username): web::Path<String>, params: web::Query<Params>) -> Result<HttpResponse> {
	render(username, params.sort.clone(), params.t.clone(), (params.before.clone(), params.after.clone())).await
}

// USER
async fn user(name: &String) -> Result<User, &'static str> {
	// Build the Reddit JSON API url
	let url: String = format!("user/{}/about.json", name);

	// Send a request to the url, receive JSON in response
	let req = request(url).await;

	// If the Reddit API returns an error, exit this function
	if req.is_err() {
		return Err(req.err().unwrap());
	}

	// Otherwise, grab the JSON output from the request
	let res = req.unwrap();

	// Grab creation date as unix timestamp
	let created: i64 = res["data"]["created"].as_f64().unwrap().round() as i64;

	// Parse the JSON output into a User struct
	Ok(User {
		name: name.to_string(),
		icon: format_url(nested_val(&res, "subreddit", "icon_img").await).await,
		karma: res["data"]["total_karma"].as_i64().unwrap(),
		created: Utc.timestamp(created, 0).format("%b %e, %Y").to_string(),
		banner: nested_val(&res, "subreddit", "banner_img").await,
		description: nested_val(&res, "subreddit", "public_description").await,
	})
}
