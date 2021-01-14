use actix_web::{client::Client, error, web, Error, HttpResponse, Result};
use url::Url;

use base64::decode;

pub async fn handler(web::Path(b64): web::Path<String>) -> Result<HttpResponse> {
	let domains = vec![
		// THUMBNAILS
		"a.thumbs.redditmedia.com",
		"b.thumbs.redditmedia.com",
		// EMOJI
		"emoji.redditmedia.com",
		// ICONS
		"styles.redditmedia.com",
		"www.redditstatic.com",
		// PREVIEWS
		"preview.redd.it",
		"external-preview.redd.it",
		// MEDIA
		"i.redd.it",
		"v.redd.it",
	];

	match decode(b64) {
		Ok(bytes) => {
			let media = String::from_utf8(bytes).unwrap_or_default();

			match Url::parse(media.as_str()) {
				Ok(url) => {
					let domain = url.domain().unwrap_or_default();

					if domains.contains(&domain) {
						Client::default().get(media.replace("&amp;", "&")).send().await.map_err(Error::from).map(|res| {
							HttpResponse::build(res.status())
								.header("Cache-Control", "public, max-age=1209600, s-maxage=86400")
								.header("Content-Length", res.headers().get("Content-Length").unwrap().to_owned())
								.header("Content-Type", res.headers().get("Content-Type").unwrap().to_owned())
								.streaming(res)
						})
					} else {
						Err(error::ErrorForbidden("Resource must be from Reddit"))
					}
				}
				_ => Err(error::ErrorBadRequest("Can't parse base64 into URL")),
			}
		}
		_ => Err(error::ErrorBadRequest("Can't decode base64")),
	}
}
