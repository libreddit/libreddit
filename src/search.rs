// CRATES
use crate::utils::{error, fetch_posts, param, prefs, request, val, Post, Preferences};
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

// STRUCTS
struct Subreddit {
	name: String,
	url: String,
	description: String,
	subscribers: i64,
}

#[derive(Template)]
#[template(path = "search.html", escape = "none")]
struct SearchTemplate {
	posts: Vec<Post>,
	subreddits: Vec<Subreddit>,
	sub: String,
	params: SearchParams,
	prefs: Preferences,
}

// SERVICES
pub async fn find(req: HttpRequest) -> HttpResponse {
	let path = format!("{}.json?{}", req.path(), req.query_string());
	let sub = req.match_info().get("sub").unwrap_or("").to_string();

	let sort = if param(&path, "sort").is_empty() {
		"relevance".to_string()
	} else {
		param(&path, "sort")
	};

	let subreddits = if param(&path, "restrict_sr").is_empty() {
		search_subreddits(param(&path, "q")).await
	} else {
		Vec::new()
	};

	match fetch_posts(&path, String::new()).await {
		Ok((posts, after)) => HttpResponse::Ok().content_type("text/html").body(
			SearchTemplate {
				posts,
				subreddits,
				sub,
				params: SearchParams {
					q: param(&path, "q"),
					sort,
					t: param(&path, "t"),
					before: param(&path, "after"),
					after,
					restrict_sr: param(&path, "restrict_sr"),
				},
				prefs: prefs(req),
			}
			.render()
			.unwrap(),
		),
		Err(msg) => error(msg).await,
	}
}

async fn search_subreddits(q: String) -> Vec<Subreddit> {
	let subreddit_search_path = format!("/subreddits/search.json?q={}&limit=3", q.replace(' ', "+"));

	// Send a request to the url
	match request(&subreddit_search_path).await {
		// If success, receive JSON in response
		Ok(response) => {
			match response["data"]["children"].as_array() {
				// For each subreddit from subreddit list
				Some(list) => list
					.iter()
					.map(|subreddit| Subreddit {
						name: val(subreddit, "display_name_prefixed"),
						url: val(subreddit, "url"),
						description: val(subreddit, "public_description"),
						subscribers: subreddit["data"]["subscribers"].as_u64().unwrap_or_default() as i64,
					})
					.collect::<Vec<Subreddit>>(),
				_ => Vec::new(),
			}
		}
		// If the Reddit API returns an error, exit this function
		_ => Vec::new(),
	}
}
