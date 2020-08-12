use rocket_contrib::templates::Template;

#[allow(dead_code)]
#[get("/r/<sub_name>")]
pub fn page(sub_name: String) -> Template {
	let about: String = if sub_name != "popular" { subreddit_html(sub_name.as_str()) } else {String::new()};
	let posts: String = posts_html(sub_name.as_str(), "hot");

	let mut context = std::collections::HashMap::new();
	context.insert("about", about);
	context.insert("sort", String::from("hot"));
	context.insert("sub", sub_name);
	context.insert("posts", posts);

	Template::render("subreddit", context)
}

#[allow(dead_code)]
#[get("/r/<sub_name>/<sort>")]
pub fn sorted(sub_name: String, sort: String) -> Template {
	let about: String = if sub_name != "popular" { subreddit_html(sub_name.as_str()) } else {String::new()};
	let posts: String = posts_html(sub_name.as_str(), sort.as_str());

	let mut context = std::collections::HashMap::new();
	context.insert("about", about);
	context.insert("sort", sort);
	context.insert("sub", sub_name);
	context.insert("posts", posts);

	Template::render("subreddit", context)
}

pub struct Post {
	pub title: String,
	pub community: String,
	pub author: String,
	pub score: i64,
	pub image: String,
	pub url: String
}

pub struct Subreddit {
	pub name: String,
	pub title: String,
	pub description: String,
	pub icon: String
}

fn val (j: &serde_json::Value, k: &str) -> String { String::from(j["data"][k].as_str().unwrap()) }

pub fn posts_html (sub: &str, sort: &str) -> String {
	let mut html_posts: Vec<String> = Vec::new();
	for post in posts(sub, sort).unwrap().iter() {
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

pub fn subreddit_html(sub: &str) -> String {
	let sub = subreddit(sub).unwrap();
	format!(r#"
		<div class="subreddit">
			<div class="subreddit_left">
				<img class="subreddit_icon" src="{}">
			</div>
			<div class="subreddit_right">
				<h2 class="subreddit_title">{}</h2>
				<p class="subreddit_name">r/{}</p>
			</div>
		</div>
	"#, sub.icon, sub.title, sub.name)
}

pub fn subreddit(sub: &str) -> Result<Subreddit, Box<dyn std::error::Error>> {
	let url: String = format!("https://www.reddit.com/r/{}/about.json", sub);
	let resp: String = reqwest::blocking::get(&url)?.text()?;

	let data: serde_json::Value = serde_json::from_str(resp.as_str())?;

	let icon: String = String::from(data["data"]["community_icon"].as_str().unwrap()); //val(&data, "community_icon");
	let icon_split: std::str::Split<&str> = icon.split("?");
	let icon_parts: Vec<&str> = icon_split.collect();

	Ok(Subreddit {
		name: val(&data, "display_name"),
		title: val(&data, "title"),
		description: val(&data, "public_description"),
		icon: String::from(icon_parts[0]),
	}) 
}

pub fn posts(sub: &str, sort: &str) -> Result<Vec<Post>, Box<dyn std::error::Error>> {
	let url: String = format!("https://www.reddit.com/r/{}/{}.json", sub, sort);
	let resp: String = reqwest::blocking::get(&url)?.text()?;
	
	let popular: serde_json::Value = serde_json::from_str(resp.as_str())?;
	let post_list = popular["data"]["children"].as_array().unwrap();

	let mut posts: Vec<Post> = Vec::new();
	
	for post in post_list.iter() {
		let img = if val(post, "thumbnail").starts_with("https:/") { val(post, "thumbnail") } else { String::new() };
		posts.push(Post {
			title: val(post, "title"),
			community: val(post, "subreddit"),
			author: val(post, "author"),
			score: post["data"]["score"].as_i64().unwrap(),
			image: img,
			url: val(post, "permalink")
		});
	}

	Ok(posts)
}