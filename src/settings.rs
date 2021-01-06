// CRATES
use crate::utils::cookie;
use actix_web::{cookie::Cookie, web::Form, HttpRequest, HttpResponse}; // http::Method,
use askama::Template;
use time::{Duration, OffsetDateTime};

// STRUCTS
#[derive(Template)]
#[template(path = "settings.html", escape = "none")]
struct SettingsTemplate {
	layout: String,
}

#[derive(serde::Deserialize)]
pub struct Preferences {
	layout: Option<String>,
}

// FUNCTIONS

// Retrieve cookies from request "Cookie" header
pub async fn get(req: HttpRequest) -> HttpResponse {
	let s = SettingsTemplate { layout: cookie(req, "layout") }.render().unwrap();

	HttpResponse::Ok().content_type("text/html").body(s)
}

// Set cookies using response "Set-Cookie" header
pub async fn set(req: HttpRequest, form: Form<Preferences>) -> HttpResponse {
	let mut response = HttpResponse::Found();

	match &form.layout {
		Some(value) => response.cookie(
			Cookie::build("layout", value)
				.path("/")
				.secure(true)
				.http_only(true)
				.expires(OffsetDateTime::now_utc() + Duration::weeks(52))
				.finish(),
		),
		None => response.del_cookie(&actix_web::HttpMessage::cookie(&req, "layout").unwrap()),
	};

	response
		.content_type("text/html")
		.set_header("Location", "/settings")
		.body(r#"Redirecting to <a href="/settings">settings</a>..."#)
}
