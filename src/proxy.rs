use async_std::{io, net::TcpStream, prelude::*};
use async_tls::TlsConnector;
use tide::{http::url::Url, Request, Response};

/// Handle tide routes to proxy by parsing `params` from `req`uest.
pub async fn handler(req: Request<()>, format: &str, params: Vec<&str>) -> tide::Result {
	let mut url = format.to_string();

	for name in params {
		let param = req.param(name).unwrap_or_default();
		url = url.replacen("{}", param, 1);
	}

	request(url).await
}

/// Sends a request to a Reddit media domain and proxy the response.
///
/// Relays the `Content-Length` and `Content-Type` header.
async fn request(url: String) -> tide::Result {
	// Parse url into parts
	let parts = Url::parse(&url).unwrap();
	let host = parts.host().unwrap().to_string();
	let domain = parts.domain().unwrap_or_default();
	let path = format!("{}?{}", parts.path(), parts.query().unwrap_or_default());
	// Build reddit-compliant user agent for Libreddit
	let user_agent = format!("web:libreddit:{}", env!("CARGO_PKG_VERSION"));

	// Construct a request body
	let req = format!(
		"GET {} HTTP/1.1\r\nHost: {}\r\nAccept: */*\r\nConnection: close\r\nUser-Agent: {}\r\n\r\n",
		path, host, user_agent
	);

	// Initialize TLS connector for requests
	let connector = TlsConnector::default();

	// Open a TCP connection
	let tcp_stream = TcpStream::connect(format!("{}:443", domain)).await.unwrap();

	// Use the connector to start the handshake process
	let mut tls_stream = connector.connect(domain, tcp_stream).await.unwrap();

	// Write the aforementioned HTTP request to the stream
	tls_stream.write_all(req.as_bytes()).await.unwrap();

	// And read the response
	let mut writer = Vec::new();
	io::copy(&mut tls_stream, &mut writer).await.unwrap();

	// Find the delimiter which separates the body and headers
	match (0..writer.len()).find(|i| writer[i.to_owned()] == 10_u8 && writer[i - 2] == 10_u8) {
		Some(delim) => {
			// Split the response into the body and headers
			let split = writer.split_at(delim);
			let headers_str = String::from_utf8_lossy(split.0);
			let headers = headers_str.split("\r\n").collect::<Vec<&str>>();
			let body = split.1[1..split.1.len()].to_vec();

			// Parse the status code from the first header line
			let status: u16 = headers[0].split(' ').collect::<Vec<&str>>()[1].parse().unwrap_or_default();

			// Define a closure for easier header fetching
			let header = |name: &str| {
				headers
					.iter()
					.find(|x| x.starts_with(name))
					.map(|f| f.split(": ").collect::<Vec<&str>>()[1])
					.unwrap_or_default()
			};

			// Parse Content-Length and Content-Type from headers
			let content_length = header("Content-Length");
			let content_type = header("Content-Type");

			// Build response
			Ok(
				Response::builder(status)
					.body(tide::http::Body::from_bytes(body))
					.header("Cache-Control", "public, max-age=1209600, s-maxage=86400")
					.header("Content-Length", content_length)
					.header("Content-Type", content_type)
					.build(),
			)
		}
		None => Ok(Response::builder(503).body("Couldn't parse media".to_string()).build()),
	}
}
