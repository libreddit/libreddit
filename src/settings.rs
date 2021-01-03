// // CRATES
// use crate::utils::cookies;
// use actix_web::{cookie::Cookie, web::Form, HttpRequest, HttpResponse, Result}; // http::Method,
// use askama::Template;

// // STRUCTS
// #[derive(Template)]
// #[template(path = "settings.html", escape = "none")]
// struct SettingsTemplate {
// 	pref_nsfw: String,
// }

// #[derive(serde::Deserialize)]
// pub struct Preferences {
// 	pref_nsfw: Option<String>,
// }

// // FUNCTIONS

// // Retrieve cookies from request "Cookie" header
// pub async fn get(req: HttpRequest) -> Result<HttpResponse> {
// 	let cookies = cookies(req);

// 	let pref_nsfw: String = cookies.get("pref_nsfw").unwrap_or(&String::new()).to_owned();

// 	let s = SettingsTemplate { pref_nsfw }.render().unwrap();
// 	Ok(HttpResponse::Ok().content_type("text/html").body(s))
// }

// // Set cookies using response "Set-Cookie" header
// pub async fn set(form: Form<Preferences>) -> HttpResponse {
// 	let nsfw: Cookie = match &form.pref_nsfw {
// 		Some(value) => Cookie::build("pref_nsfw", value).path("/").secure(true).http_only(true).finish(),
// 		None => Cookie::build("pref_nsfw", "").finish(),
// 	};

// 	let body = SettingsTemplate {
// 		pref_nsfw: form.pref_nsfw.clone().unwrap_or_default(),
// 	}
// 	.render()
// 	.unwrap();

// 	HttpResponse::Found()
// 		.content_type("text/html")
// 		.set_header("Set-Cookie", nsfw.to_string())
// 		.set_header("Location", "/settings")
// 		.body(body)
// }
