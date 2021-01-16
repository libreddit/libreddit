// CRATES
use crate::utils::{error, fetch_posts, format_url, param, prefs, request, Post, Preferences, User};
use actix_web::{HttpRequest, HttpResponse, Result};
use askama::Template;
use time::OffsetDateTime;

// STRUCTS
#[derive(Template)]
#[template(path = "user.html", escape = "none")]
struct UserTemplate {
	user: User,
	posts: Vec<Post>,
	sort: (String, String),
	ends: (String, String),
	prefs: Preferences,
}

// FUNCTIONS
pub async fn profile(req: HttpRequest) -> HttpResponse {
	// Build the Reddit JSON API path
	let path = format!("{}.json?{}&raw_json=1", req.path(), req.query_string());

	// Retrieve other variables from Libreddit request
	let sort = param(&path, "sort");
	let username = req.match_info().get("username").unwrap_or("").to_string();

	// Request user posts/comments from Reddit
	let posts = fetch_posts(&path, "Comment".to_string()).await;

	match posts {
		Ok((posts, after)) => {
			// If you can get user posts, also request user data
			let user = user(&username).await.unwrap_or_default();

			let s = UserTemplate {
				user,
				posts,
				sort: (sort, param(&path, "t")),
				ends: (param(&path, "after"), after),
				prefs: prefs(req),
			}
			.render()
			.unwrap();
			HttpResponse::Ok().content_type("text/html").body(s)
		}
		// If there is an error show error page
		Err(msg) => error(msg).await,
	}
}

// USER
async fn user(name: &str) -> Result<User, String> {
	// Build the Reddit JSON API path
	let path: String = format!("/user/{}/about.json", name);

	// Send a request to the url
	match request(&path).await {
		// If success, receive JSON in response
		Ok(res) => {
			// Grab creation date as unix timestamp
			let created: i64 = res["data"]["created"].as_f64().unwrap_or(0.0).round() as i64;

			// nested_val function used to parse JSON from Reddit APIs
			let about = |item| res["data"]["subreddit"][item].as_str().unwrap_or_default().to_string();

			// Parse the JSON output into a User struct
			Ok(User {
				name: name.to_string(),
				title: about("title"),
				icon: format_url(about("icon_img").as_str()),
				karma: res["data"]["total_karma"].as_i64().unwrap_or(0),
				created: OffsetDateTime::from_unix_timestamp(created).format("%b %d '%y"),
				banner: about("banner_img"),
				description: about("public_description"),
			})
		}
		// If the Reddit API returns an error, exit this function
		Err(msg) => return Err(msg),
	}
}
