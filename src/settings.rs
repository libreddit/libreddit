// CRATES
use crate::utils::{redirect, template, Preferences};
use askama::Template;
use tide::{http::Cookie, Request};
use time::{Duration, OffsetDateTime};

// STRUCTS
#[derive(Template)]
#[template(path = "settings.html")]
struct SettingsTemplate {
	prefs: Preferences,
}

#[derive(serde::Deserialize, Default)]
#[serde(default)]
pub struct SettingsForm {
	theme: Option<String>,
	front_page: Option<String>,
	layout: Option<String>,
	wide: Option<String>,
	comment_sort: Option<String>,
	show_nsfw: Option<String>,
	redirect: Option<String>,
	subscriptions: Option<String>,
}

// FUNCTIONS

// Retrieve cookies from request "Cookie" header
pub async fn get(req: Request<()>) -> tide::Result {
	template(SettingsTemplate { prefs: Preferences::new(req) })
}

// Set cookies using response "Set-Cookie" header
pub async fn set(mut req: Request<()>) -> tide::Result {
	let form: SettingsForm = req.body_form().await.unwrap_or_default();

	let mut res = redirect("/settings".to_string());

	let names = vec!["theme", "front_page", "layout", "wide", "comment_sort", "show_nsfw"];
	let values = vec![form.theme, form.front_page, form.layout, form.wide, form.comment_sort, form.show_nsfw];

	for (i, name) in names.iter().enumerate() {
		match values.get(i) {
			Some(value) => res.insert_cookie(
				Cookie::build(name.to_owned(), value.to_owned().unwrap_or_default())
					.path("/")
					.http_only(true)
					.expires(OffsetDateTime::now_utc() + Duration::weeks(52))
					.finish(),
			),
			None => res.remove_cookie(Cookie::named(name.to_owned())),
		};
	}

	Ok(res)
}

// Set cookies using response "Set-Cookie" header
pub async fn restore(req: Request<()>) -> tide::Result {
	let form: SettingsForm = req.query()?;

	let path = match form.redirect {
		Some(value) => format!("/{}/", value),
		None => "/".to_string(),
	};

	let mut res = redirect(path);

	let names = vec!["theme", "front_page", "layout", "wide", "comment_sort", "show_nsfw", "subscriptions"];
	let values = vec![form.theme, form.front_page, form.layout, form.wide, form.comment_sort, form.show_nsfw, form.subscriptions];

	for (i, name) in names.iter().enumerate() {
		match values.get(i) {
			Some(value) => res.insert_cookie(
				Cookie::build(name.to_owned(), value.to_owned().unwrap_or_default())
					.path("/")
					.http_only(true)
					.expires(OffsetDateTime::now_utc() + Duration::weeks(52))
					.finish(),
			),
			None => res.remove_cookie(Cookie::named(name.to_owned())),
		};
	}

	Ok(res)
}
