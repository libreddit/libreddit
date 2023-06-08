use std::{collections::HashMap, time::Duration};

use crate::client::{CLIENT, OAUTH_CLIENT};
use base64::{engine::general_purpose, Engine as _};
use hyper::{client, Body, Method, Request};
use log::info;

use serde_json::json;

static REDDIT_ANDROID_OAUTH_CLIENT_ID: &str = "ohXpoqrZYub1kg";
static REDDIT_IOS_OAUTH_CLIENT_ID: &str = "LNDo9k1o8UAEUw";

static AUTH_ENDPOINT: &str = "https://accounts.reddit.com";

// Various Android user agents - build numbers from valid APK variants
pub(crate) static ANDROID_USER_AGENT: [&str; 3] = [
	"Reddit/Version 2023.21.0/Build 956283/Android 13",
	"Reddit/Version 2023.21.0/Build 968223/Android 10",
	"Reddit/Version 2023.21.0/Build 946732/Android 12",
];

// Various iOS user agents - iOS versions.
pub(crate) static IOS_USER_AGENT: [&str; 3] = [
	"Reddit/Version 2023.22.0/Build 613580/iOS Version 17.0 (Build 21A5248V)",
	"Reddit/Version 2023.22.0/Build 613580/iOS Version 16.0 (Build 20A5328h)",
	"Reddit/Version 2023.22.0/Build 613580/iOS Version 16.5",
];
// Various iOS device codes. iPhone 11 displays as `iPhone12,1`
// I just changed the number a few times for some plausible values
pub(crate) static IOS_DEVICES: [&str; 5] = ["iPhone8,1", "iPhone11,1", "iPhone12,1", "iPhone13,1", "iPhone14,1"];

#[derive(Debug, Clone, Default)]
pub(crate) struct Oauth {
	// Currently unused, may be necessary if we decide to support GQL in the future
	pub(crate) headers_map: HashMap<String, String>,
	pub(crate) token: String,
	expires_in: u64,
	device: Device,
}

impl Oauth {
	pub(crate) async fn new() -> Self {
		let mut oauth = Oauth::default();
		oauth.login().await;
		oauth
	}
	pub(crate) fn default() -> Self {
		// Generate a random device to spoof
		let device = Device::random();
		let headers = device.headers.clone();
		// For now, just insert headers - no token request
		Oauth {
			headers_map: headers,
			token: String::new(),
			expires_in: 0,
			device,
		}
	}
	async fn login(&mut self) -> Option<()> {
		// Construct URL for OAuth token
		let url = format!("{}/api/access_token", AUTH_ENDPOINT);
		let mut builder = Request::builder().method(Method::POST).uri(&url);

		// Add headers from spoofed client
		for (key, value) in self.headers_map.iter() {
			// Skip Authorization header - won't be present in `Device` struct
			// and will only be there in subsequent token refreshes.
			// Sending a bearer auth token when requesting one is a bad idea
			// Normally, you'd want to send it along to authenticate a refreshed token,
			// but neither Android nor iOS does this - it just requests a new token.
			// We try to match behavior as closely as possible.
			if key != "Authorization" {
				builder = builder.header(key, value);
			}
		}
		// Set up HTTP Basic Auth - basically just the const OAuth ID's with no password,
		// Base64-encoded. https://en.wikipedia.org/wiki/Basic_access_authentication
		// This could be constant, but I don't think it's worth it. OAuth ID's can change
		// over time and we want to be flexible.
		let auth = general_purpose::STANDARD.encode(format!("{}:", self.device.oauth_id));
		builder = builder.header("Authorization", format!("Basic {auth}"));

		// Set JSON body. I couldn't tell you what this means. But that's what the client sends
		let json = json!({
				"scopes": ["*","email","pii"]
		});
		let body = Body::from(json.to_string());

		// Build request
		let request = builder.body(body).unwrap();

		// Send request
		let client: client::Client<_, hyper::Body> = CLIENT.clone();
		let resp = client.request(request).await.ok()?;

		// Parse headers - loid header _should_ be saved sent on subsequent token refreshes.
		// Technically it's not needed, but it's easy for Reddit API to check for this.
		// It's some kind of header that uniquely identifies the device.
		if let Some(header) = resp.headers().get("x-reddit-loid") {
			self.headers_map.insert("x-reddit-loid".to_owned(), header.to_str().ok()?.to_string());
		}

		// Serialize response
		let body_bytes = hyper::body::to_bytes(resp.into_body()).await.ok()?;
		let json: serde_json::Value = serde_json::from_slice(&body_bytes).ok()?;

		// Save token and expiry
		self.token = json.get("access_token")?.as_str()?.to_string();
		self.expires_in = json.get("expires_in")?.as_u64()?;
		self.headers_map.insert("Authorization".to_owned(), format!("Bearer {}", self.token));

		info!("✅ Success - Retrieved token \"{}...\", expires in {}", &self.token[..32], self.expires_in);

		Some(())
	}

