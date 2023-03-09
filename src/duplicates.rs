// Handler for post duplicates.

use crate::client::json;
use crate::server::RequestExt;
use crate::subreddit::{can_access_quarantine, quarantine};
use crate::utils::{error, filter_posts, get_filters, nsfw_landing, parse_post, setting, template, Post, Preferences};

use askama::Template;
use hyper::{Body, Request, Response};
use serde_json::Value;
use std::borrow::ToOwned;
use std::collections::HashSet;
use std::vec::Vec;

/// DuplicatesParams contains the parameters in the URL.
struct DuplicatesParams {
	before: String,
	after: String,
	sort: String,
}

/// DuplicatesTemplate defines an Askama template for rendering duplicate
/// posts.
#[derive(Template)]
#[template(path = "duplicates.html")]
struct DuplicatesTemplate {
	/// params contains the relevant request parameters.
	params: DuplicatesParams,

	/// post is the post whose ID is specified in the reqeust URL. Note that
	/// this is not necessarily the "original" post.
	post: Post,

	/// duplicates is the list of posts that, per Reddit, are duplicates of
	/// Post above.
	duplicates: Vec<Post>,

	/// prefs are the user preferences.
	prefs: Preferences,

	/// url is the request URL.
	url: String,

	/// num_posts_filtered counts how many posts were filtered from the
	/// duplicates list.
	num_posts_filtered: u64,

	/// all_posts_filtered is true if every duplicate was filtered. This is an
	/// edge case but can still happen.
	all_posts_filtered: bool,
}

/// Make the GET request to Reddit. It assumes `req` is the appropriate Reddit
/// REST endpoint for enumerating post duplicates.
pub async fn item(req: Request<Body>) -> Result<Response<Body>, String> {
	let path: String = format!("{}.json?{}&raw_json=1", req.uri().path(), req.uri().query().unwrap_or_default());
	let sub = req.param("sub").unwrap_or_default();
	let quarantined = can_access_quarantine(&req, &sub);

	// Log the request in debugging mode
	#[cfg(debug_assertions)]
	dbg!(req.param("id").unwrap_or_default());

	// Send the GET, and await JSON.
	match json(path, quarantined).await {
		// Process response JSON.
		Ok(response) => {
			let post = parse_post(&response[0]["data"]["children"][0]).await;

			// Return landing page if this post if this Reddit deems this post
			// NSFW, but we have also disabled the display of NSFW content
			// or if the instance is SFW-only.
			if post.nsfw && (setting(&req, "show_nsfw") != "on" || crate::utils::sfw_only()) {
				return Ok(nsfw_landing(req).await.unwrap_or_default());
			}

			let filters = get_filters(&req);
			let (duplicates, num_posts_filtered, all_posts_filtered) = parse_duplicates(&response[1], &filters).await;

			// These are the values for the "before=", "after=", and "sort="
			// query params, respectively.
			let mut before: String = String::new();
			let mut after: String = String::new();
			let mut sort: String = String::new();

			// FIXME: We have to perform a kludge to work around a Reddit API
			// bug.
			//
			// The JSON object in "data" will never contain a "before" value so
			// it is impossible to use it to determine our position in a
			// listing. We'll make do by getting the ID of the first post in
			// the listing, setting that as our "before" value, and ask Reddit
			// to give us a batch of duplicate posts up to that post.
			//
			// Likewise, if we provide a "before" request in the GET, the
			// result won't have an "after" in the JSON, in addition to missing
			// the "before." So we will have to use the final post in the list
			// of duplicates.
			//
			// That being said, we'll also need to capture the value of the
			// "sort=" parameter as well, so we will need to inspect the
			// query key-value pairs anyway.
			let l = duplicates.len();
			if l > 0 {
				// This gets set to true if "before=" is one of the GET params.
				let mut have_before: bool = false;

				// This gets set to true if "after=" is one of the GET params.
				let mut have_after: bool = false;

				// Inspect the query key-value pairs. We will need to record
				// the value of "sort=", along with checking to see if either
				// one of "before=" or "after=" are given.
				//
				// If we're in the middle of the batch (evidenced by the
				// presence of a "before=" or "after=" parameter in the GET),
				// then use the first post as the "before" reference.
				//
				// We'll do this iteratively. Better than with .map_or()
				// since a closure will continue to operate on remaining
				// elements even after we've determined one of "before=" or
				// "after=" (or both) are in the GET request.
				//
				// In practice, here should only ever be one of "before=" or
				// "after=" and never both.
				let query_str = req.uri().query().unwrap_or_default().to_string();

				if !query_str.is_empty() {
					for param in query_str.split('&') {
						let kv: Vec<&str> = param.split('=').collect();
						if kv.len() < 2 {
							// Reject invalid query parameter.
							continue;
						}

						let key: &str = kv[0];
						match key {
							"before" => have_before = true,
							"after" => have_after = true,
							"sort" => {
								let val: &str = kv[1];
								match val {
									"new" | "num_comments" => sort = val.to_string(),
									_ => {}
								}
							}
							_ => {}
						}
					}
				}

				if have_after {
					before = "t3_".to_owned();
					before.push_str(&duplicates[0].id);
				}

				// Address potentially missing "after". If "before=" is in the
				// GET, then "after" will be null in the JSON (see FIXME
				// above).
				if have_before {
					// The next batch will need to start from one after the
					// last post in the current batch.
					after = "t3_".to_owned();
					after.push_str(&duplicates[l - 1].id);

					// Here is where things get terrible. Notice that we
					// haven't set `before`. In order to do so, we will
					// need to know if there is a batch that exists before
					// this one, and doing so requires actually fetching the
					// previous batch. In other words, we have to do yet one
					// more GET to Reddit. There is no other way to determine
					// whether or not to define `before`.
					//
					// We'll mitigate that by requesting at most one duplicate.
					let new_path: String = format!(
						"{}.json?before=t3_{}&sort={}&limit=1&raw_json=1",
						req.uri().path(),
						&duplicates[0].id,
						if sort.is_empty() { "num_comments".to_string() } else { sort.clone() }
					);
					match json(new_path, true).await {
						Ok(response) => {
							if !response[1]["data"]["children"].as_array().unwrap_or(&Vec::new()).is_empty() {
								before = "t3_".to_owned();
								before.push_str(&duplicates[0].id);
							}
						}
						Err(msg) => {
							// Abort entirely if we couldn't get the previous
							// batch.
							return error(req, msg).await;
						}
					}
				} else {
					after = response[1]["data"]["after"].as_str().unwrap_or_default().to_string();
				}
			}
			let url = req.uri().to_string();

			template(DuplicatesTemplate {
				params: DuplicatesParams { before, after, sort },
				post,
				duplicates,
				prefs: Preferences::new(&req),
				url,
				num_posts_filtered,
				all_posts_filtered,
			})
		}

		// Process error.
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

// DUPLICATES
async fn parse_duplicates(json: &serde_json::Value, filters: &HashSet<String>) -> (Vec<Post>, u64, bool) {
	let post_duplicates: &Vec<Value> = &json["data"]["children"].as_array().map_or(Vec::new(), ToOwned::to_owned);
	let mut duplicates: Vec<Post> = Vec::new();

	// Process each post and place them in the Vec<Post>.
	for val in post_duplicates.iter() {
		let post: Post = parse_post(val).await;
		duplicates.push(post);
	}

	let (num_posts_filtered, all_posts_filtered) = filter_posts(&mut duplicates, filters);
	(duplicates, num_posts_filtered, all_posts_filtered)
}
