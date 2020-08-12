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
	pub author: String,
	pub score: i64,
	pub image: String
}

pub struct Comment {
	pub body: String,
	pub author: String,
	pub score: i64
}

fn val (j: &serde_json::Value, k: &str) -> String { String::from(j["data"][k].as_str().unwrap_or("")) }

pub fn post_html (sub: &str, id: &str, title: &str) -> String {
	let post: Post = fetch_post(String::from(sub), String::from(id), String::from(title)).unwrap();
	println!("{}", post.image);
	format!(r#"
		<div class="post">
			<div class="post_left">
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
				<img class="post_image" src="{img}">
			</div>
		</div><br>
	"#, if post.score>1000{format!("{}k", post.score/1000)} else {post.score.to_string()}, sub = post.community,
			author = post.author, t = post.title, img = post.image)
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
					<h3 class="post_title">{t}</h3>
				</div>
			</div><br>
		"#, if comment.score>1000{format!("{}k", comment.score/1000)} else {comment.score.to_string()},
				author = comment.author, t = comment.body);
		html.push(hc)
	}; html.join("\n")
}

fn fetch_post (sub: String, id: String, title: String) -> Result<Post, Box<dyn std::error::Error>> {
  let url: String = format!("https://www.reddit.com/r/{}/comments/{}/{}.json", sub, id, title);
	let resp: String = reqwest::blocking::get(&url)?.text()?;
  
  let data: serde_json::Value = serde_json::from_str(resp.as_str())?;
  
	let post_data: &serde_json::Value = &data[0]["data"]["children"][0];

	Ok(Post {
		title: val(post_data, "title"),
		community: val(post_data, "subreddit"),
		author: val(post_data, "author"),
		score: post_data["data"]["score"].as_i64().unwrap(),
		image: if post_data["data"]["post_hint"]=="image" { val(post_data, "url") } else { String::new() }
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
			body: val(comment, "body"),
			author: val(comment, "author"),
			score: comment["data"]["score"].as_i64().unwrap_or(0)
		});
	}

	Ok(comments)
}