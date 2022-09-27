use cached::proc_macro::cached;
use futures_lite::{future::Boxed, FutureExt};
use hyper::{body, body::Buf, client, header, Body, Method, Request, Response, Uri};
use libflate::gzip;
use percent_encoding::{percent_encode, CONTROLS};
use serde_json::Value;
use std::{io, result::Result};

use crate::dbg_msg;
use crate::server::RequestExt;

const REDDIT_URL_BASE: &str = "https://www.reddit.com";

/// Gets the canonical path for a resource on Reddit. This is accomplished by
/// making a `HEAD` request to Reddit at the path given in `path`.
///
/// This function returns `Ok(Some(path))`, where `path`'s value is identical
/// to that of the value of the argument `path`, if Reddit responds to our
/// `HEAD` request with a 2xx-family HTTP code. It will also return an
/// `Ok(Some(String))` if Reddit responds to our `HEAD` request with a
/// `Location` header in the response, and the HTTP code is in the 3xx-family;
/// the `String` will contain the path as reported in `Location`. The return
/// value is `Ok(None)` if Reddit responded with a 3xx, but did not provide a
/// `Location` header. An `Err(String)` is returned if Reddit responds with a
/// 429, or if we were unable to decode the value in the `Location` header.
#[cached(size = 1024, time = 600, result = true)]
pub async fn canonical_path(path: String) -> Result<Option<String>, String> {
	let res = reddit_head(path.clone(), true).await?;

	if res.status() == 429 {
		return Err("Too many requests.".to_string());
	};

	// If Reddit responds with a 2xx, then the path is already canonical.
	if res.status().to_string().starts_with('2') {
		return Ok(Some(path));
	}

	// If Reddit responds with anything other than 3xx (except for the 2xx as
	// above), return a None.
	if !res.status().to_string().starts_with('3') {
		return Ok(None);
	}

	Ok(
		res
			.headers()
			.get(header::LOCATION)
			.map(|val| percent_encode(val.as_bytes(), CONTROLS).to_string().trim_start_matches(REDDIT_URL_BASE).to_string()),
	)
}

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
	let client: client::Client<_, hyper::Body> = client::Client::builder().build(https);

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

/// Makes a GET request to Reddit at `path`. By default, this will honor HTTP
/// 3xx codes Reddit returns and will automatically redirect.
fn reddit_get(path: String, quarantine: bool) -> Boxed<Result<Response<Body>, String>> {
	request(&Method::GET, path, true, quarantine)
}

/// Makes a HEAD request to Reddit at `path`. This will not follow redirects.
fn reddit_head(path: String, quarantine: bool) -> Boxed<Result<Response<Body>, String>> {
	request(&Method::HEAD, path, false, quarantine)
}

/// Makes a request to Reddit. If `redirect` is `true`, request_with_redirect
/// will recurse on the URL that Reddit provides in the Location HTTP header
/// in its response.
fn request(method: &'static Method, path: String, redirect: bool, quarantine: bool) -> Boxed<Result<Response<Body>, String>> {
	// Build Reddit URL from path.
	let url = format!("{}{}", REDDIT_URL_BASE, path);

	// Prepare the HTTPS connector.
	let https = hyper_rustls::HttpsConnectorBuilder::new().with_native_roots().https_or_http().enable_http1().build();

	// Construct the hyper client from the HTTPS connector.
	let client: client::Client<_, hyper::Body> = client::Client::builder().build(https);

	// Build request to Reddit. When making a GET, request gzip compression.
	// (Reddit doesn't do brotli yet.)
	let builder = Request::builder()
		.method(method)
		.uri(&url)
		.header("User-Agent", format!("web:libreddit:{}", env!("CARGO_PKG_VERSION")))
		.header("Host", "www.reddit.com")
		.header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8")
		.header("Accept-Encoding", if method == Method::GET { "gzip" } else { "identity" })
		.header("Accept-Language", "en-US,en;q=0.5")
		.header("Connection", "keep-alive")
		.header("Cookie", if quarantine { "_options=%7B%22pref_quarantine_optin%22%3A%20true%7D" } else { "" })
		.body(Body::empty());

	async move {
		match builder {
			Ok(req) => match client.request(req).await {
				Ok(mut response) => {
					// Reddit may respond with a 3xx. Decide whether or not to
					// redirect based on caller params.
					if response.status().to_string().starts_with('3') {
						if !redirect {
							return Ok(response);
						};

						return request(
							method,
							response
								.headers()
								.get("Location")
								.map(|val| {
									let new_url = percent_encode(val.as_bytes(), CONTROLS).to_string();
									format!("{}{}raw_json=1", new_url, if new_url.contains('?') { "&" } else { "?" })
								})
								.unwrap_or_default()
								.to_string(),
							true,
							quarantine,
						)
						.await;
					};

					match response.headers().get(header::CONTENT_ENCODING) {
						// Content not compressed.
						None => Ok(response),

						// Content encoded (hopefully with gzip).
						Some(hdr) => {
							match hdr.to_str() {
								Ok(val) => match val {
									"gzip" => {}
									"identity" => return Ok(response),
									_ => return Err("Reddit response was encoded with an unsupported compressor".to_string()),
								},
								Err(_) => return Err("Reddit response was invalid".to_string()),
							}

							// We get here if the body is gzip-compressed.

							// The body must be something that implements
							// std::io::Read, hence the conversion to
							// bytes::buf::Buf and then transformation into a
							// Reader.
							let mut decompressed: Vec<u8>;
							{
								let mut aggregated_body = match body::aggregate(response.body_mut()).await {
									Ok(b) => b.reader(),
									Err(e) => return Err(e.to_string()),
								};

								let mut decoder = match gzip::Decoder::new(&mut aggregated_body) {
									Ok(decoder) => decoder,
									Err(e) => return Err(e.to_string()),
								};

								decompressed = Vec::<u8>::new();
								if let Err(e) = io::copy(&mut decoder, &mut decompressed) {
									return Err(e.to_string());
								};
							}

							response.headers_mut().remove(header::CONTENT_ENCODING);
							response.headers_mut().insert(header::CONTENT_LENGTH, decompressed.len().into());
							*(response.body_mut()) = Body::from(decompressed);

							Ok(response)
						}
					}
				}
				Err(e) => {
					dbg_msg!("{} {}: {}", method, path, e);

					Err(e.to_string())
				}
			},
			Err(_) => Err("Post url contains non-ASCII characters".to_string()),
		}
	}
	.boxed()
}

// Make a request to a Reddit API and parse the JSON response
#[cached(size = 100, time = 30, result = true)]
pub async fn json(path: String, quarantine: bool) -> Result<Value, String> {
	// Closure to quickly build errors
	let err = |msg: &str, e: String| -> Result<Value, String> {
		// eprintln!("{} - {}: {}", url, msg, e);
		Err(format!("{}: {}", msg, e))
	};

	// Fetch the url...
	match reddit_get(path.clone(), quarantine).await {
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
												eprintln!("{}{} - Error parsing reddit error", REDDIT_URL_BASE, path);
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
