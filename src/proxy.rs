use actix_web::{client::Client, get, web, Error, HttpResponse, Result};

#[cfg(feature = "proxy")]
use percent_encoding::percent_decode_str;

#[get("/imageproxy/{url:.*}")]
async fn handler(web::Path(url): web::Path<String>) -> Result<HttpResponse> {
	if cfg!(feature = "proxy") {
		#[cfg(feature = "proxy")]
		let media: String = percent_decode_str(url.as_str()).decode_utf8()?.to_string();

		#[cfg(not(feature = "proxy"))]
		let media: String = url;

		dbg!(&media);

		let client = Client::default();
		client
			.get(media)
			.send()
			.await
			.map_err(Error::from)
			.and_then(|res| Ok(HttpResponse::build(res.status()).streaming(res)))
	} else {
		Ok(HttpResponse::Ok().body(""))
	}
}
