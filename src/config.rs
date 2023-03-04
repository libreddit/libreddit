use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{env::var, fs::read_to_string};

// Waiting for https://github.com/rust-lang/rust/issues/74465 to land, so we
// can reduce reliance on once_cell.
//
// This is the local static that is initialized at runtime (technically at
// first request) and contains the instance settings.
pub(crate) static CONFIG: Lazy<Config> = Lazy::new(Config::load);

/// Stores the configuration parsed from the environment variables and the
/// config file. `Config::Default()` contains None for each setting.
/// When adding more config settings, add it to `Config::load`,
/// `get_setting_from_config`, both below, as well as
/// instance_info::InstanceInfo.to_string(), README.md and app.json.
#[derive(Default, Serialize, Deserialize, Clone)]
pub struct Config {
	#[serde(rename = "LIBREDDIT_SFW_ONLY")]
	pub(crate) sfw_only: Option<String>,

	#[serde(rename = "LIBREDDIT_DEFAULT_THEME")]
	pub(crate) default_theme: Option<String>,

	#[serde(rename = "LIBREDDIT_DEFAULT_FRONT_PAGE")]
	pub(crate) default_front_page: Option<String>,

	#[serde(rename = "LIBREDDIT_DEFAULT_LAYOUT")]
	pub(crate) default_layout: Option<String>,

	#[serde(rename = "LIBREDDIT_DEFAULT_WIDE")]
	pub(crate) default_wide: Option<String>,

	#[serde(rename = "LIBREDDIT_DEFAULT_COMMENT_SORT")]
	pub(crate) default_comment_sort: Option<String>,

	#[serde(rename = "LIBREDDIT_DEFAULT_POST_SORT")]
	pub(crate) default_post_sort: Option<String>,

	#[serde(rename = "LIBREDDIT_DEFAULT_SHOW_NSFW")]
	pub(crate) default_show_nsfw: Option<String>,

	#[serde(rename = "LIBREDDIT_DEFAULT_BLUR_NSFW")]
	pub(crate) default_blur_nsfw: Option<String>,

	#[serde(rename = "LIBREDDIT_DEFAULT_USE_HLS")]
	pub(crate) default_use_hls: Option<String>,

	#[serde(rename = "LIBREDDIT_DEFAULT_HIDE_HLS_NOTIFICATION")]
	pub(crate) default_hide_hls_notification: Option<String>,

	#[serde(rename = "LIBREDDIT_DEFAULT_HIDE_AWARDS")]
	pub(crate) default_hide_awards: Option<String>,

	#[serde(rename = "LIBREDDIT_DEFAULT_DISABLE_VISIT_REDDIT_CONFIRMATION")]
	pub(crate) default_disable_visit_reddit_confirmation: Option<String>,

	#[serde(rename = "LIBREDDIT_BANNER")]
	pub(crate) banner: Option<String>,
}

impl Config {
	/// Load the configuration from the environment variables and the config file.
	/// In the case that there are no environment variables set and there is no
	/// config file, this function returns a Config that contains all None values.
	pub fn load() -> Self {
		// Read from libreddit.toml config file. If for any reason, it fails, the
		// default `Config` is used (all None values)
		let config: Config = toml::from_str(&read_to_string("libreddit.toml").unwrap_or_default()).unwrap_or_default();
		// This function defines the order of preference - first check for
		// environment variables with "LIBREDDIT", then check the config, then if
		// both are `None`, return a `None` via the `map_or_else` function
		let parse = |key: &str| -> Option<String> { var(key).ok().map_or_else(|| get_setting_from_config(key, &config), Some) };
		Self {
			sfw_only: parse("LIBREDDIT_SFW_ONLY"),
			default_theme: parse("LIBREDDIT_DEFAULT_THEME"),
			default_front_page: parse("LIBREDDIT_DEFAULT_FRONT_PAGE"),
			default_layout: parse("LIBREDDIT_DEFAULT_LAYOUT"),
			default_post_sort: parse("LIBREDDIT_DEFAULT_POST_SORT"),
			default_wide: parse("LIBREDDIT_DEFAULT_WIDE"),
			default_comment_sort: parse("LIBREDDIT_DEFAULT_COMMENT_SORT"),
			default_show_nsfw: parse("LIBREDDIT_DEFAULT_SHOW_NSFW"),
			default_blur_nsfw: parse("LIBREDDIT_DEFAULT_BLUR_NSFW"),
			default_use_hls: parse("LIBREDDIT_DEFAULT_USE_HLS"),
			default_hide_hls_notification: parse("LIBREDDIT_DEFAULT_HIDE_HLS"),
			default_hide_awards: parse("LIBREDDIT_DEFAULT_HIDE_AWARDS"),
			default_disable_visit_reddit_confirmation: parse("LIBREDDIT_DEFAULT_DISABLE_VISIT_REDDIT_CONFIRMATION"),
			banner: parse("LIBREDDIT_BANNER"),
		}
	}
}

