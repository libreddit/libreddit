// Import Crates
use actix_web::{
	dev::{Service, ServiceResponse},
	middleware, web, App, HttpResponse, HttpServer,
};
use futures::future::FutureExt;

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
	let mut force_https = false;

	for arg in std::env::args().collect::<Vec<String>>() {
		match arg.split('=').collect::<Vec<&str>>()[0] {
			"--address" | "-a" => address = arg.split('=').collect::<Vec<&str>>()[1].to_string(),
			"--redirect-https" | "-r" => force_https = true,
			_ => (),
		}
	}

	// start http server
	println!("Running Libreddit v{} on {}!", env!("CARGO_PKG_VERSION"), &address);

	HttpServer::new(move || {
		App::new()
			// Redirect to HTTPS if "--redirect-https" enabled
			.wrap_fn(move |req, srv| {
				let secure = req.connection_info().scheme() == "https";
				let https_url = format!("https://{}{}", req.connection_info().host(), req.uri().to_string());
				srv.call(req).map(move |res: Result<ServiceResponse, _>| {
					if force_https && !secure {
						let redirect: ServiceResponse<actix_web::dev::Body> =
							ServiceResponse::new(res.unwrap().request().clone(), HttpResponse::Found().header("Location", https_url).finish());
						Ok(redirect)
					} else {
						res
					}
				})
			})
			// Append trailing slash and remove double slashes
			.wrap(middleware::NormalizePath::default())
			// Default service in case no routes match
			.default_service(web::get().to(|| utils::error("Nothing here".to_string())))
			// Read static files
			.route("/style.css/", web::get().to(style))
			.route("/favicon.ico/", web::get().to(favicon))
			.route("/robots.txt/", web::get().to(robots))
			// Proxy media through Libreddit
			.route("/proxy/{url:.*}/", web::get().to(proxy::handler))
			// Browse user profile
			.service(
				web::scope("/{scope:user|u}").service(
					web::scope("/{username}").route("/", web::get().to(user::profile)).service(
						web::scope("/comments/{id}/{title}")
							.route("/", web::get().to(post::item))
							.route("/{comment_id}/", web::get().to(post::item)),
					),
				),
			)
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
					.route("/search/", web::get().to(search::find))
					// Short link for post
					.route("/{id:.{5,6}}/", web::get().to(post::item)),
			)
	})
	.bind(&address)
	.unwrap_or_else(|e| panic!("Cannot bind to the address {}: {}", address, e))
	.run()
	.await
}
