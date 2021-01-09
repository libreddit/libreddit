// CRATES
use crate::utils::cookie;
use actix_web::{cookie::Cookie, web::Form, HttpMessage, HttpRequest, HttpResponse};
use askama::Template;
use time::{Duration, OffsetDateTime};

// STRUCTS
#[derive(Template)]
#[template(path = "settings.html")]
struct SettingsTemplate {
	layout: String,
	comment_sort: String,
	hide_nsfw: String,
}

#[derive(serde::Deserialize)]
pub struct SettingsForm {
	layout: Option<String>,
	comment_sort: Option<String>,
	hide_nsfw: Option<String>,
}

// FUNCTIONS

// Retrieve cookies from request "Cookie" header
pub async fn get(req: HttpRequest) -> HttpResponse {
	let s = SettingsTemplate {
		layout: cookie(req.to_owned(), "layout"),
		comment_sort: cookie(req.to_owned(), "comment_sort"),
		hide_nsfw: cookie(req, "hide_nsfw"),
	}
	.render()
	.unwrap();
	HttpResponse::Ok().content_type("text/html").body(s)
}

// Set cookies using response "Set-Cookie" header
pub async fn set(req: HttpRequest, form: Form<SettingsForm>) -> HttpResponse {
	let mut res = HttpResponse::Found();

	let names = vec!["layout", "comment_sort", "hide_nsfw"];
	let values = vec![&form.layout, &form.comment_sort, &form.hide_nsfw];

	for (i, name) in names.iter().enumerate() {
		match values[i] {
			Some(value) => res.cookie(
				Cookie::build(name.to_owned(), value)
					.path("/")
					.http_only(true)
					.expires(OffsetDateTime::now_utc() + Duration::weeks(52))
					.finish(),
			),
			None => match HttpMessage::cookie(&req, name.to_owned()) {
				Some(cookie) => res.del_cookie(&cookie),
				None => &mut res,
			},
		};
	}

	res
		.content_type("text/html")
		.set_header("Location", "/settings")
		.body(r#"Redirecting to <a href="/settings">settings</a>..."#)
}
