// CRATES
use crate::utils::{catch_random, error, filter_posts, format_num, format_url, get_filters, param, redirect, setting, template, val, Post, Preferences};
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
	typed: String,
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
#[template(path = "search.html")]
struct SearchTemplate {
	posts: Vec<Post>,
	subreddits: Vec<Subreddit>,
	sub: String,
	params: SearchParams,
	prefs: Preferences,
	url: String,
	/// Whether the subreddit itself is filtered.
	is_filtered: bool,
	/// Whether all fetched posts are filtered (to differentiate between no posts fetched in the first place,
	/// and all fetched posts being filtered).
	all_posts_filtered: bool,
	/// Whether all posts were hidden because they are NSFW (and user has disabled show NSFW)
	all_posts_hidden_nsfw: bool,
}

// SERVICES
pub async fn find(req: Request<Body>) -> Result<Response<Body>, String> {
	let nsfw_results = if setting(&req, "show_nsfw") == "on" { "&include_over_18=on" } else { "" };
	let path = format!("{}.json?{}{}&raw_json=1", req.uri().path(), req.uri().query().unwrap_or_default(), nsfw_results);
	let query = param(&path, "q").unwrap_or_default();

	if query.is_empty() {
		return Ok(redirect("/".to_string()));
	}

	if query.starts_with("r/") {
		return Ok(redirect(format!("/{}", query)));
	}

	let sub = req.param("sub").unwrap_or_default();
	let quarantined = can_access_quarantine(&req, &sub);
	// Handle random subreddits
	if let Ok(random) = catch_random(&sub, "/find").await {
		return Ok(random);
	}

	let typed = param(&path, "type").unwrap_or_default();

	let sort = param(&path, "sort").unwrap_or_else(|| "relevance".to_string());
	let filters = get_filters(&req);

	// If search is not restricted to this subreddit, show other subreddits in search results
	let subreddits = if param(&path, "restrict_sr").is_none() {
		let mut subreddits = search_subreddits(&query, &typed).await;
		subreddits.retain(|s| !filters.contains(s.name.as_str()));
		subreddits
	} else {
		Vec::new()
	};

	let url = String::from(req.uri().path_and_query().map_or("", |val| val.as_str()));

	// If all requested subs are filtered, we don't need to fetch posts.
	if sub.split('+').all(|s| filters.contains(s)) {
		template(SearchTemplate {
			posts: Vec::new(),
			subreddits,
			sub,
			params: SearchParams {
				q: query.replace('"', "&quot;"),
				sort,
				t: param(&path, "t").unwrap_or_default(),
				before: param(&path, "after").unwrap_or_default(),
				after: "".to_string(),
				restrict_sr: param(&path, "restrict_sr").unwrap_or_default(),
				typed,
			},
			prefs: Preferences::new(req),
			url,
			is_filtered: true,
			all_posts_filtered: false,
			all_posts_hidden_nsfw: false,
		})
	} else {
		match Post::fetch(&path, quarantined).await {
			Ok((mut posts, after)) => {
				let all_posts_filtered = filter_posts(&mut posts, &filters);
				let all_posts_hidden_nsfw = posts.iter().all(|p| p.flags.nsfw) && setting(&req, "show_nsfw") != "on";
				template(SearchTemplate {
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
						typed,
					},
					prefs: Preferences::new(req),
					url,
					is_filtered: false,
					all_posts_filtered,
					all_posts_hidden_nsfw,
				})
			}
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
}

async fn search_subreddits(q: &str, typed: &str) -> Vec<Subreddit> {
	let limit = if typed == "sr_user" { "50" } else { "3" };
	let subreddit_search_path = format!("/subreddits/search.json?q={}&limit={}", q.replace(' ', "+"), limit);

	// Send a request to the url
	json(subreddit_search_path, false).await.unwrap_or_default()["data"]["children"]
		.as_array()
		.map(ToOwned::to_owned)
		.unwrap_or_default()
		.iter()
		.map(|subreddit| {
			// For each subreddit from subreddit list
			// Fetch subreddit icon either from the community_icon or icon_img value
			let icon = subreddit["data"]["community_icon"].as_str().map_or_else(|| val(subreddit, "icon_img"), ToString::to_string);

			Subreddit {
				name: val(subreddit, "display_name"),
				url: val(subreddit, "url"),
				icon: format_url(&icon),
				description: val(subreddit, "public_description"),
				subscribers: format_num(subreddit["data"]["subscribers"].as_f64().unwrap_or_default() as i64),
			}
		})
		.collect::<Vec<Subreddit>>()
}
