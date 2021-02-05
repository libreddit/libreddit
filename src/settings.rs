// CRATES
use crate::utils::{prefs, Preferences};
use tide::{Request, Response, http::Cookie};
use askama::Template;
use time::{Duration, OffsetDateTime};

// STRUCTS
#[derive(Template)]
#[template(path = "settings.html")]
struct SettingsTemplate {
	prefs: Preferences,
}

#[derive(serde::Serialize, serde::Deserialize, Default, Debug)]
pub struct SettingsForm {
	theme: Option<String>,
	front_page: Option<String>,
	layout: Option<String>,
	wide: Option<String>,
	comment_sort: Option<String>,
	show_nsfw: Option<String>,
}

// FUNCTIONS

// Retrieve cookies from request "Cookie" header
pub async fn get(req: Request<()>) -> tide::Result {
	let s = SettingsTemplate { prefs: prefs(req) }.render().unwrap();
	Ok(Response::builder(200).content_type("text/html").body(s).build())
}

// Set cookies using response "Set-Cookie" header
pub async fn set(mut req: Request<()>) -> tide::Result {
	let form: SettingsForm = req.body_form().await.unwrap_or_default();

	let mut res = Response::builder(302)
		.content_type("text/html")
		.header("Location", "/settings")
		.body(r#"Redirecting to <a href="/settings">settings</a>..."#)
		.build();

	let names = vec!["theme", "front_page", "layout", "wide", "comment_sort", "show_nsfw"];
	let values = vec![form.theme, form.front_page, form.layout, form.wide, form.comment_sort, form.show_nsfw];

	for (i, name) in names.iter().enumerate() {
		match values.get(i) {
			Some(value) => res.insert_cookie(
				Cookie::build(name.to_owned(), value.to_owned().unwrap_or_default())
					.path("/")
					.http_only(true)
					.expires(OffsetDateTime::now_utc() + Duration::weeks(52))
					.finish()
			),
			None => res.remove_cookie(Cookie::named(name.to_owned())),
		};
	}

	Ok(res)
}
