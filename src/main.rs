// Import Crates
// use askama::filters::format;
use surf::utils::async_trait;
use tide::{utils::After, Middleware, Next, Request, Response};

// Reference local files
mod post;
mod proxy;
mod search;
mod settings;
mod subreddit;
mod user;
mod utils;

// Build middleware
struct HttpsRedirect<HttpsOnly>(HttpsOnly);
struct NormalizePath;

#[async_trait]
impl<State, HttpsOnly> Middleware<State> for HttpsRedirect<HttpsOnly>
where
	State: Clone + Send + Sync + 'static,
	HttpsOnly: Into<bool> + Copy + Send + Sync + 'static,
{
	async fn handle(&self, request: Request<State>, next: Next<'_, State>) -> tide::Result {
		let secure = request.url().scheme() == "https";

		if self.0.into() && !secure {
			let mut secured = request.url().to_owned();
			secured.set_scheme("https").unwrap_or_default();

			Ok(Response::builder(302).header("Location", secured.to_string()).build())
		} else {
			Ok(next.run(request).await)
		}
	}
}

#[async_trait]
impl<State: Clone + Send + Sync + 'static> Middleware<State> for NormalizePath {
	async fn handle(&self, request: Request<State>, next: Next<'_, State>) -> tide::Result {
		if !request.url().path().ends_with('/') {
			Ok(Response::builder(301).header("Location", format!("{}/", request.url().path())).build())
		} else {
			Ok(next.run(request).await)
		}
	}
}

// Create Services
async fn style(_req: Request<()>) -> tide::Result {
	Ok(
		Response::builder(200)
			.content_type("text/css")
			.body(include_str!("../static/style.css"))
			.build(),
	)
}

// Required for creating a PWA
async fn manifest(_req: Request<()>) -> tide::Result {
	Ok(
		Response::builder(200)
			.content_type("application/json")
			.body(include_str!("../static/manifest.json"))
			.build(),
	)
}

// Required for the manifest to be valid
async fn pwa_logo(_req: Request<()>) -> tide::Result {
	Ok(
		Response::builder(200)
			.content_type("image/png")
			.body(include_bytes!("../static/logo.png").as_ref())
			.build(),
	)
}

// Required for iOS App Icons
async fn iphone_logo(_req: Request<()>) -> tide::Result {
	Ok(
		Response::builder(200)
			.content_type("image/png")
			.body(include_bytes!("../static/touch-icon-iphone.png").as_ref())
			.build(),
	)
}

async fn robots(_req: Request<()>) -> tide::Result {
	Ok(
		Response::builder(200)
			.content_type("text/plain")
			.header("Cache-Control", "public, max-age=1209600, s-maxage=86400")
			.body("User-agent: *\nAllow: /")
			.build(),
	)
}

async fn favicon(_req: Request<()>) -> tide::Result {
	Ok(
		Response::builder(200)
			.content_type("image/vnd.microsoft.icon")
			.header("Cache-Control", "public, max-age=1209600, s-maxage=86400")
			.body(include_bytes!("../static/favicon.ico").as_ref())
			.build(),
	)
}

#[async_std::main]
async fn main() -> tide::Result<()> {
	let mut address = "0.0.0.0:8080".to_string();
	let mut force_https = false;

	for arg in std::env::args().collect::<Vec<String>>() {
		match arg.split('=').collect::<Vec<&str>>()[0] {
			"--address" | "-a" => address = arg.split('=').collect::<Vec<&str>>()[1].to_string(),
			"--redirect-https" | "-r" => force_https = true,
			_ => (),
		}
	}

	// Start HTTP server
	println!("Running Libreddit v{} on {}!", env!("CARGO_PKG_VERSION"), &address);

	let mut app = tide::new();

	// Redirect to HTTPS if "--redirect-https" enabled
	app.with(HttpsRedirect(force_https));

	// Append trailing slash and remove double slashes
	app.with(NormalizePath);

	// Apply default headers for security
	app.with(After(|mut res: Response| async move {
		res.insert_header("Referrer-Policy", "no-referrer");
		res.insert_header("X-Content-Type-Options", "nosniff");
		res.insert_header("X-Frame-Options", "DENY");
		res.insert_header(
			"Content-Security-Policy",
			"default-src 'none'; manifest-src 'self'; media-src 'self'; style-src 'self' 'unsafe-inline'; base-uri 'none'; img-src 'self' data:; form-action 'self'; frame-ancestors 'none';",
		);
		Ok(res)
	}));

	// Read static files
	app.at("/style.css/").get(style);
	app.at("/favicon.ico/").get(favicon);
	app.at("/robots.txt/").get(robots);
	app.at("/manifest.json/").get(manifest);
	app.at("/logo.png/").get(pwa_logo);
	app.at("/touch-icon-iphone.png/").get(iphone_logo);

	// Proxy media through Libreddit
	app.at("/proxy/*url/").get(proxy::handler);

	// Browse user profile
	app.at("/u/:name/").get(user::profile);
	app.at("/u/:name/comments/:id/:title/").get(post::item);
	app.at("/u/:name/comments/:id/:title/:comment/").get(post::item);

	app.at("/user/:name/").get(user::profile);
	app.at("/user/:name/comments/:id/:title/").get(post::item);
	app.at("/user/:name/comments/:id/:title/:comment/").get(post::item);

	// Configure settings
	app.at("/settings/").get(settings::get).post(settings::set);

	// Subreddit services
	// See posts and info about subreddit
	app.at("/r/:sub/").get(subreddit::page);
	// Handle subscribe/unsubscribe
	app.at("/r/:sub/subscribe/").post(subreddit::subscriptions);
	app.at("/r/:sub/unsubscribe/").post(subreddit::subscriptions);
	// View post on subreddit
	app.at("/r/:sub/comments/:id/:title/").get(post::item);
	app.at("/r/:sub/comments/:id/:title/:comment_id/").get(post::item);
	// Search inside subreddit
	app.at("/r/:sub/search/").get(search::find);
	// View wiki of subreddit
	app.at("/r/:sub/w/").get(subreddit::wiki);
	app.at("/r/:sub/w/:page/").get(subreddit::wiki);
	app.at("/r/:sub/wiki/").get(subreddit::wiki);
	app.at("/r/:sub/wiki/:page/").get(subreddit::wiki);
	// Sort subreddit posts
	app.at("/r/:sub/:sort/").get(subreddit::page);

	// Front page
	app.at("/").get(subreddit::page);

	// View Reddit wiki
	app.at("/w/").get(subreddit::wiki);
	app.at("/w/:page/").get(subreddit::wiki);
	app.at("/wiki/").get(subreddit::wiki);
	app.at("/wiki/:page/").get(subreddit::wiki);

	// Search all of Reddit
	app.at("/search/").get(search::find);

	// Short link for post
	// .route("/{sort:best|hot|new|top|rising|controversial}/", web::get().to(subreddit::page))
	// .route("/{id:.{5,6}}/", web::get().to(post::item))
	app.at("/:id/").get(|req: Request<()>| async {
		match req.param("id").unwrap_or_default() {
			"best" | "hot" | "new" | "top" | "rising" | "controversial" => subreddit::page(req).await,
			_ => post::item(req).await,
		}
	});

	// Default service in case no routes match
	app.at("*").get(|_| utils::error("Nothing here".to_string()));

	app.listen("127.0.0.1:8080").await?;
	Ok(())
}
