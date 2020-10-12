// Load macros
#![feature(proc_macro_hygiene, decl_macro)]

// Load Rocket
#[macro_use] extern crate rocket;
use rocket_contrib::templates::Template;

// Reference local files
mod subreddit;
mod popular;
mod user;
mod post;

// Favicon

#[get("/favicon.ico")]
fn favicon() -> String { String::new() }

// Main function
fn main() {
	let routes = routes![
		favicon,
		popular::page,
		popular::sorted,
		subreddit::page,
		subreddit::sorted,
		user::page,
		user::sorted,
		post::page,
		post::sorted
	];

	rocket::ignite().mount("/", routes).attach(Template::fairing()).launch();
}