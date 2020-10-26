// CRATES
use actix_web::{get, web, HttpResponse, Result};
use serde::Deserialize;
use askama::Template;

#[path = "subreddit.rs"] mod subreddit;

// STRUCTS
#[derive(Template)]
#[template(path = "popular.html", escape = "none")]
struct PopularTemplate {
	posts: Vec<subreddit::Post>,
	sort: String
}

#[derive(Deserialize)]
pub struct Params {
  sort: Option<String>
}

#[get("/")]
pub async fn page(params: web::Query<Params>) -> Result<HttpResponse> {
	match &params.sort {
    Some(sort) => render("popular".to_string(), sort.to_string()).await,
    None => render("popular".to_string(), "hot".to_string()).await,
	}
}

async fn render(sub_name: String, sort: String) -> Result<HttpResponse> {
	let posts: Vec<subreddit::Post> = subreddit::posts(sub_name, &sort).await;
  
	let s = PopularTemplate {
		posts: posts,
		sort: sort
	}
	.render()
	.unwrap();
	Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

// #[get("/?<sort>")]
// pub fn sorted(sort: String) -> Template {
// 	println!("{}", sort);
// 	let posts: Vec<subreddit::Post> = subreddit::posts(&"popular".to_string(), &sort).unwrap();

// 	let mut context = std::collections::HashMap::new();
// 	context.insert("about", String::new());
// 	context.insert("sort", sort);
// 	context.insert("posts", subreddit::posts_html(posts));

// 	Template::render("popular", context)
// }
