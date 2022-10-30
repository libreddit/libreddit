// Global specifiers
#![forbid(unsafe_code)]
#![allow(clippy::cmp_owned)]

// Reference local files
mod post;
mod search;
mod settings;
mod subreddit;
mod user;
mod utils;

// Import Crates
use clap::{Arg, Command};

use futures_lite::FutureExt;
use hyper::{header::HeaderValue, Body, Request, Response};

mod client;
use client::{canonical_path, proxy};
use server::RequestExt;
use utils::{error, redirect, ThemeAssets};

mod server;

// Create Services

// Required for the manifest to be valid
async fn pwa_logo() -> Result<Response<Body>, String> {
	Ok(
		Response::builder()
			.status(200)
			.header("content-type", "image/png")
			.body(include_bytes!("../static/logo.png").as_ref().into())
			.unwrap_or_default(),
	)
}

// Required for iOS App Icons
async fn iphone_logo() -> Result<Response<Body>, String> {
	Ok(
		Response::builder()
			.status(200)
			.header("content-type", "image/png")
			.body(include_bytes!("../static/apple-touch-icon.png").as_ref().into())
			.unwrap_or_default(),
	)
}

async fn favicon() -> Result<Response<Body>, String> {
	Ok(
		Response::builder()
			.status(200)
			.header("content-type", "image/vnd.microsoft.icon")
			.header("Cache-Control", "public, max-age=1209600, s-maxage=86400")
			.body(include_bytes!("../static/favicon.ico").as_ref().into())
			.unwrap_or_default(),
	)
}

async fn font() -> Result<Response<Body>, String> {
	Ok(
		Response::builder()
			.status(200)
			.header("content-type", "font/woff2")
			.header("Cache-Control", "public, max-age=1209600, s-maxage=86400")
			.body(include_bytes!("../static/Inter.var.woff2").as_ref().into())
			.unwrap_or_default(),
	)
}

async fn resource(body: &str, content_type: &str, cache: bool) -> Result<Response<Body>, String> {
	let mut res = Response::builder()
		.status(200)
		.header("content-type", content_type)
		.body(body.to_string().into())
		.unwrap_or_default();

	if cache {
		if let Ok(val) = HeaderValue::from_str("public, max-age=1209600, s-maxage=86400") {
			res.headers_mut().insert("Cache-Control", val);
		}
	}

	Ok(res)
}

async fn style() -> Result<Response<Body>, String> {
	let mut res = include_str!("../static/style.css").to_string();
	for file in ThemeAssets::iter() {
		res.push('\n');
		let theme = ThemeAssets::get(file.as_ref()).unwrap();
		res.push_str(std::str::from_utf8(theme.data.as_ref()).unwrap());
	}
	Ok(
		Response::builder()
			.status(200)
			.header("content-type", "text/css")
			.header("Cache-Control", "public, max-age=1209600, s-maxage=86400")
			.body(res.to_string().into())
			.unwrap_or_default(),
	)
}

