use std::collections::HashMap;

// CRATES
use crate::server::{ RequestExt, ResponseExt };
use crate::utils::{get_saved_posts, redirect, template, Post, Preferences};
use askama::Template;
use cookie::Cookie;
use hyper::{Body, Request, Response};
use time::{Duration, OffsetDateTime};

// STRUCTS
#[derive(Template)]
#[template(path = "saved.html")]
struct SavedTemplate {
	posts: Vec<Post>,
    prefs: Preferences,
    saved: Vec<String>,
	url: String,
}

// FUNCTIONS

pub async fn get(req: Request<Body>) -> Result<Response<Body>, String> {
    let saved = get_saved_posts(&req);
    let full_names: Vec<String> = saved.iter().map(|id| format!("t3_{}", id)).collect();
    let path = format!("/api/info.json?id={}", full_names.join(","));
    let posts;
    match Post::fetch(&path, false).await {
        Ok((post_results, _after)) => posts = post_results,
        Err(_) => posts = vec![],
    }
    // let posts = vec![];
    let url = req.uri().to_string();
    template(SavedTemplate{
        posts,
        prefs: Preferences::new(req),
        saved,
        url,
    })
}

pub async fn save(req: Request<Body>) -> Result<Response<Body>, String> {
	// Get existing cookie
    let mut saved_posts = get_saved_posts(&req);

	let query = req.uri().query().unwrap_or_default().as_bytes();
	let form = url::form_urlencoded::parse(query).collect::<HashMap<_, _>>();

    let path = match form.get("redirect") {
		Some(value) => format!("{}", value.replace("%26", "&").replace("%23", "#")),
        None => "saved".to_string(),
    };

    let mut response = redirect(path);

	match req.param("id") {
        Some(id) => {
            saved_posts.push(id);
            response.insert_cookie(
            Cookie::build(String::from("saved_posts"), saved_posts.join("+"))
                .path("/")
                .http_only(true)
                .expires(OffsetDateTime::now_utc() + Duration::weeks(52))
                .finish(),
            );
            Ok(response)
        },
        None => Ok(response),
    }
}

pub async fn unsave(req: Request<Body>) -> Result<Response<Body>, String> {
	// Get existing cookie
    let mut saved_posts = get_saved_posts(&req);

	let query = req.uri().query().unwrap_or_default().as_bytes();
	let form = url::form_urlencoded::parse(query).collect::<HashMap<_, _>>();

    let path = match form.get("redirect") {
		Some(value) => format!("{}", value.replace("%26", "&").replace("%23", "#")),
        None => "saved".to_string(),
    };

    let mut response = redirect(path);

	match req.param("id") {
        Some(id) => {
            if let Some(index) = saved_posts.iter().position(|el| el == &id) {
                saved_posts.remove(index);
            }
            response.insert_cookie(
            Cookie::build(String::from("saved_posts"), saved_posts.join("+"))
                .path("/")
                .http_only(true)
                .expires(OffsetDateTime::now_utc() + Duration::weeks(52))
                .finish(),
            );
            Ok(response)
        },
        None => Ok(response),
    }
}
