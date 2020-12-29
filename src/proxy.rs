use actix_web::{client::Client, web, Error, HttpResponse, Result};

#[cfg(feature = "proxy")]
use base64::decode;

pub async fn handler(web::Path(url): web::Path<String>) -> Result<HttpResponse> {
	if cfg!(feature = "proxy") {
		#[cfg(feature = "proxy")]
		let media: String;

		#[cfg(not(feature = "proxy"))]
		let media = url;

		#[cfg(feature = "proxy")]
		match decode(url) {
			Ok(bytes) => media = String::from_utf8(bytes).unwrap(),
			Err(_e) => return Ok(HttpResponse::Ok().body("")),
		};

		let client = Client::default();
		client
			.get(media.replace("&amp;", "&"))
			.send()
			.await
			.map_err(Error::from)
			.and_then(|res| Ok(HttpResponse::build(res.status()).streaming(res)))
	} else {
		Ok(HttpResponse::Ok().body(""))
	}
}
