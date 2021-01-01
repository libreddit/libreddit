// CRATES
use crate::utils::{error, fetch_posts, format_num, format_url, param, request, val, Post, Subreddit};
use actix_web::{HttpRequest, HttpResponse, Result};
use askama::Template;
use std::convert::TryInto;

// STRUCTS
#[derive(Template)]
#[template(path = "subreddit.html", escape = "none")]
struct SubredditTemplate {
	sub: Subreddit,
	posts: Vec<Post>,
	sort: (String, String),
	ends: (String, String),
}

// SERVICES
// web::Path(sub): web::Path<String>, params: web::Query<Params>
pub async fn page(req: HttpRequest) -> Result<HttpResponse> {
	let path = format!("{}.json?{}", req.path(), req.query_string());
	let sub = req.match_info().get("sub").unwrap_or("popular").to_string();
	let sort = req.match_info().get("sort").unwrap_or("hot").to_string();

	let sub_result = if !&sub.contains("+") && sub != "popular" {
		subreddit(&sub).await
	} else {
		Ok(Subreddit::default())
	};
	let posts = fetch_posts(path.clone(), String::new()).await;

	if posts.is_err() {
		error(posts.err().unwrap().to_string()).await
	} else {
		let sub = sub_result.unwrap_or(Subreddit::default());
		let items = posts.unwrap();

		let s = SubredditTemplate {
			sub: sub,
			posts: items.0,
			sort: (sort, param(&path, "t").await),
			ends: (param(&path, "after").await, items.1),
		}
		.render()
		.unwrap();
		Ok(HttpResponse::Ok().content_type("text/html").body(s))
	}
}

// SUBREDDIT
async fn subreddit(sub: &String) -> Result<Subreddit, &'static str> {
	// Build the Reddit JSON API url
	let url: String = format!("r/{}/about.json?raw_json=1", sub);

	// Send a request to the url, receive JSON in response
	let req = request(url).await;

	// If the Reddit API returns an error, exit this function
	if req.is_err() {
		return Err(req.err().unwrap());
	}

	// Otherwise, grab the JSON output from the request
	let res = req.unwrap();

	// Metadata regarding the subreddit
	let members = res["data"]["subscribers"].as_u64().unwrap_or(0);
	let active = res["data"]["accounts_active"].as_u64().unwrap_or(0);

	// Fetch subreddit icon either from the community_icon or icon_img value
	let community_icon: &str = res["data"]["community_icon"].as_str().unwrap_or("").split("?").collect::<Vec<&str>>()[0];
	let icon = if community_icon.is_empty() {
		val(&res, "icon_img").await
	} else {
		community_icon.to_string()
	};

	let sub = Subreddit {
		name: val(&res, "display_name").await,
		title: val(&res, "title").await,
		description: val(&res, "public_description").await,
		info: val(&res, "description_html").await.replace("\\", ""),
		icon: format_url(icon).await,
		members: format_num(members.try_into().unwrap_or(0)),
		active: format_num(active.try_into().unwrap_or(0)),
	};

	Ok(sub)
}
