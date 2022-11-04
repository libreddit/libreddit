use brotli::enc::{BrotliCompress, BrotliEncoderParams};
use cached::proc_macro::cached;
use cookie::Cookie;
use core::f64;
use futures_lite::{future::Boxed, Future, FutureExt};
use hyper::{
	body,
	body::HttpBody,
	header,
	service::{make_service_fn, service_fn},
	HeaderMap,
};
use hyper::{Body, Method, Request, Response, Server as HyperServer};
use libflate::gzip;
use route_recognizer::{Params, Router};
use std::{
	cmp::Ordering,
	io,
	pin::Pin,
	result::Result,
	str::{from_utf8, Split},
	string::ToString,
};
use time::Duration;

use crate::dbg_msg;

type BoxResponse = Pin<Box<dyn Future<Output = Result<Response<Body>, String>> + Send>>;

/// Compressors for the response Body, in ascending order of preference.
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
enum CompressionType {
	Passthrough,
	Gzip,
	Brotli,
}

/// All browsers support gzip, so if we are given `Accept-Encoding: *`, deliver
/// gzipped-content.
///
/// Brotli would be nice universally, but Safari (iOS, iPhone, macOS) reportedly
/// doesn't support it yet.
const DEFAULT_COMPRESSOR: CompressionType = CompressionType::Gzip;

impl CompressionType {
	/// Returns a `CompressionType` given a content coding
	/// in [RFC 7231](https://datatracker.ietf.org/doc/html/rfc7231#section-5.3.4)
	/// format.
	fn parse(s: &str) -> Option<CompressionType> {
		let c = match s {
			// Compressors we support.
			"gzip" => CompressionType::Gzip,
			"br" => CompressionType::Brotli,

			// The wildcard means that we can choose whatever
			// compression we prefer. In this case, use the
			// default.
			"*" => DEFAULT_COMPRESSOR,

			// Compressor not supported.
			_ => return None,
		};

		Some(c)
	}
}

impl ToString for CompressionType {
	fn to_string(&self) -> String {
		match self {
			CompressionType::Gzip => "gzip".to_string(),
			CompressionType::Brotli => "br".to_string(),
			_ => String::new(),
		}
	}
}

pub struct Route<'a> {
	router: &'a mut Router<fn(Request<Body>) -> BoxResponse>,
	path: String,
}

pub struct Server {
	pub default_headers: HeaderMap,
	router: Router<fn(Request<Body>) -> BoxResponse>,
}

#[macro_export]
macro_rules! headers(
	{ $($key:expr => $value:expr),+ } => {
		{
			let mut m = hyper::HeaderMap::new();
			$(
				if let Ok(val) = hyper::header::HeaderValue::from_str($value) {
					m.insert($key, val);
				}
			)+
			m
		}
	 };
);

pub trait RequestExt {
	fn params(&self) -> Params;
	fn param(&self, name: &str) -> Option<String>;
	fn set_params(&mut self, params: Params) -> Option<Params>;
	fn cookies(&self) -> Vec<Cookie>;
	fn cookie(&self, name: &str) -> Option<Cookie>;
}

pub trait ResponseExt {
	fn cookies(&self) -> Vec<Cookie>;
	fn insert_cookie(&mut self, cookie: Cookie);
	fn remove_cookie(&mut self, name: String);
}

impl RequestExt for Request<Body> {
	fn params(&self) -> Params {
		self.extensions().get::<Params>().unwrap_or(&Params::new()).clone()
		// self.extensions()
		// 	.get::<RequestMeta>()
		// 	.and_then(|meta| meta.route_params())
		// 	.expect("Routerify: No RouteParams added while processing request")
	}

	fn param(&self, name: &str) -> Option<String> {
		self.params().find(name).map(std::borrow::ToOwned::to_owned)
	}

	fn set_params(&mut self, params: Params) -> Option<Params> {
		self.extensions_mut().insert(params)
	}

