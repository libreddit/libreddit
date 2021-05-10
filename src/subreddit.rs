// CRATES
use crate::esc;
use crate::utils::{catch_random, cookie, error, format_num, format_url, param, redirect, rewrite_urls, template, val, Post, Preferences, Subreddit};
use crate::{client::json, server::ResponseExt, RequestExt};
use askama::Template;
use cookie::Cookie;
use hyper::{Body, Request, Response};
use time::{Duration, OffsetDateTime};

// STRUCTS
#[derive(Template)]
#[template(path = "subreddit.html", escape = "none")]
struct SubredditTemplate {
	sub: Subreddit,
	posts: Vec<Post>,
	sort: (String, String),
	ends: (String, String),
	prefs: Preferences,
	url: String,
}

#[derive(Template)]
#[template(path = "wiki.html", escape = "none")]
struct WikiTemplate {
	sub: String,
	wiki: String,
	page: String,
	prefs: Preferences,
}

// SERVICES
pub async fn community(req: Request<Body>) -> Result<Response<Body>, String> {
	// Build Reddit API path
	let subscribed = cookie(&req, "subscriptions");
	let front_page = cookie(&req, "front_page");
	let post_sort = req.cookie("post_sort").map_or_else(|| "hot".to_string(), |c| c.value().to_string());
	let sort = req.param("sort").unwrap_or_else(|| req.param("id").unwrap_or(post_sort));

	let sub = req.param("sub").unwrap_or(if front_page == "default" || front_page.is_empty() {
		if subscribed.is_empty() {
			"popular".to_string()
		} else {
			subscribed.to_owned()
		}
	} else {
		front_page.to_owned()
	});

	// Handle random subreddits
	if let Ok(random) = catch_random(&sub, "").await {
		return Ok(random);
	}

	if req.param("sub").is_some() && sub.starts_with("u_") {
		return Ok(redirect(["/user/", &sub[2..]].concat()));
	}

	let path = format!("/r/{}/{}.json?{}&raw_json=1", sub, sort, req.uri().query().unwrap_or_default());

	match Post::fetch(&path, String::new()).await {
		Ok((posts, after)) => {
			// If you can get subreddit posts, also request subreddit metadata
			let sub = if !sub.contains('+') && sub != subscribed && sub != "popular" && sub != "all" {
				// Regular subreddit
				subreddit(&sub).await.unwrap_or_default()
			} else if sub == subscribed {
				// Subscription feed
				if req.uri().path().starts_with("/r/") {
					subreddit(&sub).await.unwrap_or_default()
				} else {
					Subreddit::default()
				}
			} else if sub.contains('+') {
				// Multireddit
				Subreddit {
					name: sub,
					..Subreddit::default()
				}
			} else {
				Subreddit::default()
			};

			let url = String::from(req.uri().path_and_query().map_or("", |val| val.as_str()));

			template(SubredditTemplate {
				sub,
				posts,
				sort: (sort, param(&path, "t")),
				ends: (param(&path, "after"), after),
				prefs: Preferences::new(req),
				url,
			})
		}
		Err(msg) => match msg.as_str() {
			"quarantined" => error(req, format!("r/{} has been quarantined by Reddit", sub)).await,
			"private" => error(req, format!("r/{} is a private community", sub)).await,
			"banned" => error(req, format!("r/{} has been banned from Reddit", sub)).await,
			_ => error(req, msg).await,
		},
	}
}

// Sub or unsub by setting subscription cookie using response "Set-Cookie" header
pub async fn subscriptions(req: Request<Body>) -> Result<Response<Body>, String> {
	let sub = req.param("sub").unwrap_or_default();
	// Handle random subreddits
	if sub == "random" || sub == "randnsfw" {
		return Err("Can't subscribe to random subreddit!".to_string());
	}

	let query = req.uri().query().unwrap_or_default().to_string();
	let action: Vec<String> = req.uri().path().split('/').map(String::from).collect();

	let mut sub_list = Preferences::new(req).subscriptions;

	// Retrieve list of posts for these subreddits to extract display names
	let display = json(format!("/r/{}/hot.json?raw_json=1", sub)).await?;
	let display_lookup: Vec<(String, &str)> = display["data"]["children"]
		.as_array()
		.unwrap()
		.iter()
		.map(|post| {
			let display_name = post["data"]["subreddit"].as_str().unwrap();
			(display_name.to_lowercase(), display_name)
		})
		.collect();

	// Find each subreddit name (separated by '+') in sub parameter
	for part in sub.split('+') {
		// Retrieve display name for the subreddit
		let display;
		let part = if let Some(&(_, display)) = display_lookup.iter().find(|x| x.0 == part.to_lowercase()) {
			// This is already known, doesn't require seperate request
			display
		} else {
			// This subreddit display name isn't known, retrieve it
			let path: String = format!("/r/{}/about.json?raw_json=1", part);
			display = json(path).await?;
			display["data"]["display_name"].as_str().ok_or_else(|| "Failed to query subreddit name".to_string())?
		};

		// Modify sub list based on action
		if action.contains(&"subscribe".to_string()) && !sub_list.contains(&part.to_owned()) {
			// Add each sub name to the subscribed list
			sub_list.push(part.to_owned());
			// Reorder sub names alphabettically
			sub_list.sort_by_key(|a| a.to_lowercase())
		} else if action.contains(&"unsubscribe".to_string()) {
			// Remove sub name from subscribed list
			sub_list.retain(|s| s != part);
		}
	}

	// Redirect back to subreddit
	// check for redirect parameter if unsubscribing from outside sidebar
	let redirect_path = param(&format!("/?{}", query), "redirect");
	let path = if redirect_path.is_empty() {
		format!("/r/{}", sub)
	} else {
		format!("/{}/", redirect_path)
	};

	let mut res = redirect(path);

	// Delete cookie if empty, else set
	if sub_list.is_empty() {
		res.remove_cookie("subscriptions".to_string());
	} else {
		res.insert_cookie(
			Cookie::build("subscriptions", sub_list.join("+"))
				.path("/")
				.http_only(true)
				.expires(OffsetDateTime::now_utc() + Duration::weeks(52))
				.finish(),
		);
	}

	Ok(res)
}

