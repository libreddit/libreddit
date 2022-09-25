use cookie::Cookie;
use futures_lite::{future::Boxed, Future, FutureExt};
use hyper::{
	header::HeaderValue,
	service::{make_service_fn, service_fn},
	HeaderMap,
};
use hyper::{Body, Method, Request, Response, Server as HyperServer};
use route_recognizer::{Params, Router};
use std::{pin::Pin, result::Result};
use time::Duration;

type BoxResponse = Pin<Box<dyn Future<Output = Result<Response<Body>, String>> + Send>>;

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
		if let Ok(val) = HeaderValue::from_str(&cookie.to_string()) {
			self.headers_mut().append("Set-Cookie", val);
		}
	}

	fn remove_cookie(&mut self, name: String) {
		let mut cookie = Cookie::named(name);
		cookie.set_path("/");
		cookie.set_max_age(Duration::seconds(1));
		if let Ok(val) = HeaderValue::from_str(&cookie.to_string()) {
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
		Self {
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
					let headers = default_headers.clone();

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
								let res: Result<Response<Body>, String> = func.await;
								// Add default headers to response
								res.map(|mut response| {
									response.headers_mut().extend(headers);
									response
								})
							}
							.boxed()
						}
						// If there was a routing error
						Err(e) => async move {
							// Return a 404 error
							let res: Result<Response<Body>, String> = Ok(Response::builder().status(404).body(e.into()).unwrap_or_default());
							// Add default headers to response
							res.map(|mut response| {
								response.headers_mut().extend(headers);
								response
							})
						}
						.boxed(),
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
