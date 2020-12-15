// CRATES
use crate::utils::{fetch_posts, nested_val, request, ErrorTemplate, Params, Post, User};
use actix_web::{http::StatusCode, web, HttpResponse, Result};
use askama::Template;

// STRUCTS
#[derive(Template)]
#[template(path = "user.html", escape = "none")]
struct UserTemplate {
	user: User,
	posts: Vec<Post>,
	sort: String,
}

async fn render(username: String, sort: String) -> Result<HttpResponse> {
	// Build the Reddit JSON API url
	let url: String = format!("https://www.reddit.com/user/{}/.json?sort={}", username, sort);

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
		let s = UserTemplate {
			user: user.unwrap(),
			posts: posts.unwrap().0,
			sort: sort,
		}
		.render()
		.unwrap();
		Ok(HttpResponse::Ok().content_type("text/html").body(s))
	}
}

// SERVICES
pub async fn page(web::Path(username): web::Path<String>, params: web::Query<Params>) -> Result<HttpResponse> {
	match &params.sort {
		Some(sort) => render(username, sort.to_string()).await,
		None => render(username, "hot".to_string()).await,
	}
}

// USER
async fn user(name: &String) -> Result<User, &'static str> {
	// Build the Reddit JSON API url
	let url: String = format!("https://www.reddit.com/user/{}/about.json", name);

	// Send a request to the url, receive JSON in response
	let req = request(url).await;

	// If the Reddit API returns an error, exit this function
	if req.is_err() {
		return Err(req.err().unwrap());
	}

	// Otherwise, grab the JSON output from the request
	let res = req.unwrap();

	// Parse the JSON output into a User struct
	Ok(User {
		name: name.to_string(),
		icon: nested_val(&res, "subreddit", "icon_img").await,
		karma: res["data"]["total_karma"].as_i64().unwrap(),
		banner: nested_val(&res, "subreddit", "banner_img").await,
		description: nested_val(&res, "subreddit", "public_description").await,
	})
}