	async fn refresh(&mut self) -> Option<()> {
		// Refresh is actually just a subsequent login with the same headers (without the old token
		// or anything). This logic is handled in login, so we just call login again.
		let refresh = self.login().await;
		info!("Refreshing OAuth token... {}", if refresh.is_some() { "success" } else { "failed" });
		refresh
	}
}

pub(crate) async fn token_daemon() {
	// Monitor for refreshing token
	loop {
		// Get expiry time - be sure to not hold the read lock
		let expires_in = { OAUTH_CLIENT.read().await.expires_in };

		// sleep for the expiry time minus 2 minutes
		let duration = Duration::from_secs(expires_in - 120);

		info!("Waiting for {duration:?} seconds before refreshing OAuth token...");

		tokio::time::sleep(duration).await;

		info!("[{duration:?} ELAPSED] Refreshing OAuth token...");

		// Refresh token - in its own scope
		{
			let mut client = OAUTH_CLIENT.write().await;
			client.refresh().await;
		}
	}
}
#[derive(Debug, Clone, Default)]
struct Device {
	oauth_id: String,
	headers: HashMap<String, String>,
}

impl Device {
	fn android() -> Self {
		// Generate uuid
		let uuid = uuid::Uuid::new_v4().to_string();

		// Select random user agent from ANDROID_USER_AGENT
		let android_user_agent = choose(&ANDROID_USER_AGENT).to_string();

		// Android device headers
		let headers = HashMap::from([
			("Client-Vendor-Id".into(), uuid.clone()),
			("X-Reddit-Device-Id".into(), uuid.clone()),
			("User-Agent".into(), android_user_agent),
		]);

		info!("Spoofing Android client with headers: {headers:?}, uuid: \"{uuid}\", and OAuth ID \"{REDDIT_ANDROID_OAUTH_CLIENT_ID}\"");

		Device {
			oauth_id: REDDIT_ANDROID_OAUTH_CLIENT_ID.to_string(),
			headers,
		}
	}
	fn ios() -> Self {
		// Generate uuid
		let uuid = uuid::Uuid::new_v4().to_string();

		// Select random user agent from IOS_USER_AGENT
		let ios_user_agent = choose(&IOS_USER_AGENT).to_string();

		// Select random iOS device from IOS_DEVICES
		let ios_device = choose(&IOS_DEVICES).to_string();

		// iOS device headers
		let headers = HashMap::from([
			("X-Reddit-DPR".into(), "2".into()),
			("Device-Name".into(), ios_device.clone()),
			("X-Reddit-Device-Id".into(), uuid.clone()),
			("User-Agent".into(), ios_user_agent),
			("Client-Vendor-Id".into(), uuid.clone()),
		]);

		info!("Spoofing iOS client {ios_device} with headers: {headers:?}, uuid: \"{uuid}\", and OAuth ID \"{REDDIT_IOS_OAUTH_CLIENT_ID}\"");

		Device {
			oauth_id: REDDIT_IOS_OAUTH_CLIENT_ID.to_string(),
			headers,
		}
	}
	// Randomly choose a device
	fn random() -> Self {
		if fastrand::bool() {
			Device::android()
		} else {
			Device::ios()
		}
	}
}

// Waiting on fastrand 2.0.0 for the `choose` function
// https://github.com/smol-rs/fastrand/pull/59/
fn choose<T: Copy>(list: &[T]) -> T {
	list[fastrand::usize(..list.len())]
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_oauth_client() {
	assert!(!OAUTH_CLIENT.read().await.token.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_oauth_client_refresh() {
	OAUTH_CLIENT.write().await.refresh().await.unwrap();
}
