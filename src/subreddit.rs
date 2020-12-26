// CRATES
use crate::utils::{fetch_posts, format_num, format_url, request, val, ErrorTemplate, Params, Post, Subreddit};
use actix_web::{http::StatusCode, web, HttpResponse, Result};
use askama::Template;
use std::convert::TryInto;

// STRUCTS
#[derive(Template)]
#[template(path = "subreddit.html", escape = "none")]
struct SubredditTemplate {
	sub: Subreddit,
	posts: Vec<Post>,
	sort: String,
	ends: (String, String),
}

// SERVICES
#[allow(dead_code)]
pub async fn page(web::Path(sub): web::Path<String>, params: web::Query<Params>) -> Result<HttpResponse> {
	render(sub, params.sort.clone(), (params.before.clone(), params.after.clone())).await
}

pub async fn render(sub_name: String, sort: Option<String>, ends: (Option<String>, Option<String>)) -> Result<HttpResponse> {
	let sorting = sort.unwrap_or("hot".to_string());
	let before = ends.1.clone().unwrap_or(String::new()); // If there is an after, there must be a before

	// Build the Reddit JSON API url
	let url = match ends.0 {
		Some(val) => format!("r/{}/{}.json?before={}&count=25", sub_name, sorting, val),
		None => match ends.1 {
			Some(val) => format!("r/{}/{}.json?after={}&count=25", sub_name, sorting, val),
			None => format!("r/{}/{}.json", sub_name, sorting),
		},
	};

	let sub_result = if !&sub_name.contains("+") {
		subreddit(&sub_name).await
	} else {
		Ok(Subreddit {
			name: String::new(),
			title: String::new(),
			description: String::new(),
			icon: String::new(),
			members: String::new(),
			active: String::new(),
		})
	};
	let items_result = fetch_posts(url, String::new()).await;

	if sub_result.is_err() || items_result.is_err() {
		let s = ErrorTemplate {
			message: sub_result.err().unwrap().to_string(),
		}
		.render()
		.unwrap();
		Ok(HttpResponse::Ok().status(StatusCode::NOT_FOUND).content_type("text/html").body(s))
	} else {
		let sub = sub_result.unwrap();
		let items = items_result.unwrap();

		let s = SubredditTemplate {
			sub: sub,
			posts: items.0,
			sort: sorting,
			ends: (before, items.1),
		}
		.render()
		.unwrap();
		Ok(HttpResponse::Ok().content_type("text/html").body(s))
	}
}

// SUBREDDIT
async fn subreddit(sub: &String) -> Result<Subreddit, &'static str> {
	// Build the Reddit JSON API url
	let url: String = format!("r/{}/about.json", sub);

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
	let community_icon: &str = res["data"]["community_icon"].as_str().unwrap().split("?").collect::<Vec<&str>>()[0];
	let icon = if community_icon.is_empty() {
		val(&res, "icon_img").await
	} else {
		community_icon.to_string()
	};

	let sub = Subreddit {
		name: val(&res, "display_name").await,
		title: val(&res, "title").await,
		description: val(&res, "public_description").await,
		icon: format_url(icon).await,
		members: format_num(members.try_into().unwrap()),
		active: format_num(active.try_into().unwrap()),
	};

	Ok(sub)
}
