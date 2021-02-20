use surf::Body;
use tide::{Request, Response};

pub async fn handler(req: Request<()>, format: &str, params: Vec<&str>) -> tide::Result {
	let mut url = format.to_string();

	for name in params {
		let param = req.param(name).unwrap_or_default();
		url = url.replacen("{}", param, 1);
	}

	request(url).await
}

async fn request(url: String) -> tide::Result {
	match surf::get(url).await {
		Ok(res) => {
			let content_length = res.header("Content-Length").map(|v| v.to_string()).unwrap_or_default();
			let content_type = res.content_type().map(|m| m.to_string()).unwrap_or_default();

			Ok(
				Response::builder(res.status())
					.body(Body::from_reader(res, None))
					.header("Cache-Control", "public, max-age=1209600, s-maxage=86400")
					.header("Content-Length", content_length)
					.header("Content-Type", content_type)
					.build(),
			)
		}
		Err(e) => Ok(Response::builder(503).body(e.to_string()).build()),
	}
}
