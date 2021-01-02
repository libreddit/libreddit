// CRATES
use crate::utils::{error, fetch_posts, format_url, nested_val, param, request, Post, User};
use actix_web::{HttpRequest, HttpResponse, Result};
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

pub async fn profile(req: HttpRequest) -> Result<HttpResponse> {
	// Build the Reddit JSON API path
	let path = format!("{}.json?{}&raw_json=1", req.path(), req.query_string());

	// Retrieve other variables from Libreddit request
	let sort = param(&path, "sort");
	let username = req.match_info().get("username").unwrap_or("").to_string();

	// Request user profile data and user posts/comments from Reddit
	let user = user(&username).await;
	let posts = fetch_posts(&path, "Comment".to_string()).await;

	match posts {
		Ok(items) => {
			let s = UserTemplate {
				user: user.unwrap(),
				posts: items.0,
				sort: (sort, param(&path, "t")),
				ends: (param(&path, "after"), items.1),
			}
			.render()
			.unwrap();
			Ok(HttpResponse::Ok().content_type("text/html").body(s))
		}
		// If there is an error show error page
		Err(msg) => error(msg.to_string()).await,
	}
}

// USER
async fn user(name: &str) -> Result<User, &'static str> {
	// Build the Reddit JSON API path
	let path: String = format!("user/{}/about.json", name);

	let res;

	// Send a request to the url
	match request(&path).await {
		// If success, receive JSON in response
		Ok(response) => {
			res = response;
		}
		// If the Reddit API returns an error, exit this function
		Err(msg) => return Err(msg),
	}

	// Grab creation date as unix timestamp
	let created: i64 = res["data"]["created"].as_f64().unwrap().round() as i64;

	// Parse the JSON output into a User struct
	Ok(User {
		name: name.to_string(),
		title: nested_val(&res, "subreddit", "title"),
		icon: format_url(nested_val(&res, "subreddit", "icon_img")),
		karma: res["data"]["total_karma"].as_i64().unwrap(),
		created: Utc.timestamp(created, 0).format("%b %e, %Y").to_string(),
		banner: nested_val(&res, "subreddit", "banner_img"),
		description: nested_val(&res, "subreddit", "public_description"),
	})
}
