// Import Crates
use actix_web::{get, middleware::NormalizePath, web, App, HttpResponse, HttpServer};

// Reference local files
mod popular;
mod post;
mod proxy;
mod subreddit;
mod user;
mod utils;

// Create Services
async fn style() -> HttpResponse {
	HttpResponse::Ok().content_type("text/css").body(include_str!("../static/style.css"))
}

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
			// TRAILING SLASH MIDDLEWARE
			.wrap(NormalizePath::default())
			// GENERAL SERVICES
			.route("/style.css/", web::get().to(style))
			.route("/favicon.ico/", web::get().to(|| HttpResponse::Ok()))
			.route("/robots.txt/", web::get().to(robots))
			// PROXY SERVICE
			.route("/proxy/{url:.*}/", web::get().to(proxy::handler))
			// USER SERVICES
			.route("/u/{username}/", web::get().to(user::page))
			.route("/user/{username}/", web::get().to(user::page))
			// SUBREDDIT SERVICES
			.route("/r/{sub}/", web::get().to(subreddit::page))
			// POPULAR SERVICES
			.route("/", web::get().to(popular::page))
			// POST SERVICES
			.route("/{id:.{5,6}}/", web::get().to(post::short))
			.route("/r/{sub}/comments/{id}/{title}/", web::get().to(post::page))
			.route("/r/{sub}/comments/{id}/{title}/{comment_id}/", web::get().to(post::comment))
	})
	.bind(address.clone())
	.expect(format!("Cannot bind to the address: {}", address).as_str())
	.run()
	.await
}