pub async fn wiki(req: Request<Body>) -> Result<Response<Body>, String> {
	let sub = req.param("sub").unwrap_or_else(|| "reddit.com".to_string());
	// Handle random subreddits
	if let Ok(random) = catch_random(&sub, "/wiki").await {
		return Ok(random);
	}

	let page = req.param("page").unwrap_or_else(|| "index".to_string());
	let path: String = format!("/r/{}/wiki/{}.json?raw_json=1", sub, page);

	match json(path).await {
		Ok(response) => template(WikiTemplate {
			sub,
			wiki: rewrite_urls(response["data"]["content_html"].as_str().unwrap_or("<h3>Wiki not found</h3>")),
			page,
			prefs: Preferences::new(req),
		}),
		Err(msg) => error(req, msg).await,
	}
}

pub async fn sidebar(req: Request<Body>) -> Result<Response<Body>, String> {
	let sub = req.param("sub").unwrap_or_else(|| "reddit.com".to_string());
	// Handle random subreddits
	if let Ok(random) = catch_random(&sub, "/about/sidebar").await {
		return Ok(random);
	}

	// Build the Reddit JSON API url
	let path: String = format!("/r/{}/about.json?raw_json=1", sub);

	// Send a request to the url
	match json(path).await {
		// If success, receive JSON in response
		Ok(response) => template(WikiTemplate {
			wiki: format!(
				"{}<hr><h1>Moderators</h1><br><ul>{}</ul>",
				rewrite_urls(&val(&response, "description_html").replace("\\", "")),
				moderators(&sub).await?.join(""),
			),
			sub,
			page: "Sidebar".to_string(),
			prefs: Preferences::new(req),
		}),
		Err(msg) => error(req, msg).await,
	}
}

pub async fn moderators(sub: &str) -> Result<Vec<String>, String> {
	// Retrieve and format the html for the moderators list
	Ok(
		moderators_list(sub)
			.await?
			.iter()
			.map(|m| format!("<li><a style=\"color: var(--accent)\" href=\"/u/{name}\">{name}</a></li>", name = m))
			.collect(),
	)
}

async fn moderators_list(sub: &str) -> Result<Vec<String>, String> {
	// Build the moderator list URL
	let path: String = format!("/r/{}/about/moderators.json?raw_json=1", sub);

	// Retrieve response
	let response = json(path).await?["data"]["children"].clone();
	Ok(if let Some(response) = response.as_array() {
		// Traverse json tree and format into list of strings
		response
			.iter()
			.map(|m| m["name"].as_str().unwrap_or(""))
			.filter(|m| !m.is_empty())
			.map(std::string::ToString::to_string)
			.collect::<Vec<_>>()
	} else {
		vec![]
	})
}

// SUBREDDIT
async fn subreddit(sub: &str) -> Result<Subreddit, String> {
	// Build the Reddit JSON API url
	let path: String = format!("/r/{}/about.json?raw_json=1", sub);

	// Send a request to the url
	match json(path).await {
		// If success, receive JSON in response
		Ok(res) => {
			// Metadata regarding the subreddit
			let members: i64 = res["data"]["subscribers"].as_u64().unwrap_or_default() as i64;
			let active: i64 = res["data"]["accounts_active"].as_u64().unwrap_or_default() as i64;

			// Fetch subreddit icon either from the community_icon or icon_img value
			let community_icon: &str = res["data"]["community_icon"].as_str().unwrap();
			let icon = if community_icon.is_empty() { val(&res, "icon_img") } else { community_icon.to_string() };

			let sub = Subreddit {
				name: esc!(&res, "display_name"),
				title: esc!(&res, "title"),
				description: esc!(&res, "public_description"),
				info: rewrite_urls(&val(&res, "description_html").replace("\\", "")),
				moderators: moderators_list(sub).await?,
				icon: format_url(&icon),
				members: format_num(members),
				active: format_num(active),
				wiki: res["data"]["wiki_enabled"].as_bool().unwrap_or_default(),
			};

			Ok(sub)
		}
		// If the Reddit API returns an error, exit this function
		Err(msg) => return Err(msg),
	}
}
