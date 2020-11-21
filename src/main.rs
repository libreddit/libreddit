// Import Crates
use actix_web::{get, App, HttpResponse, HttpServer};
use std::fs;

// Reference local files
mod popular;
mod post;
mod subreddit;
mod user;

// Create Services
#[get("/style.css")]
async fn style() -> HttpResponse {
	let file = fs::read_to_string("static/style.css").expect("ERROR: Could not read style.css");
	HttpResponse::Ok().content_type("text/css").body(file)
}

#[get("/robots.txt")]
async fn robots() -> HttpResponse {
	let file = fs::read_to_string("static/robots.txt").expect("ERROR: Could not read robots.txt");
	HttpResponse::Ok().body(file)
}

#[get("/favicon.ico")]
async fn favicon() -> HttpResponse {
	HttpResponse::Ok().body("")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
	// start http server
	println!("Running Libreddit on 0.0.0.0:8080!");

	HttpServer::new(|| {
		App::new()
			// GENERAL SERVICES
			.service(style)
			.service(favicon)
			.service(robots)
			// POST SERVICES
			.service(post::short)
			.service(post::page)
			// SUBREDDIT SERVICES
			.service(subreddit::page)
			// POPULAR SERVICES
			.service(popular::page)
			// USER SERVICES
			.service(user::page)
	})
	.bind("0.0.0.0:8080")?
	.run()
	.await
}
