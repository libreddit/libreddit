use actix_web::{client::Client, error, web, Error, HttpResponse, Result};
use url::Url;

use base64::decode;

pub async fn handler(web::Path(b64): web::Path<String>) -> Result<HttpResponse> {
	let domains = vec![
		"a.thumbs.redditmedia.com",
		"b.thumbs.redditmedia.com",
		"preview.redd.it",
		"external-preview.redd.it",
		"i.redd.it",
		"v.redd.it",
	];

	match decode(b64) {
		Ok(bytes) => {
			let media = String::from_utf8(bytes).unwrap();

			match Url::parse(media.as_str()) {
				Ok(url) => {
					let domain = url.domain().unwrap_or_default();

					if domains.contains(&domain) {
						Client::default()
							.get(media.replace("&amp;", "&"))
							.send()
							.await
							.map_err(Error::from)
							.map(|res| HttpResponse::build(res.status()).streaming(res))
					} else {
						Err(error::ErrorForbidden("Resource must be from Reddit"))
					}
				}
				Err(_) => Err(error::ErrorBadRequest("Can't parse encoded base64 URL")),
			}
		}
		Err(_) => Err(error::ErrorBadRequest("Can't decode base64 URL")),
	}
}
