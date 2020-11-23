// Import Crates
use actix_web::{get, App, HttpResponse, HttpServer};

// Reference local files
mod popular;
mod post;
mod subreddit;
mod user;
mod proxy;

// Create Services
#[get("/style.css")]
async fn style() -> HttpResponse {
	let file = std::fs::read_to_string("static/style.css").expect("ERROR: Could not read style.css");
	HttpResponse::Ok().content_type("text/css").body(file)
}

#[get("/robots.txt")]
async fn robots() -> HttpResponse {
	let file = std::fs::read_to_string("static/robots.txt").expect("ERROR: Could not read robots.txt");
	HttpResponse::Ok().body(file)
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
		if args[1].starts_with("--address=") || args[1].starts_with("-a=") {
			let split: Vec<&str> = args[1].split("=").collect();
			address = split[1].to_string();
		}
	}

	// start http server
	println!("Running Libreddit on {}!", address.clone());

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
