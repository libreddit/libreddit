// CRATES
use crate::utils::{catch_random, error, format_num, format_url, param, setting, template, val, Post, Preferences};
use crate::{
	client::json,
	subreddit::{can_access_quarantine, quarantine},
	RequestExt,
};
use askama::Template;
use hyper::{Body, Request, Response};

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
	icon: String,
	description: String,
	subscribers: (String, String),
}

#[derive(Template)]
#[template(path = "search.html", escape = "none")]
struct SearchTemplate {
	posts: Vec<Post>,
	subreddits: Vec<Subreddit>,
	sub: String,
	params: SearchParams,
	prefs: Preferences,
	url: String,
}

// SERVICES
pub async fn find(req: Request<Body>) -> Result<Response<Body>, String> {
	let nsfw_results = if setting(&req, "show_nsfw") == "on" { "&include_over_18=on" } else { "" };
	let path = format!("{}.json?{}{}", req.uri().path(), req.uri().query().unwrap_or_default(), nsfw_results);
	let sub = req.param("sub").unwrap_or_default();
	let quarantined = can_access_quarantine(&req, &sub);
	// Handle random subreddits
	if let Ok(random) = catch_random(&sub, "/find").await {
		return Ok(random);
	}
	let query = param(&path, "q");

	let sort = if param(&path, "sort").is_empty() {
		"relevance".to_string()
	} else {
		param(&path, "sort")
	};

	let subreddits = if param(&path, "restrict_sr").is_empty() {
		search_subreddits(&query).await
	} else {
		Vec::new()
	};

	let url = String::from(req.uri().path_and_query().map_or("", |val| val.as_str()));

	match Post::fetch(&path, String::new(), quarantined).await {
		Ok((posts, after)) => template(SearchTemplate {
			posts,
			subreddits,
			sub,
			params: SearchParams {
				q: query.replace('"', "&quot;"),
				sort,
				t: param(&path, "t"),
				before: param(&path, "after"),
				after,
				restrict_sr: param(&path, "restrict_sr"),
			},
			prefs: Preferences::new(req),
			url,
		}),
		Err(msg) => {
			if msg == "quarantined" {
				let sub = req.param("sub").unwrap_or_default();
				quarantine(req, sub)
			} else {
				error(req, msg).await
			}
		}
	}
}

async fn search_subreddits(q: &str) -> Vec<Subreddit> {
	let subreddit_search_path = format!("/subreddits/search.json?q={}&limit=3", q.replace(' ', "+"));

	// Send a request to the url
	match json(subreddit_search_path, false).await {
		// If success, receive JSON in response
		Ok(response) => {
			match response["data"]["children"].as_array() {
				// For each subreddit from subreddit list
				Some(list) => list
					.iter()
					.map(|subreddit| {
						// Fetch subreddit icon either from the community_icon or icon_img value
						let community_icon: &str = subreddit["data"]["community_icon"].as_str().map_or("", |s| s.split('?').collect::<Vec<&str>>()[0]);
						let icon = if community_icon.is_empty() {
							val(&subreddit, "icon_img")
						} else {
							community_icon.to_string()
						};

						Subreddit {
							name: val(subreddit, "display_name_prefixed"),
							url: val(subreddit, "url"),
							icon: format_url(&icon),
							description: val(subreddit, "public_description"),
							subscribers: format_num(subreddit["data"]["subscribers"].as_f64().unwrap_or_default() as i64),
						}
					})
					.collect::<Vec<Subreddit>>(),
				_ => Vec::new(),
			}
		}
		// If the Reddit API returns an error, exit this function
		_ => Vec::new(),
	}
}
