use std::collections::HashMap;

use crate::client::CLIENT;
use base64::{engine::general_purpose, Engine as _};
use hyper::{client, Body, Method, Request};
use serde_json::json;

static REDDIT_ANDROID_OAUTH_CLIENT_ID: &str = "ohXpoqrZYub1kg";

static AUTH_ENDPOINT: &str = "https://accounts.reddit.com";
pub(crate) static USER_AGENT: &str = "Reddit/Version 2023.21.0/Build 956283/Android 13";

pub(crate) struct Oauth {
	// Currently unused, may be necessary if we decide to support GQL in the future
	pub(crate) headers_map: HashMap<String, String>,
	pub(crate) token: String,
}

impl Oauth {
	pub fn new() -> Self {
		let uuid = uuid::Uuid::new_v4().to_string();
		Oauth {
			headers_map: HashMap::from([
				("Client-Vendor-Id".into(), uuid.clone()),
				("X-Reddit-Device-Id".into(), uuid),
				("User-Agent".into(), USER_AGENT.to_string()),
			]),
			token: String::new(),
		}
	}
	pub async fn login(&mut self) -> Option<()> {
		let url = format!("{}/api/access_token", AUTH_ENDPOINT);
		let mut builder = Request::builder().method(Method::POST).uri(&url);
		for (key, value) in self.headers_map.iter() {
			builder = builder.header(key, value);
		}

		let auth = general_purpose::STANDARD.encode(format!("{REDDIT_ANDROID_OAUTH_CLIENT_ID}:"));
		builder = builder.header("Authorization", format!("Basic {auth}"));
		let json = json!({
				"scopes": ["*","email","pii"]
		});
		let body = Body::from(json.to_string());
		let request = builder.body(body).unwrap();
		let client: client::Client<_, hyper::Body> = CLIENT.clone();
		let resp = client.request(request).await.ok()?;
		let body_bytes = hyper::body::to_bytes(resp.into_body()).await.ok()?;
		let json: serde_json::Value = serde_json::from_slice(&body_bytes).ok()?;
		self.token = json.get("access_token")?.as_str()?.to_string();
		self.headers_map.insert("Authorization".to_owned(), format!("Bearer {}", self.token));
		Some(())
	}
}
