// CRATES
use crate::esc;
use crate::utils::{cookie, error, format_num, format_url, param, redirect, rewrite_urls, template, val, Post, Preferences, Subreddit};
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

			template(SubredditTemplate {
				sub,
				posts,
				sort: (sort, param(&path, "t")),
				ends: (param(&path, "after"), after),
				prefs: Preferences::new(req),
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
	let query = req.uri().query().unwrap_or_default().to_string();
	let action: Vec<String> = req.uri().path().split('/').map(String::from).collect();

	let mut sub_list = Preferences::new(req).subscriptions;

	// Find each subreddit name (separated by '+') in sub parameter
	for part in sub.split('+') {
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
	let page = req.param("page").unwrap_or_else(|| "index".to_string());
	let path: String = format!("/r/{}/wiki/{}.json?raw_json=1", sub, page);

	match json(path).await {
		Ok(response) => template(WikiTemplate {
			sub,
			wiki: rewrite_urls(response["data"]["content_html"].as_str().unwrap_or_default()),
			page,
			prefs: Preferences::new(req),
		}),
		Err(msg) => error(req, msg).await,
	}
}

pub async fn sidebar(req: Request<Body>) -> Result<Response<Body>, String> {
	let sub = req.param("sub").unwrap_or_else(|| "reddit.com".to_string());

	// Build the Reddit JSON API url
	let path: String = format!("/r/{}/about.json?raw_json=1", sub);

	// Send a request to the url
	match json(path).await {
		// If success, receive JSON in response
		Ok(response) => template(WikiTemplate {
			sub,
			wiki: rewrite_urls(&val(&response, "description_html").replace("\\", "")),
			page: "Sidebar".to_string(),
			prefs: Preferences::new(req),
		}),
		Err(msg) => error(req, msg).await,
	}
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
			let community_icon: &str = res["data"]["community_icon"].as_str().map_or("", |s| s.split('?').collect::<Vec<&str>>()[0]);
			let icon = if community_icon.is_empty() { val(&res, "icon_img") } else { community_icon.to_string() };

			let sub = Subreddit {
				name: esc!(&res, "display_name"),
				title: esc!(&res, "title"),
				description: esc!(&res, "public_description"),
				info: rewrite_urls(&val(&res, "description_html").replace("\\", "")),
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