	fn cookies(&self) -> Vec<Cookie> {
		self.headers().get("Cookie").map_or(Vec::new(), |header| {
			header
				.to_str()
				.unwrap_or_default()
				.split("; ")
				.map(|cookie| Cookie::parse(cookie).unwrap_or_else(|_| Cookie::named("")))
				.collect()
		})
	}

	fn cookie(&self, name: &str) -> Option<Cookie> {
		self.cookies().into_iter().find(|c| c.name() == name)
	}
}

impl ResponseExt for Response<Body> {
	fn cookies(&self) -> Vec<Cookie> {
		self.headers().get("Cookie").map_or(Vec::new(), |header| {
			header
				.to_str()
				.unwrap_or_default()
				.split("; ")
				.map(|cookie| Cookie::parse(cookie).unwrap_or_else(|_| Cookie::named("")))
				.collect()
		})
	}

	fn insert_cookie(&mut self, cookie: Cookie) {
		if let Ok(val) = header::HeaderValue::from_str(&cookie.to_string()) {
			self.headers_mut().append("Set-Cookie", val);
		}
	}

	fn remove_cookie(&mut self, name: String) {
		let mut cookie = Cookie::named(name);
		cookie.set_path("/");
		cookie.set_max_age(Duration::seconds(1));
		if let Ok(val) = header::HeaderValue::from_str(&cookie.to_string()) {
			self.headers_mut().append("Set-Cookie", val);
		}
	}
}

impl Route<'_> {
	fn method(&mut self, method: Method, dest: fn(Request<Body>) -> BoxResponse) -> &mut Self {
		self.router.add(&format!("/{}{}", method.as_str(), self.path), dest);
		self
	}

	/// Add an endpoint for `GET` requests
	pub fn get(&mut self, dest: fn(Request<Body>) -> BoxResponse) -> &mut Self {
		self.method(Method::GET, dest)
	}

	/// Add an endpoint for `POST` requests
	pub fn post(&mut self, dest: fn(Request<Body>) -> BoxResponse) -> &mut Self {
		self.method(Method::POST, dest)
	}
}

impl Server {
	pub fn new() -> Self {
		Server {
			default_headers: HeaderMap::new(),
			router: Router::new(),
		}
	}

	pub fn at(&mut self, path: &str) -> Route {
		Route {
			path: path.to_owned(),
			router: &mut self.router,
		}
	}

	pub fn listen(self, addr: String) -> Boxed<Result<(), hyper::Error>> {
		let make_svc = make_service_fn(move |_conn| {
			// For correct borrowing, these values need to be borrowed
			let router = self.router.clone();
			let default_headers = self.default_headers.clone();

			// This is the `Service` that will handle the connection.
			// `service_fn` is a helper to convert a function that
			// returns a Response into a `Service`.
			// let shared_router = router.clone();
			async move {
				Ok::<_, String>(service_fn(move |req: Request<Body>| {
					let req_headers = req.headers().clone();
					let def_headers = default_headers.clone();

					// Remove double slashes and decode encoded slashes
					let mut path = req.uri().path().replace("//", "/").replace("%2F", "/");

					// Remove trailing slashes
					if path != "/" && path.ends_with('/') {
						path.pop();
					}

					// Match the visited path with an added route
					match router.recognize(&format!("/{}{}", req.method().as_str(), path)) {
						// If a route was configured for this path
						Ok(found) => {
							let mut parammed = req;
							parammed.set_params(found.params().clone());

							// Run the route's function
							let func = (found.handler().to_owned().to_owned())(parammed);
							async move {
								match func.await {
									Ok(mut res) => {
										res.headers_mut().extend(def_headers);
										let _ = compress_response(req_headers, &mut res).await;

										Ok(res)
									}
									Err(msg) => new_boilerplate(def_headers, req_headers, 500, Body::from(msg)).await,
								}
							}
							.boxed()
						}
						// If there was a routing error
						Err(e) => async move { new_boilerplate(def_headers, req_headers, 404, e.into()).await }.boxed(),
					}
				}))
			}
		});

		// Build SocketAddr from provided address
		let address = &addr.parse().unwrap_or_else(|_| panic!("Cannot parse {} as address (example format: 0.0.0.0:8080)", addr));

		// Bind server to address specified above. Gracefully shut down if CTRL+C is pressed
		let server = HyperServer::bind(address).serve(make_svc).with_graceful_shutdown(async {
			// Wait for the CTRL+C signal
			tokio::signal::ctrl_c().await.expect("Failed to install CTRL+C signal handler");
		});

		server.boxed()
	}
}

