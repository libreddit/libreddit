use hyper::{http::Error, Body, Request, Response};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{
	config::{Config, CONFIG},
	server::RequestExt,
};

// This is the local static that is intialized at runtime (technically at
// the first request *to the instance-info endpoint) and contains the data
// retrieved from the instance-info endpoint.
pub(crate) static INSTANCE_INFO: Lazy<InstanceInfo> = Lazy::new(InstanceInfo::new);

/// Handles instance info endpoint
pub async fn instance_info(req: Request<Body>) -> Result<Response<Body>, String> {
	let extension = req.param("extension").unwrap_or("json".into());
	let response = match extension.as_str() {
		"yaml" => info_yaml(),
		"txt" => info_txt(),
		"json" | _ => info_json(),
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
		.body((INSTANCE_INFO.to_string()).into())
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
}
impl ToString for InstanceInfo {
	fn to_string(&self) -> String {
		format!(
			"Crate version: {}\n
                Git commit: {}\n
                Deploy date: {}\n
                Deploy timestamp: {}\n
                Compile mode: {}\n
                Config:\n
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
}