fn get_setting_from_config(name: &str, config: &Config) -> Option<String> {
	match name {
		"LIBREDDIT_SFW_ONLY" => config.sfw_only.clone(),
		"LIBREDDIT_DEFAULT_THEME" => config.default_theme.clone(),
		"LIBREDDIT_DEFAULT_FRONT_PAGE" => config.default_front_page.clone(),
		"LIBREDDIT_DEFAULT_LAYOUT" => config.default_layout.clone(),
		"LIBREDDIT_DEFAULT_COMMENT_SORT" => config.default_comment_sort.clone(),
		"LIBREDDIT_DEFAULT_POST_SORT" => config.default_post_sort.clone(),
		"LIBREDDIT_DEFAULT_SHOW_NSFW" => config.default_show_nsfw.clone(),
		"LIBREDDIT_DEFAULT_BLUR_NSFW" => config.default_blur_nsfw.clone(),
		"LIBREDDIT_DEFAULT_USE_HLS" => config.default_use_hls.clone(),
		"LIBREDDIT_DEFAULT_HIDE_HLS_NOTIFICATION" => config.default_hide_hls_notification.clone(),
		"LIBREDDIT_DEFAULT_WIDE" => config.default_wide.clone(),
		"LIBREDDIT_DEFAULT_HIDE_AWARDS" => config.default_hide_awards.clone(),
		"LIBREDDIT_DEFAULT_DISABLE_VISIT_REDDIT_CONFIRMATION" => config.default_disable_visit_reddit_confirmation.clone(),
		"LIBREDDIT_BANNER" => config.banner.clone(),
		_ => None,
	}
}

/// Retrieves setting from environment variable or config file.
pub(crate) fn get_setting(name: &str) -> Option<String> {
	get_setting_from_config(name, &CONFIG)
}

#[cfg(test)]
use {sealed_test::prelude::*, std::fs::write};

#[test]
#[sealed_test(env = [("LIBREDDIT_SFW_ONLY", "on")])]
fn test_env_var() {
	assert!(crate::utils::sfw_only())
}

#[test]
#[sealed_test]
fn test_config() {
	let config_to_write = r#"LIBREDDIT_DEFAULT_COMMENT_SORT = "best""#;
	write("libreddit.toml", config_to_write).unwrap();
	assert_eq!(get_setting("LIBREDDIT_DEFAULT_COMMENT_SORT"), Some("best".into()));
}

#[test]
#[sealed_test(env = [("LIBREDDIT_DEFAULT_COMMENT_SORT", "top")])]
fn test_env_config_precedence() {
	let config_to_write = r#"LIBREDDIT_DEFAULT_COMMENT_SORT = "best""#;
	write("libreddit.toml", config_to_write).unwrap();
	assert_eq!(get_setting("LIBREDDIT_DEFAULT_COMMENT_SORT"), Some("top".into()))
}

#[test]
#[sealed_test(env = [("LIBREDDIT_DEFAULT_COMMENT_SORT", "top")])]
fn test_alt_env_config_precedence() {
	let config_to_write = r#"LIBREDDIT_DEFAULT_COMMENT_SORT = "best""#;
	write("libreddit.toml", config_to_write).unwrap();
	assert_eq!(get_setting("LIBREDDIT_DEFAULT_COMMENT_SORT"), Some("top".into()))
}
