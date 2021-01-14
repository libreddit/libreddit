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
		.body("User-agent: *\nAllow: /")
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
			_ => (),
		}
	}

	// start http server
	println!("Running Libreddit v{} on {}!", env!("CARGO_PKG_VERSION"), &address);

	HttpServer::new(|| {
		App::new()
			// Redirect to HTTPS
			// .wrap_fn(|req, srv| { let fut = srv.call(req); async { let mut res = fut.await?; if https {}	Ok(res) } })
			// Append trailing slash and remove double slashes
			.wrap(middleware::NormalizePath::default())
			// Default service in case no routes match
			.default_service(web::get().to(|| utils::error("Nothing here")))
			// Read static files
			.route("/style.css/", web::get().to(style))
			.route("/favicon.ico/", web::get().to(favicon))
			.route("/robots.txt/", web::get().to(robots))
			// Proxy media through Libreddit
			.route("/proxy/{url:.*}/", web::get().to(proxy::handler))
			// Browse user profile
			.route("/{scope:u|user}/{username}/", web::get().to(user::profile))
			// Short link for post
			.route("/{id:.{5,6}}/", web::get().to(post::item))
			// Configure settings
			.service(web::resource("/settings/").route(web::get().to(settings::get)).route(web::post().to(settings::set)))
			// Subreddit services
			.service(
				web::scope("/r/{sub}")
					// See posts and info about subreddit
					.route("/", web::get().to(subreddit::page))
					.route("/{sort:hot|new|top|rising|controversial}/", web::get().to(subreddit::page))
					// View post on subreddit
					.service(
						web::scope("/comments/{id}/{title}")
							.route("/", web::get().to(post::item))
							.route("/{comment_id}/", web::get().to(post::item)),
					)
					// Search inside subreddit
					.route("/search/", web::get().to(search::find))
					// View wiki of subreddit
					.service(
						web::scope("/wiki")
							.route("/", web::get().to(subreddit::wiki))
							.route("/{page}/", web::get().to(subreddit::wiki)),
					),
			)
			// Universal services
			.service(
				web::scope("")
					// Front page
					.route("/", web::get().to(subreddit::page))
					.route("/{sort:best|hot|new|top|rising|controversial}/", web::get().to(subreddit::page))
					// View Reddit wiki
					.service(
						web::scope("/wiki")
							.route("/", web::get().to(subreddit::wiki))
							.route("/{page}/", web::get().to(subreddit::wiki)),
					)
					// Search all of Reddit
					.route("/search/", web::get().to(search::find)),
			)
	})
	.bind(&address)
	.unwrap_or_else(|e| panic!("Cannot bind to the address {}: {}", address, e))
	.run()
	.await
}
