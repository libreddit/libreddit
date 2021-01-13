// Import Crates
use actix_web::{middleware, web, App, HttpResponse, HttpServer}; // dev::Service

// Reference local files
mod post;
mod proxy;
mod search;
mod settings;
mod subreddit;
mod user;
mod utils;

// Create Services
async fn style() -> HttpResponse {
	HttpResponse::Ok().content_type("text/css").body(include_str!("../static/style.css"))
}

async fn robots() -> HttpResponse {
	HttpResponse::Ok()
		.header("Cache-Control", "public, max-age=1209600, s-maxage=86400")
		.body(include_str!("../static/robots.txt"))
}

async fn favicon() -> HttpResponse {
	HttpResponse::Ok()
		.header("Cache-Control", "public, max-age=1209600, s-maxage=86400")
		.body(include_bytes!("../static/favicon.ico").as_ref())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
	let mut address = "0.0.0.0:8080".to_string();
	// let mut https = false;

	for arg in std::env::args().collect::<Vec<String>>() {
		match arg.split('=').collect::<Vec<&str>>()[0] {
			"--address" | "-a" => address = arg.split('=').collect::<Vec<&str>>()[1].to_string(),
			// "--redirect-https" | "-r" => https = true,
			_ => {}
		}
	}

	// start http server
	println!("Running Libreddit v{} on {}!", env!("CARGO_PKG_VERSION"), &address);

	HttpServer::new(|| {
		App::new()
			// REDIRECT TO HTTPS
			// .wrap(middleware::DefaultHeaders::new().header("Strict-Transport-Security", "max-age=31536000"))
			// .wrap_fn(|req, srv| {
			// 	let fut = srv.call(req);
			// 	async {
			// 		let mut res = fut.await?;
			// 		if https {
			// 			res.headers_mut().insert(
			// 				actix_web::http::header::STRICT_TRANSPORT_SECURITY, actix_web::http::HeaderValue::from_static("max-age=31536000;"),
			// 			);
			// 		}
			// 		Ok(res)
			// 	}
			// })
			// TRAILING SLASH MIDDLEWARE
			.wrap(middleware::NormalizePath::default())
			// DEFAULT SERVICE
			.default_service(web::get().to(|| utils::error("Nothing here".to_string())))
			// GENERAL SERVICES
			.route("/style.css/", web::get().to(style))
			.route("/favicon.ico/", web::get().to(favicon))
			.route("/robots.txt/", web::get().to(robots))
			// SETTINGS SERVICE
			.route("/settings/", web::get().to(settings::get))
			.route("/settings/", web::post().to(settings::set))
			// PROXY SERVICE
			.route("/proxy/{url:.*}/", web::get().to(proxy::handler))
			// SEARCH SERVICES
			.route("/search/", web::get().to(search::find))
			.route("r/{sub}/search/", web::get().to(search::find))
			// USER SERVICES
			.route("/u/{username}/", web::get().to(user::profile))
			.route("/user/{username}/", web::get().to(user::profile))
			// WIKI SERVICES
			.route("/wiki/", web::get().to(subreddit::wiki))
			.route("/wiki/{page}/", web::get().to(subreddit::wiki))
			.route("/r/{sub}/wiki/", web::get().to(subreddit::wiki))
			.route("/r/{sub}/wiki/{page}/", web::get().to(subreddit::wiki))
			// SUBREDDIT SERVICES
			.route("/r/{sub}/", web::get().to(subreddit::page))
			.route("/r/{sub}/{sort:hot|new|top|rising|controversial}/", web::get().to(subreddit::page))
			// POPULAR SERVICES
			.route("/", web::get().to(subreddit::page))
			.route("/{sort:best|hot|new|top|rising|controversial}/", web::get().to(subreddit::page))
			// POST SERVICES
			.route("/{id:.{5,6}}/", web::get().to(post::item))
			.route("/r/{sub}/comments/{id}/{title}/", web::get().to(post::item))
			.route("/r/{sub}/comments/{id}/{title}/{comment_id}/", web::get().to(post::item))
	})
	.bind(&address)
	.unwrap_or_else(|e| panic!("Cannot bind to the address {}: {}", address, e))
	.run()
	.await
}
