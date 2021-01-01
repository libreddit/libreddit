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
	let sort = param(&path, "sort").await;
	let username = req.match_info().get("username").unwrap_or("").to_string();

	// Request user profile data and user posts/comments from Reddit
	let user = user(&username).await;
	let posts = fetch_posts(path.clone(), "Comment".to_string()).await;

	// If there is an error show error page
	if user.is_err() || posts.is_err() {
		error(user.err().unwrap().to_string()).await
	} else {
		let posts_unwrapped = posts.unwrap();

		let s = UserTemplate {
			user: user.unwrap(),
			posts: posts_unwrapped.0,
			sort: (sort, param(&path, "t").await),
			ends: (param(&path, "after").await, posts_unwrapped.1),
		}
		.render()
		.unwrap();
		Ok(HttpResponse::Ok().content_type("text/html").body(s))
	}
}

// SERVICES
// pub async fn page(web::Path(username): web::Path<String>, params: web::Query<Params>) -> Result<HttpResponse> {
// 	render(username, params.sort.clone(), params.t.clone(), (params.before.clone(), params.after.clone())).await
// }

// USER
async fn user(name: &String) -> Result<User, &'static str> {
	// Build the Reddit JSON API path
	let path: String = format!("user/{}/about.json", name);

	// Send a request to the url, receive JSON in response
	let req = request(path).await;

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
		title: nested_val(&res, "subreddit", "title").await,
		icon: format_url(nested_val(&res, "subreddit", "icon_img").await).await,
		karma: res["data"]["total_karma"].as_i64().unwrap(),
		created: Utc.timestamp(created, 0).format("%b %e, %Y").to_string(),
		banner: nested_val(&res, "subreddit", "banner_img").await,
		description: nested_val(&res, "subreddit", "public_description").await,
	})
}
