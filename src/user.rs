// CRATES
use crate::client::json;
use crate::esc;
use crate::server::RequestExt;
use crate::utils::{error, format_url, param, template, Post, Preferences, User};
use askama::Template;
use hyper::{Body, Request, Response};
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
pub async fn profile(req: Request<Body>) -> Result<Response<Body>, String> {
	// Build the Reddit JSON API path
	let path = format!(
		"/user/{}.json?{}&raw_json=1",
		req.param("name").unwrap_or_else(|| "reddit".to_string()),
		req.uri().query().unwrap_or_default()
	);

	// Retrieve other variables from Libreddit request
	let sort = param(&path, "sort");
	let username = req.param("name").unwrap_or_default();

	// Request user posts/comments from Reddit
	let posts = Post::fetch(&path, "Comment".to_string()).await;

	match posts {
		Ok((posts, after)) => {
			// If you can get user posts, also request user data
			let user = user(&username).await.unwrap_or_default();

			template(UserTemplate {
				user,
				posts,
				sort: (sort, param(&path, "t")),
				ends: (param(&path, "after"), after),
				prefs: Preferences::new(req),
			})
		}
		// If there is an error show error page
		Err(msg) => error(req, msg).await,
	}
}

// USER
async fn user(name: &str) -> Result<User, String> {
	// Build the Reddit JSON API path
	let path: String = format!("/user/{}/about.json?raw_json=1", name);

	// Send a request to the url
	match json(path).await {
		// If success, receive JSON in response
		Ok(res) => {
			// Grab creation date as unix timestamp
			let created: i64 = res["data"]["created"].as_f64().unwrap_or(0.0).round() as i64;

			// Closure used to parse JSON from Reddit APIs
			let about = |item| res["data"]["subreddit"][item].as_str().unwrap_or_default().to_string();

			// Parse the JSON output into a User struct
			Ok(User {
				name: name.to_string(),
				title: esc!(about("title")),
				icon: format_url(&about("icon_img")),
				karma: res["data"]["total_karma"].as_i64().unwrap_or(0),
				created: OffsetDateTime::from_unix_timestamp(created).format("%b %d '%y"),
				banner: esc!(about("banner_img")),
				description: about("public_description"),
			})
		}
		// If the Reddit API returns an error, exit this function
		Err(msg) => return Err(msg),
	}
}
