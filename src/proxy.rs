use base64::decode;
use surf::{Body, Url};
use tide::{Request, Response};

pub async fn handler(req: Request<()>) -> tide::Result {
	let domains = vec![
		// EMOJI
		"emoji.redditmedia.com",
		// ICONS
		"styles.redditmedia.com",
		"www.redditstatic.com",
		// PREVIEWS
		"preview.redd.it",
		"external-preview.redd.it",
	];

	let decoded = decode(req.param("url").unwrap_or_default()).map(|bytes| String::from_utf8(bytes).unwrap_or_default());

	match decoded {
		Ok(media) => match Url::parse(media.as_str()) {
			Ok(url) => {
				if domains.contains(&url.domain().unwrap_or_default()) {
					request(url.to_string()).await
				} else {
					Err(tide::Error::from_str(403, "Resource must be from Reddit"))
				}
			}
			Err(_) => Err(tide::Error::from_str(400, "Can't parse base64 into URL")),
		},
		Err(_) => Err(tide::Error::from_str(400, "Can't decode base64")),
	}
}

pub async fn video(req: Request<()>) -> tide::Result {
	let id = req.param("id").unwrap_or_default();
	let size = req.param("size").unwrap_or("720.mp4");
	let url = format!("https://v.redd.it/{}/DASH_{}", id, size);
	request(url).await
}

pub async fn image(req: Request<()>) -> tide::Result {
	let id = req.param("id").unwrap_or_default();
	let url = format!("https://i.redd.it/{}", id);
	request(url).await
}

pub async fn thumbnail(req: Request<()>) -> tide::Result {
	let id = req.param("id").unwrap_or_default();
	let point = req.param("point").unwrap_or_default();
	let url = format!("https://{}.thumbs.redditmedia.com/{}", point, id);
	request(url).await
}

async fn request(url: String) -> tide::Result {
	let http = surf::get(url).await.unwrap();

	let content_length = http.header("Content-Length").map(|v| v.to_string()).unwrap_or_default();
	let content_type = http.content_type().map(|m| m.to_string()).unwrap_or_default();

	Ok(
		Response::builder(http.status())
			.body(Body::from_reader(http, None))
			.header("Cache-Control", "public, max-age=1209600, s-maxage=86400")
			.header("Content-Length", content_length)
			.header("Content-Type", content_type)
			.build(),
	)
}