/// Create a boilerplate Response for error conditions. This response will be
/// compressed if requested by client.
async fn new_boilerplate(
	default_headers: HeaderMap<header::HeaderValue>,
	req_headers: HeaderMap<header::HeaderValue>,
	status: u16,
	body: Body,
) -> Result<Response<Body>, String> {
	match Response::builder().status(status).body(body) {
		Ok(mut res) => {
			let _ = compress_response(req_headers, &mut res).await;

			res.headers_mut().extend(default_headers.clone());
			Ok(res)
		}
		Err(msg) => Err(msg.to_string()),
	}
}

/// Determines the desired compressor based on the Accept-Encoding header.
///
/// This function will honor the [q-value](https://developer.mozilla.org/en-US/docs/Glossary/Quality_values)
///  for each compressor. The q-value is an optional parameter, a decimal value
/// on \[0..1\], to order the compressors by preference. An Accept-Encoding value
/// with no q-values is also accepted.
///
/// Here are [examples](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Accept-Encoding#examples)
/// of valid Accept-Encoding headers.
///
/// ```http
/// Accept-Encoding: gzip
/// Accept-Encoding: gzip, compress, br
/// Accept-Encoding: br;q=1.0, gzip;q=0.8, *;q=0.1
/// ```
fn determine_compressor(accept_encoding: &str) -> Option<CompressionType> {
	if accept_encoding.is_empty() {
		return None;
	};

	// Keep track of the compressor candidate based on both the client's
	// preference and our own. Concrete examples:
	//
	// 1. "Accept-Encoding: gzip, br" => assuming we like brotli more than
	//    gzip, and the browser supports brotli, we choose brotli
	//
	// 2. "Accept-Encoding: gzip;q=0.8, br;q=0.3" => the client has stated a
	//    preference for gzip over brotli, so we choose gzip
	//
	// To do this, we need to define a struct which contains the requested
	// requested compressor (abstracted as a CompressionType enum) and the
	// q-value. If no q-value is defined for the compressor, we assume one of
	// 1.0. We first compare compressor candidates by comparing q-values, and
	// then CompressionTypes. We keep track of whatever is the greatest per our
	// ordering.

	struct CompressorCandidate {
		alg: CompressionType,
		q: f64,
	}

	impl Ord for CompressorCandidate {
		fn cmp(&self, other: &Self) -> Ordering {
			// Compare q-values. Break ties with the
			// CompressionType values.

			match self.q.total_cmp(&other.q) {
				Ordering::Equal => self.alg.cmp(&other.alg),
				ord => ord,
			}
		}
	}

	impl PartialOrd for CompressorCandidate {
		fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
			// Guard against NAN, both on our end and on the other.
			if self.q.is_nan() || other.q.is_nan() {
				return None;
			};

			// f64 and CompressionType are ordered, except in the case
			// where the f64 is NAN (which we checked against), so we
			// can safely return a Some here.
			Some(self.cmp(other))
		}
	}

	impl PartialEq for CompressorCandidate {
		fn eq(&self, other: &Self) -> bool {
			(self.q == other.q) && (self.alg == other.alg)
		}
	}

	impl Eq for CompressorCandidate {}

	// This is the current candidate.
	//
	// Assmume no candidate so far. We do this by assigning the sentinel value
	// of negative infinity to the q-value. If this value is negative infinity,
	// that means there was no viable compressor candidate.
	let mut cur_candidate = CompressorCandidate {
		alg: CompressionType::Passthrough,
		q: f64::NEG_INFINITY,
	};

	// This loop reads the requested compressors and keeps track of whichever
	// one has the highest priority per our heuristic.
	for val in accept_encoding.to_string().split(',') {
		let mut q: f64 = 1.0;

		// The compressor and q-value (if the latter is defined)
		// will be delimited by semicolons.
		let mut spl: Split<char> = val.split(';');

		// Get the compressor. For example, in
		//   gzip;q=0.8
		// this grabs "gzip" in the string. It
		// will further validate the compressor against the
		// list of those we support. If it is not supported,
		// we move onto the next one.
		let compressor: CompressionType = match spl.next() {
			// CompressionType::parse will return the appropriate enum given
			// a string. For example, it will return CompressionType::Gzip
			// when given "gzip".
			Some(s) => match CompressionType::parse(s.trim()) {
				Some(candidate) => candidate,

				// We don't support the requested compression algorithm.
				None => continue,
			},

			// We should never get here, but I'm paranoid.
			None => continue,
		};

		// Get the q-value. This might not be defined, in which case assume
		// 1.0.
		if let Some(s) = spl.next() {
			if !(s.len() > 2 && s.starts_with("q=")) {
				// If the q-value is malformed, the header is malformed, so
				// abort.
				return None;
			}

			match s[2..].parse::<f64>() {
				Ok(val) => {
					if (0.0..=1.0).contains(&val) {
						q = val;
					} else {
						// If the value is outside [0..1], header is malformed.
						// Abort.
						return None;
					};
				}
				Err(_) => {
					// If this isn't a f64, then assume a malformed header
					// value and abort.
					return None;
				}
			}
		};

		// If new_candidate > cur_candidate, make new_candidate the new
		// cur_candidate. But do this safely! It is very possible that
		// someone gave us the string "NAN", which (&str).parse::<f64>
		// will happily translate to f64::NAN.
		let new_candidate = CompressorCandidate { alg: compressor, q };
		if let Some(ord) = new_candidate.partial_cmp(&cur_candidate) {
			if ord == Ordering::Greater {
				cur_candidate = new_candidate;
			}
		};
	}

	if cur_candidate.q != f64::NEG_INFINITY {
		Some(cur_candidate.alg)
	} else {
		None
	}
}

