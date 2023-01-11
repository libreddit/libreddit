use crate::{
	config::{Config, CONFIG},
	server::RequestExt,
	utils::Preferences,
};
use askama::Template;
use build_html::{Container, Html, HtmlContainer, Table};
use hyper::{http::Error, Body, Request, Response};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

// This is the local static that is intialized at runtime (technically at
// the first request *to the instance-info endpoint) and contains the data
// retrieved from the instance-info endpoint.
pub(crate) static INSTANCE_INFO: Lazy<InstanceInfo> = Lazy::new(InstanceInfo::new);

/// Handles instance info endpoint
pub async fn instance_info(req: Request<Body>) -> Result<Response<Body>, String> {
	// This will retrieve the extension given, or create a new string - which will
	// simply become the last option, an HTML page.
	let extension = req.param("extension").unwrap_or(String::new());
	let response = match extension.as_str() {
		"yaml" => info_yaml(),
		"txt" => info_txt(),
		"json" => info_json(),
		"html" | _ => info_html(req),
	};
	response.map_err(|err| format!("{err}"))
}

fn info_json() -> Result<Response<Body>, Error> {
	let body = serde_json::to_string(&*INSTANCE_INFO).unwrap_or("Error serializing JSON.".into());
	Response::builder().status(200).header("content-type", "application/json").body(body.into())
}

fn info_yaml() -> Result<Response<Body>, Error> {
	let body = serde_yaml::to_string(&*INSTANCE_INFO).unwrap_or("Error serializing YAML.".into());
	// https://github.com/ietf-wg-httpapi/mediatypes/blob/main/draft-ietf-httpapi-yaml-mediatypes.md
	Response::builder().status(200).header("content-type", "application/yaml").body(body.into())
}

fn info_txt() -> Result<Response<Body>, Error> {
	Response::builder()
		.status(200)
		.header("content-type", "text/plain")
		.body(INSTANCE_INFO.to_string(StringType::Raw).into())
}
fn info_html(req: Request<Body>) -> Result<Response<Body>, Error> {
	let message = MessageTemplate {
		title: String::from("Instance information"),
		body: INSTANCE_INFO.to_string(StringType::Html),
		prefs: Preferences::new(&req),
		url: req.uri().to_string(),
	}
	.render()
	.unwrap();
	Response::builder().status(200).header("content-type", "text/html; charset=utf8").body(message.into())
}
#[derive(Serialize, Deserialize, Default)]
pub(crate) struct InstanceInfo {
	crate_version: String,
	git_commit: String,
	deploy_date: String,
	compile_mode: String,
	deploy_unix_ts: i64,
	config: Config,
}

impl InstanceInfo {
	pub fn new() -> Self {
		Self {
			crate_version: env!("CARGO_PKG_VERSION").to_string(),
			git_commit: env!("GIT_HASH").to_string(),
			deploy_date: OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc()).to_string(),
			#[cfg(debug_assertions)]
			compile_mode: "Debug".into(),
			#[cfg(not(debug_assertions))]
			compile_mode: "Release".into(),
			deploy_unix_ts: OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc()).unix_timestamp(),
			config: CONFIG.clone(),
		}
	}
	fn to_table(&self) -> String {
		let mut container = Container::default();
		let convert = |o: &Option<String>| -> String { o.clone().unwrap_or("Unset".to_owned()) };
		container.add_header(3, "Instance banner");
		container.add_raw("<br />");
		container.add_paragraph(convert(&self.config.banner));
		container.add_raw("<br />");
		container.add_table(Table::from([
			["Crate version", &self.crate_version],
			["Git commit", &self.git_commit],
			["Deploy date", &self.deploy_date],
			["Deploy timestamp", &self.deploy_unix_ts.to_string()],
			["Compile mode", &self.compile_mode],
			["<b>Settings</b>", "<b>Settings</b>"],
			["SFW only", &convert(&self.config.sfw_only)],
			["Hide awards", &convert(&self.config.default_hide_awards)],
			["Default theme", &convert(&self.config.default_theme)],
			["Default front page", &convert(&self.config.default_front_page)],
			["Default layout", &convert(&self.config.default_layout)],
			["Default wide", &convert(&self.config.default_wide)],
			["Default comment sort", &convert(&self.config.default_comment_sort)],
			["Default post sort", &convert(&self.config.default_post_sort)],
			["Default show NSFW", &convert(&self.config.default_show_nsfw)],
			["Default blur NSFW", &convert(&self.config.default_blur_nsfw)],
			["Default use HLS", &convert(&self.config.default_use_hls)],
			["Default hide HLS notification", &convert(&self.config.default_hide_hls_notification)],
		]));
		container.to_html_string()
	}
	fn to_string(&self, string_type: StringType) -> String {
		match string_type {
			StringType::Raw => {
				format!(
					"Crate version: {}\n
                Git commit: {}\n
                Deploy date: {}\n
                Deploy timestamp: {}\n
                Compile mode: {}\n
                Config:\n
                    Banner: {:?}\n
                    Hide awards: {:?}\n
                    SFW only: {:?}\n
                    Default theme: {:?}\n
                    Default front page: {:?}\n
                    Default layout: {:?}\n
                    Default wide: {:?}\n
                    Default comment sort: {:?}\n
                    Default post sort: {:?}\n
                    Default show NSFW: {:?}\n
                    Default blur NSFW: {:?}\n
                    Default use HLS: {:?}\n
                    Default hide HLS notification: {:?}\n",
					self.crate_version,
					self.git_commit,
					self.deploy_date,
					self.deploy_unix_ts,
					self.compile_mode,
					self.config.banner,
					self.config.default_hide_awards,
					self.config.sfw_only,
					self.config.default_theme,
					self.config.default_front_page,
					self.config.default_layout,
					self.config.default_wide,
					self.config.default_comment_sort,
					self.config.default_post_sort,
					self.config.default_show_nsfw,
					self.config.default_blur_nsfw,
					self.config.default_use_hls,
					self.config.default_hide_hls_notification
				)
			}
			StringType::Html => self.to_table(),
		}
	}
}
enum StringType {
	Raw,
	Html,
}
#[derive(Template)]
#[template(path = "message.html")]
struct MessageTemplate {
	title: String,
	body: String,
	prefs: Preferences,
	url: String,
}
