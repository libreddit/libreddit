use std::collections::HashMap;

// CRATES
use crate::server::ResponseExt;
use crate::utils::{redirect, template, Preferences};
use askama::Template;
use cookie::Cookie;
use futures_lite::StreamExt;
use hyper::{Body, Request, Response};
use time::{Duration, OffsetDateTime};

// STRUCTS
#[derive(Template)]
#[template(path = "settings.html")]
struct SettingsTemplate {
	prefs: Preferences,
}

// FUNCTIONS

// Retrieve cookies from request "Cookie" header
pub async fn get(req: Request<Body>) -> Result<Response<Body>, String> {
	template(SettingsTemplate { prefs: Preferences::new(req) })
}

// Set cookies using response "Set-Cookie" header
pub async fn set(req: Request<Body>) -> Result<Response<Body>, String> {
	// Split the body into parts
	let (parts, mut body) = req.into_parts();

	// Grab existing cookies
	let mut cookies = Vec::new();
	for header in parts.headers.get_all("Cookie") {
		if let Ok(cookie) = Cookie::parse(header.to_str().unwrap_or_default()) {
			cookies.push(cookie);
		}
	}

	// Aggregate the body...
	// let whole_body = hyper::body::aggregate(req).await.map_err(|e| e.to_string())?;
	let body_bytes = body
		.try_fold(Vec::new(), |mut data, chunk| {
			data.extend_from_slice(&chunk);
			Ok(data)
		})
		.await
		.map_err(|e| e.to_string())?;

	let form = url::form_urlencoded::parse(&body_bytes).collect::<HashMap<_, _>>();

	let mut res = redirect("/settings".to_string());

	let names = vec![
		"theme",
		"front_page",
		"layout",
		"wide",
		"comment_sort",
		"post_sort",
		"show_nsfw",
		"use_hls",
		"hide_hls_notification",
	];

	for name in names {
		match form.get(name) {
			Some(value) => res.insert_cookie(
				Cookie::build(name.to_owned(), value.to_owned())
					.path("/")
					.http_only(true)
					.expires(OffsetDateTime::now_utc() + Duration::weeks(52))
					.finish(),
			),
			None => res.remove_cookie(name.to_string()),
		};
	}

	Ok(res)
}

fn set_cookies_method(req: Request<Body>, remove_cookies: bool) -> Response<Body> {
	// Split the body into parts
	let (parts, _) = req.into_parts();

	// Grab existing cookies
	let mut cookies = Vec::new();
	for header in parts.headers.get_all("Cookie") {
		if let Ok(cookie) = Cookie::parse(header.to_str().unwrap_or_default()) {
			cookies.push(cookie);
		}
	}

	let query = parts.uri.query().unwrap_or_default().as_bytes();

	let form = url::form_urlencoded::parse(query).collect::<HashMap<_, _>>();

	let names = vec![
		"theme",
		"front_page",
		"layout",
		"wide",
		"comment_sort",
		"post_sort",
		"show_nsfw",
		"use_hls",
		"hide_hls_notification",
		"subscriptions",
	];

	let mut path = match form.get("redirect") {
		Some(value) => format!("/{}", value.replace("%26", "&").replace("%23", "#")),
		None => "/".to_string(),
	};

	if path.find('#') == None {
		path += "/";
	}

	let mut res = redirect(path);

	for name in names {
		match form.get(name) {
			Some(value) => res.insert_cookie(
				Cookie::build(name.to_owned(), value.to_owned())
					.path("/")
					.http_only(true)
					.expires(OffsetDateTime::now_utc() + Duration::weeks(52))
					.finish(),
			),
			None => {
				if remove_cookies {
					res.remove_cookie(name.to_string())
				}
			}
		};
	}

	res
}

// Set cookies using response "Set-Cookie" header
pub async fn restore(req: Request<Body>) -> Result<Response<Body>, String> {
	Ok(set_cookies_method(req, true))
}

pub async fn update(req: Request<Body>) -> Result<Response<Body>, String> {
	Ok(set_cookies_method(req, false))
}