/// Compress the response body, if possible or desirable. The Body will be
/// compressed in place, and a new header Content-Encoding will be set
/// indicating the compression algorithm.
///
/// This function deems Body eligible compression if and only if the following
/// conditions are met:
///
/// 1. the HTTP client requests a compression encoding in the Content-Encoding
///    header (hence the need for the req_headers);
///
/// 2. the content encoding corresponds to a compression algorithm we support;
///
/// 3. the Media type in the Content-Type response header is text with any
///    subtype (e.g. text/plain) or application/json.
///
/// compress_response returns Ok on successful compression, or if not all three
/// conditions above are met. It returns Err if there was a problem decoding
/// any header in either req_headers or res, but res will remain intact.
///
/// This function logs errors to stderr, but only in debug mode. No information
/// is logged in release builds.
async fn compress_response(req_headers: HeaderMap<header::HeaderValue>, res: &mut Response<Body>) -> Result<(), String> {
	// Check if the data is eligible for compression.
	if let Some(hdr) = res.headers().get(header::CONTENT_TYPE) {
		match from_utf8(hdr.as_bytes()) {
			Ok(val) => {
				let s = val.to_string();

				// TODO: better determination of what is eligible for compression
				if !(s.starts_with("text/") || s.starts_with("application/json")) {
					return Ok(());
				};
			}
			Err(e) => {
				dbg_msg!(e);
				return Err(e.to_string());
			}
		};
	} else {
		// Response declares no Content-Type. Assume for simplicity that it
		// cannot be compressed.
		return Ok(());
	};

	// Don't bother if the size of the size of the response body will fit
	// within an IP frame (less the bytes that make up the TCP/IP and HTTP
	// headers).
	if res.body().size_hint().lower() < 1452 {
		return Ok(());
	};

	// Quick and dirty closure for extracting a header from the request and
	// returning it as a &str.
	let get_req_header = |k: header::HeaderName| -> Option<&str> {
		match req_headers.get(k) {
			Some(hdr) => match from_utf8(hdr.as_bytes()) {
				Ok(val) => Some(val),

				#[cfg(debug_assertions)]
				Err(e) => {
					dbg_msg!(e);
					None
				}

				#[cfg(not(debug_assertions))]
				Err(_) => None,
			},
			None => None,
		}
	};

	// Check to see which compressor is requested, and if we can use it.
	let accept_encoding: &str = match get_req_header(header::ACCEPT_ENCODING) {
		Some(val) => val,
		None => return Ok(()), // Client requested no compression.
	};

	let compressor: CompressionType = match determine_compressor(accept_encoding) {
		Some(c) => c,
		None => return Ok(()),
	};

	// Get the body from the response.
	let body_bytes: Vec<u8> = match body::to_bytes(res.body_mut()).await {
		Ok(b) => b.to_vec(),
		Err(e) => {
			dbg_msg!(e);
			return Err(e.to_string());
		}
	};

	// Compress!
	match compress_body(compressor, body_bytes) {
		Ok(compressed) => {
			// We get here iff the compression was successful. Replace the body
			// with the compressed payload, and add the appropriate
			// Content-Encoding header in the response.
			res.headers_mut().insert(header::CONTENT_ENCODING, compressor.to_string().parse().unwrap());
			*(res.body_mut()) = Body::from(compressed);
		}

		Err(e) => return Err(e),
	}

	Ok(())
}

