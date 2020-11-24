use actix_web::{get, web, HttpResponse, Result, client::Client, Error};

#[get("/imageproxy/{url:.*}")]
async fn handler(web::Path(url): web::Path<String>) -> Result<HttpResponse> {
	if cfg!(feature = "proxy") {
		dbg!(&url);
		let client = Client::default();
		client.get(url)
			.send()
			.await
			.map_err(Error::from)
			.and_then(|res| {
				Ok(HttpResponse::build(res.status()).streaming(res))
			})
	} else {
		Ok(HttpResponse::Ok().body(""))
	}
}