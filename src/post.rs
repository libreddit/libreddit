// CRATES
use crate::utils::*;
use tide::Request;

use async_recursion::async_recursion;

use askama::Template;

// STRUCTS
#[derive(Template)]
#[template(path = "post.html", escape = "none")]
struct PostTemplate {
	comments: Vec<Comment>,
	post: Post,
	sort: String,
	prefs: Preferences,
	single_thread: bool,
}

pub async fn item(req: Request<()>) -> tide::Result {
	// Build Reddit API path
	let mut path: String = format!("{}.json?{}&raw_json=1", req.url().path(), req.url().query().unwrap_or_default());

	// Set sort to sort query parameter
	let mut sort: String = param(&path, "sort");

	// Grab default comment sort method from Cookies
	let default_sort = cookie(&req, "comment_sort");

	// If there's no sort query but there's a default sort, set sort to default_sort
	if sort.is_empty() && !default_sort.is_empty() {
		sort = default_sort;
		path = format!(
			"{}.json?{}&sort={}&raw_json=1",
			req.url().path(),
			req.url().query().unwrap_or_default(),
			sort
		);
	}

	// Log the post ID being fetched in debug mode
	#[cfg(debug_assertions)]
	dbg!(req.param("id").unwrap_or(""));
	
	let single_thread = &req.param("comment_id").is_ok();
	let highlighted_comment = &req.param("comment_id").unwrap_or_default();

	// Send a request to the url, receive JSON in response
	match request(path).await {
		// Otherwise, grab the JSON output from the request
		Ok(res) => {
			// Parse the JSON into Post and Comment structs
			let post = parse_post(&res[0]).await;
			let comments = parse_comments(&res[1], &post.permalink, &post.author.name, *highlighted_comment).await;

			// Use the Post and Comment structs to generate a website to show users
			template(PostTemplate {
				comments,
				post,
				sort,
				prefs: prefs(req),
				single_thread: *single_thread,
			})
		}
		// If the Reddit API returns an error, exit and send error page to user
		Err(msg) => error(msg).await,
	}
}

// POSTS
async fn parse_post(json: &serde_json::Value) -> Post {
	// Retrieve post (as opposed to comments) from JSON
	let post: &serde_json::Value = &json["data"]["children"][0];

	// Grab UTC time as unix timestamp
	let (rel_time, created) = time(post["data"]["created_utc"].as_f64().unwrap_or_default());
	// Parse post score and upvote ratio
	let score = post["data"]["score"].as_i64().unwrap_or_default();
	let ratio: f64 = post["data"]["upvote_ratio"].as_f64().unwrap_or(1.0) * 100.0;

	// Determine the type of media along with the media URL
	let (post_type, media, gallery) = media(&post["data"]).await;

	// Build a post using data parsed from Reddit post API
	Post {
		id: val(post, "id"),
		title: val(post, "title"),
		community: val(post, "subreddit"),
		body: rewrite_urls(&val(post, "selftext_html")).replace("\\", ""),
		author: Author {
			name: val(post, "author"),
			flair: Flair {
				flair_parts: parse_rich_flair(
					val(post, "author_flair_type"),
					post["data"]["author_flair_richtext"].as_array(),
					post["data"]["author_flair_text"].as_str(),
				),
				background_color: val(post, "author_flair_background_color"),
				foreground_color: val(post, "author_flair_text_color"),
			},
			distinguished: val(post, "distinguished"),
		},
		permalink: val(post, "permalink"),
		score: format_num(score),
		upvote_ratio: ratio as i64,
		post_type,
		media,
		thumbnail: Media {
			url: format_url(val(post, "thumbnail").as_str()),
			width: post["data"]["thumbnail_width"].as_i64().unwrap_or_default(),
			height: post["data"]["thumbnail_height"].as_i64().unwrap_or_default(),
			poster: "".to_string(),
		},
		flair: Flair {
			flair_parts: parse_rich_flair(
				val(post, "link_flair_type"),
				post["data"]["link_flair_richtext"].as_array(),
				post["data"]["link_flair_text"].as_str(),
			),
			background_color: val(post, "link_flair_background_color"),
			foreground_color: if val(post, "link_flair_text_color") == "dark" {
				"black".to_string()
			} else {
				"white".to_string()
			},
		},
		flags: Flags {
			nsfw: post["data"]["over_18"].as_bool().unwrap_or(false),
			stickied: post["data"]["stickied"].as_bool().unwrap_or(false),
		},
		domain: val(post, "domain"),
		rel_time,
		created,
		comments: format_num(post["data"]["num_comments"].as_i64().unwrap_or_default()),
		gallery,
	}
}

// COMMENTS
#[async_recursion]
async fn parse_comments(json: &serde_json::Value, post_link: &str, post_author: &str, highlighted_comment: &str) -> Vec<Comment> {
	// Separate the comment JSON into a Vector of comments
	let comment_data = match json["data"]["children"].as_array() {
		Some(f) => f.to_owned(),
		None => Vec::new(),
	};

	let mut comments: Vec<Comment> = Vec::new();

	// For each comment, retrieve the values to build a Comment object
	for comment in comment_data {
		let unix_time = comment["data"]["created_utc"].as_f64().unwrap_or_default();
		let (rel_time, created) = time(unix_time);

		let score = comment["data"]["score"].as_i64().unwrap_or(0);
		let body = rewrite_urls(&val(&comment, "body_html"));

		let replies: Vec<Comment> = if comment["data"]["replies"].is_object() {
			parse_comments(&comment["data"]["replies"], post_link, post_author, highlighted_comment).await
		} else {
			Vec::new()
		};
		
		let parent_kind_and_id = val(&comment, "parent_id");
		let parent_info = parent_kind_and_id.split("_").collect::<Vec<&str>>();
		
		let id = val(&comment, "id");
		let highlighted = id == highlighted_comment;
		
		comments.push(Comment {
			id,
			kind: comment["kind"].as_str().unwrap_or_default().to_string(),
			parent_id: parent_info[1].to_string(),
			parent_kind: parent_info[0].to_string(),
			post_link: post_link.to_string(),
			post_author: post_author.to_string(),
			body,
			author: Author {
				name: val(&comment, "author"),
				flair: Flair {
					flair_parts: parse_rich_flair(
						val(&comment, "author_flair_type"),
						comment["data"]["author_flair_richtext"].as_array(),
						comment["data"]["author_flair_text"].as_str(),
					),
					background_color: val(&comment, "author_flair_background_color"),
					foreground_color: val(&comment, "author_flair_text_color"),
				},
				distinguished: val(&comment, "distinguished"),
			},
			score: if comment["data"]["score_hidden"].as_bool().unwrap_or_default() {
				"•".to_string()
			} else {
				format_num(score)
			},
			rel_time,
			created,
			replies,
			highlighted,
		});
	}

	comments
}