/// Compresses a `Vec<u8>` given a [`CompressionType`].
///
/// This is a helper function for [`compress_response`] and should not be
/// called directly.

// I've chosen a TTL of 600 (== 10 minutes) since compression is
// computationally expensive and we don't want to be doing it often. This is
// larger than client::json's TTL, but that's okay, because if client::json
// returns a new serde_json::Value, body_bytes changes, so this function will
// execute again.
#[cached(size = 100, time = 600, result = true)]
fn compress_body(compressor: CompressionType, body_bytes: Vec<u8>) -> Result<Vec<u8>, String> {
	// io::Cursor implements io::Read, required for our encoders.
	let mut reader = io::Cursor::new(body_bytes);

	let compressed: Vec<u8> = match compressor {
		CompressionType::Gzip => {
			let mut gz: gzip::Encoder<Vec<u8>> = match gzip::Encoder::new(Vec::new()) {
				Ok(gz) => gz,
				Err(e) => {
					dbg_msg!(e);
					return Err(e.to_string());
				}
			};

			match io::copy(&mut reader, &mut gz) {
				Ok(_) => match gz.finish().into_result() {
					Ok(compressed) => compressed,
					Err(e) => {
						dbg_msg!(e);
						return Err(e.to_string());
					}
				},
				Err(e) => {
					dbg_msg!(e);
					return Err(e.to_string());
				}
			}
		}

		CompressionType::Brotli => {
			// We may want to make the compression parameters configurable
			// in the future. For now, the defaults are sufficient.
			let brotli_params = BrotliEncoderParams::default();

			let mut compressed = Vec::<u8>::new();
			match BrotliCompress(&mut reader, &mut compressed, &brotli_params) {
				Ok(_) => compressed,
				Err(e) => {
					dbg_msg!(e);
					return Err(e.to_string());
				}
			}
		}

		// This arm is for any requested compressor for which we don't yet
		// have an implementation.
		_ => {
			let msg = "unsupported compressor".to_string();
			return Err(msg);
		}
	};

	Ok(compressed)
}

#[cfg(test)]
mod tests {
	use super::*;
	use brotli::Decompressor as BrotliDecompressor;
	use futures_lite::future::block_on;
	use lipsum::lipsum;
	use std::{boxed::Box, io};

