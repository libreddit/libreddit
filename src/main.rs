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
	HttpServer::new(|| {
		App::new()
			// GENERAL SERVICES
			.service(style)
			.service(favicon)
			// POST SERVICES
			.service(post::short)
			.service(post::page)
			.service(post::sorted)
			// SUBREDDIT SERVICES
			.service(subreddit::page)
			.service(subreddit::sorted)
			// POPULAR SERVICES
			.service(popular::page)
			// .service(popular::sorted)
			// USER SERVICES
			.service(user::page)
	})
	.bind("127.0.0.1:8080")?
	.run()
	.await
}
