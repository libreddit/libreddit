extern crate comrak;
use comrak::{markdown_to_html, ComrakOptions};
use rocket_contrib::templates::Template;

#[get("/r/<subreddit>/comments/<id>/<title>")]
pub fn page(subreddit: String, id: String, title: String) -> Template {
	let post: String = post_html(subreddit.as_str(), id.as_str(), title.as_str());
	let comments: String = comments_html(subreddit, id, title);

	let mut context = std::collections::HashMap::new();
	context.insert("comments", comments);
	context.insert("post", post);
	// context.insert("sort", String::from("hot"));
	// context.insert("sub", String::from(subreddit.as_str()));

	Template::render("post", context)
}

pub struct Post {
	pub title: String,
	pub community: String,
	pub body: String,
	pub author: String,
	pub score: i64,
	pub media: String
}

pub struct Comment {
	pub body: String,
	pub author: String,
	pub score: i64
}

fn val (j: &serde_json::Value, k: &str) -> String { String::from(j["data"][k].as_str().unwrap_or("")) }

pub fn post_html (sub: &str, id: &str, title: &str) -> String {
	let post: Post = fetch_post(String::from(sub), String::from(id), String::from(title)).unwrap();
	format!(r#"
		<div class="post" style="border: 2px solid #555;background: #222;">
			<div class="post_left" style="background: #333;">
				<button class="post_upvote">↑</button>
				<h3 class="post_score">{}</h3>
				<button class="post_upvote">↓</button>
			</div>
			<div class="post_right">
				<p>
					<b><a class="post_subreddit" href="/r/{sub}">r/{sub}</a></b>
					•
					Posted by 
					<a class="post_author" href="/u/{author}">u/{author}</a>
				</p>
				<h3 class="post_title">{t}</h3>
				{media}
				<h4 class="post_body">{b}</h4>
			</div>
		</div><br>
	"#, if post.score>1000{format!("{}k", post.score/1000)} else {post.score.to_string()}, sub = post.community,
			author = post.author, t = post.title, media = post.media, b = post.body)
}

fn comments_html (sub: String, id: String, title: String) -> String {
	let mut html: Vec<String> = Vec::new();
	for comment in fetch_comments(sub, id, title).unwrap().iter() {
		let hc: String = format!(r#"
			<div class="post">
				<div class="post_left">
					<button class="post_upvote">↑</button>
					<h3 class="post_score">{}</h3>
					<button class="post_upvote">↓</button>
				</div>
				<div class="post_right">
					<p>Posted by <a class="post_author" href="/u/{author}">u/{author}</a></p>
					<h4 class="post_body">{t}</h4>
				</div>
			</div><br>
		"#, if comment.score>1000{format!("{}k", comment.score/1000)} else {comment.score.to_string()},
				author = comment.author, t = comment.body);
		html.push(hc)
	}; html.join("\n")
}

fn media(data: &serde_json::Value) -> String {
	let post_hint: &str = data["data"]["post_hint"].as_str().unwrap_or("");
	let has_media: bool = data["data"]["media"].is_object();

	let media: String = if !has_media { format!(r#"<h4 class="post_body"><a href="{u}">{u}</a></h4>"#, u=data["data"]["url"].as_str().unwrap()) }
											else { format!(r#"<img class="post_image" src="{}.png"/>"#, data["data"]["url"].as_str().unwrap()) };

	match post_hint {
		"hosted:video" => format!(r#"<video class="post_image" src="{}" controls/>"#, data["data"]["media"]["reddit_video"]["fallback_url"].as_str().unwrap()),
		"image" => format!(r#"<img class="post_image" src="{}"/>"#, data["data"]["url"].as_str().unwrap()),
		"self" => String::from(""),
		_ => media
	}
}

fn fetch_post (sub: String, id: String, title: String) -> Result<Post, Box<dyn std::error::Error>> {
  let url: String = format!("https://www.reddit.com/r/{}/comments/{}/{}.json", sub, id, title);
	let resp: String = reqwest::blocking::get(&url)?.text()?;
  
  let data: serde_json::Value = serde_json::from_str(resp.as_str())?;
  
	let post_data: &serde_json::Value = &data[0]["data"]["children"][0];

	Ok(Post {
		title: val(post_data, "title"),
		community: val(post_data, "subreddit"),
		body: markdown_to_html(post_data["data"]["selftext"].as_str().unwrap(), &ComrakOptions::default()),
		author: val(post_data, "author"),
		score: post_data["data"]["score"].as_i64().unwrap(),
		media: media(post_data)
	})
}

fn fetch_comments (sub: String, id: String, title: String) -> Result<Vec<Comment>, Box<dyn std::error::Error>> {
  let url: String = format!("https://www.reddit.com/r/{}/comments/{}/{}.json", sub, id, title);
	let resp: String = reqwest::blocking::get(&url)?.text()?;
  
  let data: serde_json::Value = serde_json::from_str(resp.as_str())?;
  
	let comment_data = data[1]["data"]["children"].as_array().unwrap();

	let mut comments: Vec<Comment> = Vec::new();
	
	for comment in comment_data.iter() {
		comments.push(Comment {
			body: markdown_to_html(comment["data"]["body"].as_str().unwrap_or(""), &ComrakOptions::default()),
			author: val(comment, "author"),
			score: comment["data"]["score"].as_i64().unwrap_or(0)
		});
	}

	Ok(comments)
}