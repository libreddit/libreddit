// CRATES
use crate::utils::{catch_random, error, format_num, format_url, param, redirect, setting, template, val, Post, Preferences};
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
	let query = param(&path, "q").unwrap_or_default();

	if query.is_empty() {
		return Ok(redirect("/".to_string()));
	}

	let sub = req.param("sub").unwrap_or_default();
	let quarantined = can_access_quarantine(&req, &sub);
	// Handle random subreddits
	if let Ok(random) = catch_random(&sub, "/find").await {
		return Ok(random);
	}

	let sort = param(&path, "sort").unwrap_or_else(|| "relevance".to_string());

	// If search is not restricted to this subreddit, show other subreddits in search results
	let subreddits = param(&path, "restrict_sr").map_or(search_subreddits(&query).await, |_| Vec::new());

	let url = String::from(req.uri().path_and_query().map_or("", |val| val.as_str()));

	match Post::fetch(&path, String::new(), quarantined).await {
		Ok((posts, after)) => template(SearchTemplate {
			posts,
			subreddits,
			sub,
			params: SearchParams {
				q: query.replace('"', "&quot;"),
				sort,
				t: param(&path, "t").unwrap_or_default(),
				before: param(&path, "after").unwrap_or_default(),
				after,
				restrict_sr: param(&path, "restrict_sr").unwrap_or_default(),
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
	json(subreddit_search_path, false).await.unwrap_or_default()["data"]["children"]
		.as_array()
		.map(ToOwned::to_owned)
		.unwrap_or_default()
		.iter()
		.map(|subreddit| {
			// For each subreddit from subreddit list
			// Fetch subreddit icon either from the community_icon or icon_img value
			let icon = subreddit["data"]["community_icon"]
				.as_str()
				.map_or_else(|| val(subreddit, "icon_img"), ToString::to_string);

			Subreddit {
				name: val(subreddit, "display_name_prefixed"),
				url: val(subreddit, "url"),
				icon: format_url(&icon),
				description: val(subreddit, "public_description"),
				subscribers: format_num(subreddit["data"]["subscribers"].as_f64().unwrap_or_default() as i64),
			}
		})
		.collect::<Vec<Subreddit>>()
}
