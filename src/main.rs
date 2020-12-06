// Import Crates
use actix_web::{get, App, HttpResponse, HttpServer};

// Reference local files
mod popular;
mod post;
mod proxy;
mod subreddit;
mod user;
mod utils;

// Create Services
#[get("/style.css")]
async fn style() -> HttpResponse {
	HttpResponse::Ok().content_type("text/css").body(include_str!("../static/style.css"))
}

#[get("/robots.txt")]
async fn robots() -> HttpResponse {
	HttpResponse::Ok().body(include_str!("../static/robots.txt"))
}

#[get("/favicon.ico")]
async fn favicon() -> HttpResponse {
	HttpResponse::Ok().body("")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
	let args: Vec<String> = std::env::args().collect();
	let mut address = "0.0.0.0:8080".to_string();

	if args.len() > 1 {
		for arg in args {
			if arg.starts_with("--address=") || arg.starts_with("-a=") {
				let split: Vec<&str> = arg.split("=").collect();
				address = split[1].to_string();
			}
		}
	}

	// start http server
	println!("Running Libreddit v{} on {}!", env!("CARGO_PKG_VERSION"), address.clone());

	HttpServer::new(|| {
		App::new()
			// GENERAL SERVICES
			.service(style)
			.service(favicon)
			.service(robots)
			// PROXY SERVICE
			.service(proxy::handler)
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
	.bind(address.clone())
	.expect(format!("Cannot bind to the address: {}", address).as_str())
	.run()
	.await
}