	#[test]
	fn test_determine_compressor() {
		// Single compressor given.
		assert_eq!(determine_compressor("unsupported"), None);
		assert_eq!(determine_compressor("gzip"), Some(CompressionType::Gzip));
		assert_eq!(determine_compressor("*"), Some(DEFAULT_COMPRESSOR));

		// Multiple compressors.
		assert_eq!(determine_compressor("gzip, br"), Some(CompressionType::Brotli));
		assert_eq!(determine_compressor("gzip;q=0.8, br;q=0.3"), Some(CompressionType::Gzip));
		assert_eq!(determine_compressor("br, gzip"), Some(CompressionType::Brotli));
		assert_eq!(determine_compressor("br;q=0.3, gzip;q=0.4"), Some(CompressionType::Gzip));

		// Invalid q-values.
		assert_eq!(determine_compressor("gzip;q=NAN"), None);
	}

	#[test]
	fn test_compress_response() {
		// This macro generates an Accept-Encoding header value given any number of
		// compressors.
		macro_rules! ae_gen {
			($x:expr) => {
				$x.to_string().as_str()
			};

			($x:expr, $($y:expr),+) => {
				format!("{}, {}", $x.to_string(), ae_gen!($($y),+)).as_str()
			};
		}

		for accept_encoding in [
			"*",
			ae_gen!(CompressionType::Gzip),
			ae_gen!(CompressionType::Brotli, CompressionType::Gzip),
			ae_gen!(CompressionType::Brotli),
		] {
			// Determine what the expected encoding should be based on both the
			// specific encodings we accept.
			let expected_encoding: CompressionType = match determine_compressor(accept_encoding) {
				Some(s) => s,
				None => panic!("determine_compressor(accept_encoding) => None"),
			};

			// Build headers with our Accept-Encoding.
			let mut req_headers = HeaderMap::new();
			req_headers.insert(header::ACCEPT_ENCODING, header::HeaderValue::from_str(accept_encoding).unwrap());

			// Build test response.
			let lorem_ipsum: String = lipsum(10000);
			let expected_lorem_ipsum = Vec::<u8>::from(lorem_ipsum.as_str());
			let mut res = Response::builder()
				.status(200)
				.header(header::CONTENT_TYPE, "text/plain")
				.body(Body::from(lorem_ipsum))
				.unwrap();

			// Perform the compression.
			if let Err(e) = block_on(compress_response(req_headers, &mut res)) {
				panic!("compress_response(req_headers, &mut res) => Err(\"{}\")", e);
			};

			// If the content was compressed, we expect the Content-Encoding
			// header to be modified.
			assert_eq!(
				res
					.headers()
					.get(header::CONTENT_ENCODING)
					.unwrap_or_else(|| panic!("missing content-encoding header"))
					.to_str()
					.unwrap_or_else(|_| panic!("failed to convert Content-Encoding header::HeaderValue to String")),
				expected_encoding.to_string()
			);

			// Decompress body and make sure it's equal to what we started
			// with.
			//
			// In the case of no compression, just make sure the "new" body in
			// the Response is the same as what with which we start.
			let body_vec = match block_on(body::to_bytes(res.body_mut())) {
				Ok(b) => b.to_vec(),
				Err(e) => panic!("{}", e),
			};

			if expected_encoding == CompressionType::Passthrough {
				assert!(body_vec.eq(&expected_lorem_ipsum));
				continue;
			}

			// This provides an io::Read for the underlying body.
			let mut body_cursor: io::Cursor<Vec<u8>> = io::Cursor::new(body_vec);

			// Match the appropriate decompresor for the given
			// expected_encoding.
			let mut decoder: Box<dyn io::Read> = match expected_encoding {
				CompressionType::Gzip => match gzip::Decoder::new(&mut body_cursor) {
					Ok(dgz) => Box::new(dgz),
					Err(e) => panic!("{}", e),
				},

				CompressionType::Brotli => Box::new(BrotliDecompressor::new(body_cursor, expected_lorem_ipsum.len())),

				_ => panic!("no decompressor for {}", expected_encoding.to_string()),
			};

			let mut decompressed = Vec::<u8>::new();
			match io::copy(&mut decoder, &mut decompressed) {
				Ok(_) => {}
				Err(e) => panic!("{}", e),
			};

			assert!(decompressed.eq(&expected_lorem_ipsum));
		}
	}
}
