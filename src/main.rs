// Import Crates
use actix_files::NamedFile;
use actix_web::{get, App, HttpResponse, HttpServer, Result};

// Reference local files
mod popular;
mod post;
mod subreddit;
mod user;

// Create Services
#[get("/style.css")]
async fn style() -> Result<NamedFile> {
	let file = NamedFile::open("static/style.css");
	Ok(file?)
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
