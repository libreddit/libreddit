use rocket_contrib::templates::Template;

#[path = "subreddit.rs"] mod subreddit;

#[get("/")]
pub fn page() -> Template {
	let posts: String = subreddit::posts_html("popular", "best");

	let mut context = std::collections::HashMap::new();
	context.insert("about", String::new());
	context.insert("sort", String::from("best"));
	context.insert("posts", posts);

	Template::render("popular", context)
}

#[get("/<sort>")]
pub fn sorted(sort: String) -> Template {
	let posts: String = subreddit::posts_html("popular", sort.as_str());

	let mut context = std::collections::HashMap::new();
	context.insert("about", String::new());
	context.insert("sort", sort);
	context.insert("posts", posts);

	Template::render("popular", context)
}
