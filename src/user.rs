use rocket_contrib::templates::Template;

#[get("/u/<username>")]
pub fn page(username: String) -> Template {
	let about: String = user_html(username.as_str());
	let posts: String = posts_html(username.as_str(), "new");

	let mut context = std::collections::HashMap::new();
	context.insert("about", about);
  context.insert("posts", posts);
  context.insert("user", username);
  context.insert("sort", String::from("new"));

	Template::render("user", context)
}

#[get("/u/<username>?<sort>")]
pub fn sorted(username: String, sort: String) -> Template {
	let about: String = user_html(username.as_str());
	let posts: String = posts_html(username.as_str(), sort.as_str());

	let mut context = std::collections::HashMap::new();
	context.insert("about", about);
  context.insert("posts", posts);
  context.insert("user", username);
  context.insert("sort", sort);

	Template::render("user", context)
}

pub fn user_html(name: &str) -> String {
	let user: User = user(name).unwrap();
	format!(r#"
		<div class="user">
			<div class="user_left">
				<img class="user_icon" src="{}">
			</div>
			<div class="user_right">
				<h2 class="user_name">u/{}</h2>
				<p class="user_description">{}</p>
			</div>
		</div>
	"#, user.icon, user.name, user.description)
}

pub struct User {
	pub name: String,
	pub icon: String,
	pub banner: String,
	pub description: String
}

pub struct Post {
	pub title: String,
	pub community: String,
	pub author: String,
	pub score: i64,
	pub image: String,
	pub url: String
}

fn user_val (j: &serde_json::Value, k: &str) -> String { String::from(j["data"]["subreddit"][k].as_str().unwrap()) }
fn post_val (j: &serde_json::Value, k: &str) -> String { String::from(j["data"][k].as_str().unwrap_or("Comment")) }

pub fn user(name: &str) -> Result<User, Box<dyn std::error::Error>> {
  let url: String = format!("https://www.reddit.com/user/{}/about.json", name);
	let resp: String = reqwest::blocking::get(&url)?.text()?;

  let data: serde_json::Value = serde_json::from_str(resp.as_str())?;
  
  Ok(User {
    name: String::from(name),
    icon: user_val(&data, "icon_img"),
    banner: user_val(&data, "banner_img"),
    description: user_val(&data, "public_description")
  })
}

pub fn posts_html (name: &str, sort: &str) -> String {
	let mut html_posts: Vec<String> = Vec::new();
	for post in posts(name, sort).unwrap().iter() {
		let hp: String = format!(r#"
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
					<h3 class="post_title"><a href="{u}">{t}</a></h3>
				</div>
				<img class="post_thumbnail" src="{thumb}">
			</div><br>
		"#, if post.score>1000{format!("{}k", post.score/1000)} else {post.score.to_string()}, sub = post.community,
				author = post.author, u = post.url, t = post.title, thumb = post.image);
		html_posts.push(hp)
	}; html_posts.join("\n")
}

pub fn posts(name: &str, sort: &str) -> Result<Vec<Post>, Box<dyn std::error::Error>> {
  let url: String = format!("https://www.reddit.com/u/{}/.json?sort={}", name, sort);
	let resp: String = reqwest::blocking::get(&url)?.text()?;
  
  let popular: serde_json::Value = serde_json::from_str(resp.as_str())?;
  
	let post_list = popular["data"]["children"].as_array().unwrap();

	let mut posts: Vec<Post> = Vec::new();
	
	for post in post_list.iter() {
    if post_val(post, "title") == "Comment" { continue };
		posts.push(Post {
			title: post_val(post, "title"),
			community: post_val(post, "subreddit"),
			author: String::from(name),
			score: post["data"]["score"].as_i64().unwrap(),
			image: String::new(),
			url: post_val(post, "permalink")
		});
	}

	Ok(posts)
}