#[tokio::main]
async fn main() {
	let matches = Command::new("Libreddit")
		.version(env!("CARGO_PKG_VERSION"))
		.about("Private front-end for Reddit written in Rust ")
		.arg(
			Arg::new("redirect-https")
				.short('r')
				.long("redirect-https")
				.help("Redirect all HTTP requests to HTTPS (no longer functional)")
				.num_args(0),
		)
		.arg(
			Arg::new("address")
				.short('a')
				.long("address")
				.value_name("ADDRESS")
				.help("Sets address to listen on")
				.default_value("0.0.0.0")
				.num_args(1),
		)
		.arg(
			Arg::new("port")
				.short('p')
				.long("port")
				.value_name("PORT")
				.help("Port to listen on")
				.default_value("8080")
				.num_args(1),
		)
		.arg(
			Arg::new("hsts")
				.short('H')
				.long("hsts")
				.value_name("EXPIRE_TIME")
				.help("HSTS header to tell browsers that this site should only be accessed over HTTPS")
				.default_value("604800")
				.num_args(1),
		)
		.get_matches();

	let address = matches.get_one("address").map(|m: &String| m.as_str()).unwrap_or("0.0.0.0");
	let port = std::env::var("PORT").unwrap_or_else(|_| matches.get_one("port").map(|m: &String| m.as_str()).unwrap_or("8080").to_string());
	let hsts = matches.get_one("hsts").map(|m: &String| m.as_str());

	let listener = [address, ":", &port].concat();

	println!("Starting Libreddit...");

	// Begin constructing a server
	let mut app = server::Server::new();

	// Define default headers (added to all responses)
	app.default_headers = headers! {
		"Referrer-Policy" => "no-referrer",
		"X-Content-Type-Options" => "nosniff",
		"X-Frame-Options" => "DENY",
		"Content-Security-Policy" => "default-src 'none'; font-src 'self'; script-src 'self' blob:; manifest-src 'self'; media-src 'self' data: blob: about:; style-src 'self' 'unsafe-inline'; base-uri 'none'; img-src 'self' data:; form-action 'self'; frame-ancestors 'none'; connect-src 'self'; worker-src blob:;"
	};

	if let Some(expire_time) = hsts {
		if let Ok(val) = HeaderValue::from_str(&format!("max-age={}", expire_time)) {
			app.default_headers.insert("Strict-Transport-Security", val);
		}
	}

	// Read static files
	app.at("/style.css").get(|_| style().boxed());
	app
		.at("/manifest.json")
		.get(|_| resource(include_str!("../static/manifest.json"), "application/json", false).boxed());
	app
		.at("/robots.txt")
		.get(|_| resource("User-agent: *\nDisallow: /u/\nDisallow: /user/", "text/plain", true).boxed());
	app.at("/favicon.ico").get(|_| favicon().boxed());
	app.at("/logo.png").get(|_| pwa_logo().boxed());
	app.at("/Inter.var.woff2").get(|_| font().boxed());
	app.at("/touch-icon-iphone.png").get(|_| iphone_logo().boxed());
	app.at("/apple-touch-icon.png").get(|_| iphone_logo().boxed());
	app
		.at("/playHLSVideo.js")
		.get(|_| resource(include_str!("../static/playHLSVideo.js"), "text/javascript", false).boxed());
	app
		.at("/hls.min.js")
		.get(|_| resource(include_str!("../static/hls.min.js"), "text/javascript", false).boxed());

	// Proxy media through Libreddit
	app.at("/vid/:id/:size").get(|r| proxy(r, "https://v.redd.it/{id}/DASH_{size}").boxed());
	app.at("/hls/:id/*path").get(|r| proxy(r, "https://v.redd.it/{id}/{path}").boxed());
	app.at("/img/*path").get(|r| proxy(r, "https://i.redd.it/{path}").boxed());
	app.at("/thumb/:point/:id").get(|r| proxy(r, "https://{point}.thumbs.redditmedia.com/{id}").boxed());
	app.at("/emoji/:id/:name").get(|r| proxy(r, "https://emoji.redditmedia.com/{id}/{name}").boxed());
	app
		.at("/preview/:loc/award_images/:fullname/:id")
		.get(|r| proxy(r, "https://{loc}view.redd.it/award_images/{fullname}/{id}").boxed());
	app.at("/preview/:loc/:id").get(|r| proxy(r, "https://{loc}view.redd.it/{id}").boxed());
	app.at("/style/*path").get(|r| proxy(r, "https://styles.redditmedia.com/{path}").boxed());
	app.at("/static/*path").get(|r| proxy(r, "https://www.redditstatic.com/{path}").boxed());

	// Browse user profile
	app
		.at("/u/:name")
		.get(|r| async move { Ok(redirect(format!("/user/{}", r.param("name").unwrap_or_default()))) }.boxed());
	app.at("/u/:name/comments/:id/:title").get(|r| post::item(r).boxed());
	app.at("/u/:name/comments/:id/:title/:comment_id").get(|r| post::item(r).boxed());

	app.at("/user/[deleted]").get(|req| error(req, "User has deleted their account".to_string()).boxed());
	app.at("/user/:name").get(|r| user::profile(r).boxed());
	app.at("/user/:name/:listing").get(|r| user::profile(r).boxed());
	app.at("/user/:name/comments/:id").get(|r| post::item(r).boxed());
	app.at("/user/:name/comments/:id/:title").get(|r| post::item(r).boxed());
	app.at("/user/:name/comments/:id/:title/:comment_id").get(|r| post::item(r).boxed());

	// Configure settings
	app.at("/settings").get(|r| settings::get(r).boxed()).post(|r| settings::set(r).boxed());
	app.at("/settings/restore").get(|r| settings::restore(r).boxed());
	app.at("/settings/update").get(|r| settings::update(r).boxed());

	// Subreddit services
	app
		.at("/r/:sub")
		.get(|r| subreddit::community(r).boxed())
		.post(|r| subreddit::add_quarantine_exception(r).boxed());

	app
		.at("/r/u_:name")
		.get(|r| async move { Ok(redirect(format!("/user/{}", r.param("name").unwrap_or_default()))) }.boxed());

	app.at("/r/:sub/subscribe").post(|r| subreddit::subscriptions_filters(r).boxed());
	app.at("/r/:sub/unsubscribe").post(|r| subreddit::subscriptions_filters(r).boxed());
	app.at("/r/:sub/filter").post(|r| subreddit::subscriptions_filters(r).boxed());
	app.at("/r/:sub/unfilter").post(|r| subreddit::subscriptions_filters(r).boxed());

	app.at("/r/:sub/comments/:id").get(|r| post::item(r).boxed());
	app.at("/r/:sub/comments/:id/:title").get(|r| post::item(r).boxed());
	app.at("/r/:sub/comments/:id/:title/:comment_id").get(|r| post::item(r).boxed());
	app.at("/comments/:id").get(|r| post::item(r).boxed());
	app.at("/comments/:id/comments").get(|r| post::item(r).boxed());
	app.at("/comments/:id/comments/:comment_id").get(|r| post::item(r).boxed());
	app.at("/comments/:id/:title").get(|r| post::item(r).boxed());
	app.at("/comments/:id/:title/:comment_id").get(|r| post::item(r).boxed());

	app.at("/r/:sub/search").get(|r| search::find(r).boxed());

	app
		.at("/r/:sub/w")
		.get(|r| async move { Ok(redirect(format!("/r/{}/wiki", r.param("sub").unwrap_or_default()))) }.boxed());
	app
		.at("/r/:sub/w/*page")
		.get(|r| async move { Ok(redirect(format!("/r/{}/wiki/{}", r.param("sub").unwrap_or_default(), r.param("wiki").unwrap_or_default()))) }.boxed());
	app.at("/r/:sub/wiki").get(|r| subreddit::wiki(r).boxed());
	app.at("/r/:sub/wiki/*page").get(|r| subreddit::wiki(r).boxed());

	app.at("/r/:sub/about/sidebar").get(|r| subreddit::sidebar(r).boxed());

	app.at("/r/:sub/:sort").get(|r| subreddit::community(r).boxed());

	// Front page
	app.at("/").get(|r| subreddit::community(r).boxed());

	// View Reddit wiki
	app.at("/w").get(|_| async { Ok(redirect("/wiki".to_string())) }.boxed());
	app
		.at("/w/*page")
		.get(|r| async move { Ok(redirect(format!("/wiki/{}", r.param("page").unwrap_or_default()))) }.boxed());
	app.at("/wiki").get(|r| subreddit::wiki(r).boxed());
	app.at("/wiki/*page").get(|r| subreddit::wiki(r).boxed());

	// Search all of Reddit
	app.at("/search").get(|r| search::find(r).boxed());

	// Handle about pages
	app.at("/about").get(|req| error(req, "About pages aren't added yet".to_string()).boxed());

	app.at("/:id").get(|req: Request<Body>| {
		Box::pin(async move {
			match req.param("id").as_deref() {
				// Sort front page
				Some("best" | "hot" | "new" | "top" | "rising" | "controversial") => subreddit::community(req).await,

				// Short link for post
				Some(id) if (5..7).contains(&id.len()) => match canonical_path(format!("/{}", id)).await {
					Ok(path_opt) => match path_opt {
						Some(path) => Ok(redirect(path)),
						None => error(req, "Post ID is invalid. It may point to a post on a community that has been banned.").await,
					},
					Err(e) => error(req, e).await,
				},

				// Error message for unknown pages
				_ => error(req, "Nothing here".to_string()).await,
			}
		})
	});

	// Default service in case no routes match
	app.at("/*").get(|req| error(req, "Nothing here".to_string()).boxed());

	println!("Running Libreddit v{} on {}!", env!("CARGO_PKG_VERSION"), listener);

	let server = app.listen(listener);

	// Run this server for... forever!
	if let Err(e) = server.await {
		eprintln!("Server error: {}", e);
	}
}
