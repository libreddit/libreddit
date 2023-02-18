// CRATES
use crate::client::json;
use crate::server::RequestExt;
use crate::subreddit::{can_access_quarantine, quarantine};
use crate::utils::{
	error, format_num, get_filters, nsfw_landing, param, parse_post, rewrite_urls, setting, template, time, val, Author, Awards, Comment, Flair, FlairPart, Post, Preferences,
};
use hyper::{Body, Request, Response};

use askama::Template;
use std::collections::HashSet;

// STRUCTS
#[derive(Template)]
#[template(path = "post.html")]
struct PostTemplate {
	comments: Vec<Comment>,
	post: Post,
	sort: String,
	prefs: Preferences,
	single_thread: bool,
	url: String,
}

pub async fn item(req: Request<Body>) -> Result<Response<Body>, String> {
	// Build Reddit API path
	let mut path: String = format!("{}.json?{}&raw_json=1", req.uri().path(), req.uri().query().unwrap_or_default());
	let sub = req.param("sub").unwrap_or_default();
	let quarantined = can_access_quarantine(&req, &sub);

	// Set sort to sort query parameter
	let sort = param(&path, "sort").unwrap_or_else(|| {
		// Grab default comment sort method from Cookies
		let default_sort = setting(&req, "comment_sort");

		// If there's no sort query but there's a default sort, set sort to default_sort
		if default_sort.is_empty() {
			String::new()
		} else {
			path = format!("{}.json?{}&sort={}&raw_json=1", req.uri().path(), req.uri().query().unwrap_or_default(), default_sort);
			default_sort
		}
	});

	// Log the post ID being fetched in debug mode
	#[cfg(debug_assertions)]
	dbg!(req.param("id").unwrap_or_default());

	let single_thread = req.param("comment_id").is_some();
	let highlighted_comment = &req.param("comment_id").unwrap_or_default();

	// Send a request to the url, receive JSON in response
	match json(path, quarantined).await {
		// Otherwise, grab the JSON output from the request
		Ok(response) => {
			// Parse the JSON into Post and Comment structs
			let post = parse_post(&response[0]["data"]["children"][0]).await;

			// Return landing page if this post if this Reddit deems this post
			// NSFW, but we have also disabled the display of NSFW content
			// or if the instance is SFW-only.
			if post.nsfw && (setting(&req, "show_nsfw") != "on" || crate::utils::sfw_only()) {
				return Ok(nsfw_landing(req).await.unwrap_or_default());
			}

			let comments = parse_comments(&response[1], &post.permalink, &post.author.name, highlighted_comment, &get_filters(&req), &req);
			let url = req.uri().to_string();

			// Use the Post and Comment structs to generate a website to show users
			template(PostTemplate {
				comments,
				post,
				sort,
				prefs: Preferences::new(&req),
				single_thread,
				url,
			})
		}
		// If the Reddit API returns an error, exit and send error page to user
		Err(msg) => {
			if msg == "quarantined" || msg == "gated" {
				let sub = req.param("sub").unwrap_or_default();
				quarantine(req, sub, msg)
			} else {
				error(req, msg).await
			}
		}
	}
}

// COMMENTS
fn parse_comments(json: &serde_json::Value, post_link: &str, post_author: &str, highlighted_comment: &str, filters: &HashSet<String>, req: &Request<Body>) -> Vec<Comment> {
	// Parse the comment JSON into a Vector of Comments
	let comments = json["data"]["children"].as_array().map_or(Vec::new(), std::borrow::ToOwned::to_owned);

	// For each comment, retrieve the values to build a Comment object
	comments
		.into_iter()
		.map(|comment| {
			let kind = comment["kind"].as_str().unwrap_or_default().to_string();
			let data = &comment["data"];

			let unix_time = data["created_utc"].as_f64().unwrap_or_default();
			let (rel_time, created) = time(unix_time);

			let edited = data["edited"].as_f64().map_or((String::new(), String::new()), time);

			let score = data["score"].as_i64().unwrap_or(0);

			// If this comment contains replies, handle those too
			let replies: Vec<Comment> = if data["replies"].is_object() {
				parse_comments(&data["replies"], post_link, post_author, highlighted_comment, filters, req)
			} else {
				Vec::new()
			};

			let awards: Awards = Awards::parse(&data["all_awardings"]);

			let parent_kind_and_id = val(&comment, "parent_id");
			let parent_info = parent_kind_and_id.split('_').collect::<Vec<&str>>();

			let id = val(&comment, "id");
			let highlighted = id == highlighted_comment;

			let body = if (val(&comment, "author") == "[deleted]" && val(&comment, "body") == "[removed]") || val(&comment, "body") == "[ Removed by Reddit ]" {
				format!(
					"<div class=\"md\"><p>[removed] — <a href=\"https://www.unddit.com{}{}\">view removed comment</a></p></div>",
					post_link, id
				)
			} else {
				rewrite_urls(&val(&comment, "body_html"))
			};

			let author = Author {
				name: val(&comment, "author"),
				flair: Flair {
					flair_parts: FlairPart::parse(
						data["author_flair_type"].as_str().unwrap_or_default(),
						data["author_flair_richtext"].as_array(),
						data["author_flair_text"].as_str(),
					),
					text: val(&comment, "link_flair_text"),
					background_color: val(&comment, "author_flair_background_color"),
					foreground_color: val(&comment, "author_flair_text_color"),
				},
				distinguished: val(&comment, "distinguished"),
			};
			let is_filtered = filters.contains(&["u_", author.name.as_str()].concat());

			// Many subreddits have a default comment posted about the sub's rules etc.
			// Many libreddit users do not wish to see this kind of comment by default.
			// Reddit does not tell us which users are "bots", so a good heuristic is to
			// collapse stickied moderator comments.
			let is_moderator_comment = data["distinguished"].as_str().unwrap_or_default() == "moderator";
			let is_stickied = data["stickied"].as_bool().unwrap_or_default();
			let collapsed = (is_moderator_comment && is_stickied) || is_filtered;

			Comment {
				id,
				kind,
				parent_id: parent_info[1].to_string(),
				parent_kind: parent_info[0].to_string(),
				post_link: post_link.to_string(),
				post_author: post_author.to_string(),
				body,
				author,
				score: if data["score_hidden"].as_bool().unwrap_or_default() {
					("\u{2022}".to_string(), "Hidden".to_string())
				} else {
					format_num(score)
				},
				rel_time,
				created,
				edited,
				replies,
				highlighted,
				awards,
				collapsed,
				is_filtered,
				prefs: Preferences::new(req),
			}
		})
		.collect()
}
