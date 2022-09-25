use cached::proc_macro::cached;
use futures_lite::{future::Boxed, FutureExt};
use hyper::{body::Buf, client, Body, Request, Response, Uri};
use percent_encoding::{percent_encode, CONTROLS};
use serde_json::Value;
use std::result::Result;

use crate::server::RequestExt;

pub async fn proxy(req: Request<Body>, format: &str) -> Result<Response<Body>, String> {
	let mut url = format!("{}?{}", format, req.uri().query().unwrap_or_default());

	// For each parameter in request
	for (name, value) in req.params().iter() {
		// Fill the parameter value in the url
		url = url.replace(&format!("{{{}}}", name), value);
	}

	stream(&url, &req).await
}

async fn stream(url: &str, req: &Request<Body>) -> Result<Response<Body>, String> {
	// First parameter is target URL (mandatory).
	let uri = url.parse::<Uri>().map_err(|_| "Couldn't parse URL".to_string())?;

	// Prepare the HTTPS connector.
	let https = hyper_rustls::HttpsConnectorBuilder::new().with_native_roots().https_only().enable_http1().build();

	// Build the hyper client from the HTTPS connector.
	let client: client::Client<_, Body> = client::Client::builder().build(https);

	let mut builder = Request::get(uri);

	// Copy useful headers from original request
	for &key in &["Range", "If-Modified-Since", "Cache-Control"] {
		if let Some(value) = req.headers().get(key) {
			builder = builder.header(key, value);
		}
	}

	let stream_request = builder.body(Body::empty()).map_err(|_| "Couldn't build empty body in stream".to_string())?;

	client
		.request(stream_request)
		.await
		.map(|mut res| {
			let mut rm = |key: &str| res.headers_mut().remove(key);

			rm("access-control-expose-headers");
			rm("server");
			rm("vary");
			rm("etag");
			rm("x-cdn");
			rm("x-cdn-client-region");
			rm("x-cdn-name");
			rm("x-cdn-server-region");
			rm("x-reddit-cdn");
			rm("x-reddit-video-features");

			res
		})
		.map_err(|e| e.to_string())
}

fn request(url: String, quarantine: bool) -> Boxed<Result<Response<Body>, String>> {
	// Prepare the HTTPS connector.
	let https = hyper_rustls::HttpsConnectorBuilder::new().with_native_roots().https_or_http().enable_http1().build();

	// Construct the hyper client from the HTTPS connector.
	let client: client::Client<_, Body> = client::Client::builder().build(https);

	// Build request
	let builder = Request::builder()
		.method("GET")
		.uri(&url)
		.header("User-Agent", format!("web:libbacon:{}", env!("CARGO_PKG_VERSION")))
		.header("Host", "www.reddit.com")
		.header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8")
		.header("Accept-Language", "en-US,en;q=0.5")
		.header("Connection", "keep-alive")
		.header("Cookie", if quarantine { "_options=%7B%22pref_quarantine_optin%22%3A%20true%7D" } else { "" })
		.body(Body::empty());

	async move {
		match builder {
			Ok(req) => match client.request(req).await {
				Ok(response) => {
					if response.status().to_string().starts_with('3') {
						request(
							response
								.headers()
								.get("Location")
								.map(|val| {
									let new_url = percent_encode(val.as_bytes(), CONTROLS).to_string();
									format!("{}{}raw_json=1", new_url, if new_url.contains('?') { "&" } else { "?" })
								})
								.unwrap_or_default()
								.to_string(),
							quarantine,
						)
						.await
					} else {
						Ok(response)
					}
				}
				Err(e) => Err(e.to_string()),
			},
			Err(_) => Err("Post url contains non-ASCII characters".to_string()),
		}
	}
	.boxed()
}

// Make a request to a Reddit API and parse the JSON response
#[cached(size = 100, time = 30, result = true)]
pub async fn json(path: String, quarantine: bool) -> Result<Value, String> {
	// Build Reddit url from path
	let url = format!("https://www.reddit.com{}", path);

	// Closure to quickly build errors
	let err = |msg: &str, e: String| -> Result<Value, String> {
		// eprintln!("{} - {}: {}", url, msg, e);
		Err(format!("{}: {}", msg, e))
	};

	// Fetch the url...
	match request(url.clone(), quarantine).await {
		Ok(response) => {
			let status = response.status();

			// asynchronously aggregate the chunks of the body
			match hyper::body::aggregate(response).await {
				Ok(body) => {
					// Parse the response from Reddit as JSON
					match serde_json::from_reader(body.reader()) {
						Ok(value) => {
							let json: Value = value;
							// If Reddit returned an error
							if json["error"].is_i64() {
								Err(
									json["reason"]
										.as_str()
										.unwrap_or_else(|| {
											json["message"].as_str().unwrap_or_else(|| {
												eprintln!("{} - Error parsing reddit error", url);
												"Error parsing reddit error"
											})
										})
										.to_string(),
								)
							} else {
								Ok(json)
							}
						}
						Err(e) => {
							if status.is_server_error() {
								Err("Reddit is having issues, check if there's an outage".to_string())
							} else {
								err("Failed to parse page JSON data", e.to_string())
							}
						}
					}
				}
				Err(e) => err("Failed receiving body from Reddit", e.to_string()),
			}
		}
		Err(e) => err("Couldn't send request to Reddit", e),
	}
}
