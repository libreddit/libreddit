// CRATES
use crate::utils::*;
use actix_web::{cookie::Cookie, HttpRequest, HttpResponse, Result};
use askama::Template;
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
pub async fn page(req: HttpRequest) -> HttpResponse {
	let subscribed = cookie(&req, "subscriptions");
	let front_page = cookie(&req, "front_page");
	let sort = req.match_info().get("sort").unwrap_or("hot").to_string();

	let sub = req
		.match_info()
		.get("sub")
		.map(String::from)
		.unwrap_or(if front_page == "default" || front_page.is_empty() {
			if subscribed.is_empty() {
				"popular".to_string()
			} else {
				subscribed.to_owned()
			}
		} else {
			front_page.to_owned()
		});

	let path = format!("/r/{}/{}.json?{}", sub, sort, req.query_string());

	match fetch_posts(&path, String::new()).await {
		Ok((posts, after)) => {
			// If you can get subreddit posts, also request subreddit metadata
			let sub = if !sub.contains('+') && sub != subscribed && sub != "popular" && sub != "all" {
				// Regular subreddit
				subreddit(&sub).await.unwrap_or_default()
			} else if sub == subscribed {
				// Subscription feed
				if req.path().starts_with("/r/") {
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

			let s = SubredditTemplate {
				sub,
				posts,
				sort: (sort, param(&path, "t")),
				ends: (param(&path, "after"), after),
				prefs: prefs(req),
			}
			.render()
			.unwrap();
			HttpResponse::Ok().content_type("text/html").body(s)
		}
		Err(msg) => error(msg).await,
	}
}

// Sub or unsub by setting subscription cookie using response "Set-Cookie" header
pub async fn subscriptions(req: HttpRequest) -> HttpResponse {
	let mut res = HttpResponse::Found();

	let sub = req.match_info().get("sub").unwrap_or_default().to_string();
	let action = req.match_info().get("action").unwrap_or_default().to_string();
	let mut sub_list = prefs(req.to_owned()).subs;

	// Modify sub list based on action
	if action == "subscribe" && !sub_list.contains(&sub) {
		sub_list.push(sub.to_owned());
		sub_list.sort_by_key(|a| a.to_lowercase());
	} else if action == "unsubscribe" {
		sub_list.retain(|s| s != &sub);
	}

	// Delete cookie if empty, else set
	if sub_list.is_empty() {
		res.del_cookie(&Cookie::build("subscriptions", "").path("/").finish());
	} else {
		res.cookie(
			Cookie::build("subscriptions", sub_list.join("+"))
				.path("/")
				.http_only(true)
				.expires(OffsetDateTime::now_utc() + Duration::weeks(52))
				.finish(),
		);
	}

	// Redirect back to subreddit
	// check for redirect parameter if unsubscribing from outside sidebar
	let redirect_path = param(&req.uri().to_string(), "redirect");
	let path = if !redirect_path.is_empty() && redirect_path.starts_with('/') {
		redirect_path
	} else {
		format!("/r/{}", sub)
	};

	res
		.content_type("text/html")
		.set_header("Location", path.to_owned())
		.body(format!("Redirecting to <a href=\"{0}\">{0}</a>...", path))
}

pub async fn wiki(req: HttpRequest) -> HttpResponse {
	let sub = req.match_info().get("sub").unwrap_or("reddit.com").to_string();
	let page = req.match_info().get("page").unwrap_or("index").to_string();
	let path: String = format!("/r/{}/wiki/{}.json?raw_json=1", sub, page);

	match request(path).await {
		Ok(res) => {
			let s = WikiTemplate {
				sub,
				wiki: rewrite_url(res["data"]["content_html"].as_str().unwrap_or_default()),
				page,
				prefs: prefs(req),
			}
			.render()
			.unwrap();
			HttpResponse::Ok().content_type("text/html").body(s)
		}
		Err(msg) => error(msg).await,
	}
}

// SUBREDDIT
async fn subreddit(sub: &str) -> Result<Subreddit, String> {
	// Build the Reddit JSON API url
	let path: String = format!("/r/{}/about.json?raw_json=1", sub);

	// Send a request to the url
	match request(path).await {
		// If success, receive JSON in response
		Ok(res) => {
			// Metadata regarding the subreddit
			let members: i64 = res["data"]["subscribers"].as_u64().unwrap_or_default() as i64;
			let active: i64 = res["data"]["accounts_active"].as_u64().unwrap_or_default() as i64;

			// Fetch subreddit icon either from the community_icon or icon_img value
			let community_icon: &str = res["data"]["community_icon"].as_str().map_or("", |s| s.split('?').collect::<Vec<&str>>()[0]);
			let icon = if community_icon.is_empty() { val(&res, "icon_img") } else { community_icon.to_string() };

			let sub = Subreddit {
				name: val(&res, "display_name"),
				title: val(&res, "title"),
				description: val(&res, "public_description"),
				info: rewrite_url(&val(&res, "description_html").replace("\\", "")),
				icon: format_url(icon.as_str()),
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
