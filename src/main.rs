// Import Crates
use tide::{Middleware, Next, Request, Response};
// use surf::utils::async_trait;

// Reference local files
// mod post;
// mod proxy;
// mod search;
// mod settings;
// mod subreddit;
// mod user;
// mod utils;

// Build middleware
// struct HttpsRedirect<HttpsOnly>(HttpsOnly);

// #[async_trait]
// impl<State, HttpsOnly> Middleware<State> for HttpsRedirect<HttpsOnly>
// where
// 	State: Clone + Send + Sync + 'static, 
// 	HttpsOnly: Into<bool> + Copy + Send + Sync + 'static
// {
// 	async fn handle(&self, request: Request<State>, next: Next<'_, State>) -> tide::Result {
// 		let secure = request.url().scheme() == "https";

// 		if self.0.into() && !secure {
// 			let mut secured = request.url().to_owned();
// 			secured.set_scheme("https").unwrap_or_default();

// 			Ok(Response::builder(302)
// 				.header("Location", secured.to_string())
// 				.build())
// 		} else {
// 			Ok(next.run(request).await)
// 		}
// 	}
// }

// Create Services
// async fn style(_req: Request<()>) -> tide::Result {
// 	Ok(Response::builder(200).content_type("text/css").body(include_str!("../static/style.css")).build())
// }

// async fn robots(_req: Request<()>) -> tide::Result {
// 	Ok(Response::builder(200)
// 		.content_type("text/plain")
// 		.header("Cache-Control", "public, max-age=1209600, s-maxage=86400")
// 		.body("User-agent: *\nAllow: /")
// 		.build())
// }

// async fn favicon(_req: Request<()>) -> tide::Result {
// 	Ok(Response::builder(200)
// 		.content_type("image/vnd.microsoft.icon")
// 		.header("Cache-Control", "public, max-age=1209600, s-maxage=86400")
// 		.body(include_bytes!("../static/favicon.ico").as_ref())
// 		.build())
// }

#[async_std::main]
async fn main() -> tide::Result<()> {
	// let mut address = "0.0.0.0:8080".to_string();
	// let mut force_https = false;

	// for arg in std::env::args().collect::<Vec<String>>() {
	// 	match arg.split('=').collect::<Vec<&str>>()[0] {
	// 		"--address" | "-a" => address = arg.split('=').collect::<Vec<&str>>()[1].to_string(),
	// 		"--redirect-https" | "-r" => force_https = true,
	// 		_ => (),
	// 	}
	// }

	// // Start HTTP server
	// println!("Running Libreddit v{} on {}!", env!("CARGO_PKG_VERSION"), &address);

	let mut app = tide::new();
	// Redirect to HTTPS if "--redirect-https" enabled
	// app.with(HttpsRedirect(force_https));

	// // Append trailing slash and remove double slashes
	// .wrap(middleware::NormalizePath::default())

	// // Default service in case no routes match
	// .default_service(web::get().to(|| utils::error("Nothing here".to_string())))
	
	
	// // Read static files
	// .route("/style.css/", web::get().to(style))
	// .route("/favicon.ico/", web::get().to(favicon))
	// .route("/robots.txt/", web::get().to(robots))
	// app.at("/style.css").get(style);
	// app.at("/favicon.ico").get(favicon);
	// app.at("/robots.txt").get(robots);
	
	// // Proxy media through Libreddit
	// .route("/proxy/{url:.*}/", web::get().to(proxy::handler))
	// app.at("/proxy/*url").get(proxy::handler);
	
	// // Browse user profile
	// .service(
	// 	web::scope("/{scope:user|u}").service(
	// 		web::scope("/{username}").route("/", web::get().to(user::profile)).service(
	// 			web::scope("/comments/{id}/{title}")
	// 				.route("/", web::get().to(post::item))
	// 				.route("/{comment_id}/", web::get().to(post::item)),
	// 		),
	// 	),
	// )
	// app.at("/user/:name").get(user::profile);
	// app.at("/user/:name/comments/:id/:title").get(post::item);
	// app.at("/user/:name/comments/:id/:title/:comment").get(post::item);
	
	// // Configure settings
	// .service(web::resource("/settings/").route(web::get().to(settings::get)).route(web::post().to(settings::set)))
	// app.at("/settings").get(settings::get).post(settings::set);
	
	// // Subreddit services
	// .service(
	// 	web::scope("/r/{sub}")
	// 		// See posts and info about subreddit
	// 		.route("/", web::get().to(subreddit::page))
	// 		.route("/{sort:hot|new|top|rising|controversial}/", web::get().to(subreddit::page))
	// 		// View post on subreddit
	// 		.service(
	// 			web::scope("/comments/{id}/{title}")
	// 				.route("/", web::get().to(post::item))
	// 				.route("/{comment_id}/", web::get().to(post::item)),
	// 		)
	// 		// Search inside subreddit
	// 		.route("/search/", web::get().to(search::find))
	// 		// View wiki of subreddit
	// 		.service(
	// 			web::scope("/wiki")
	// 				.route("/", web::get().to(subreddit::wiki))
	// 				.route("/{page}/", web::get().to(subreddit::wiki)),
	// 		),
	// )
	
	// // Front page
	// .route("/", web::get().to(subreddit::page))
	// .route("/{sort:best|hot|new|top|rising|controversial}/", web::get().to(subreddit::page))
	
	// // View Reddit wiki
	// .service(
	// 	web::scope("/wiki")
	// 		.route("/", web::get().to(subreddit::wiki))
	// 		.route("/{page}/", web::get().to(subreddit::wiki)),
	// )
	
	// // Search all of Reddit
	// .route("/search/", web::get().to(search::find))
	
	// // Short link for post
	// .route("/{id:.{5,6}}/", web::get().to(post::item))
	app.at("/test").get(test);
	app.listen("127.0.0.1:8080").await?;
	Ok(())
}

async fn test(_req: tide::Request<()>) -> tide::Result {
	async_std::task::sleep(std::time::Duration::from_secs(6)).await;
	Ok("a".into())
}
