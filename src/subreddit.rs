// CRATES
use crate::utils::*;
use actix_web::{HttpRequest, HttpResponse, Result};
use askama::Template;

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
	let path = format!("{}.json?{}", req.path(), req.query_string());
	let default = cookie(&req, "front_page");
	let sub_name = req
		.match_info()
		.get("sub")
		.unwrap_or(if default.is_empty() { "popular" } else { default.as_str() })
		.to_string();
	let sort = req.match_info().get("sort").unwrap_or("hot").to_string();

	match fetch_posts(&path, String::new()).await {
		Ok((posts, after)) => {
			// If you can get subreddit posts, also request subreddit metadata
			let sub = if !sub_name.contains('+') && sub_name != "popular" && sub_name != "all" {
				subreddit(&sub_name).await.unwrap_or_default()
			} else if sub_name.contains('+') {
				Subreddit {
					name: sub_name,
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

pub async fn wiki(req: HttpRequest) -> HttpResponse {
	let sub = req.match_info().get("sub").unwrap_or("reddit.com").to_string();
	let page = req.match_info().get("page").unwrap_or("index").to_string();
	let path: String = format!("/r/{}/wiki/{}.json?raw_json=1", sub, page);

	match request(&path).await {
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
	match request(&path).await {
		// If success, receive JSON in response
		Ok(res) => {
			// Metadata regarding the subreddit
			let members: i64 = res["data"]["subscribers"].as_u64().unwrap_or_default() as i64;
			let active: i64 = res["data"]["accounts_active"].as_u64().unwrap_or_default() as i64;

			// Fetch subreddit icon either from the community_icon or icon_img value
			let community_icon: &str = res["data"]["community_icon"].as_str().unwrap_or("").split('?').collect::<Vec<&str>>()[0];
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
