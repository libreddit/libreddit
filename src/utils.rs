#[allow(dead_code)]
// Post flair with text, background color and foreground color
pub struct Flair(pub String, pub String, pub String);

#[allow(dead_code)]
// Post containing content, metadata and media
pub struct Post {
	pub title: String,
	pub community: String,
	pub body: String,
	pub author: String,
	pub url: String,
	pub score: String,
	pub media: String,
	pub time: String,
	pub flair: Flair,
}

#[allow(dead_code)]
// Comment with content, post, score and data/time that it was posted
pub struct Comment {
	pub body: String,
	pub author: String,
	pub score: String,
	pub time: String,
}

#[allow(dead_code)]
// User struct containing metadata about user
pub struct User {
	pub name: String,
	pub icon: String,
	pub karma: i64,
	pub banner: String,
	pub description: String,
}

#[allow(dead_code)]
// Subreddit struct containing metadata about community
pub struct Subreddit {
	pub name: String,
	pub title: String,
	pub description: String,
	pub icon: String,
}

#[allow(dead_code)]
// val() function used to parse JSON from Reddit APIs
pub async fn val(j: &serde_json::Value, k: &str) -> String {
	String::from(j["data"][k].as_str().unwrap_or(""))
}

#[allow(dead_code)]
// nested_val() function used to parse JSON from Reddit APIs
pub async fn nested_val(j: &serde_json::Value, n: &str, k: &str) -> String {
	String::from(j["data"][n][k].as_str().unwrap())
}

// Parser for query params, used in sorting (eg. /r/rust/?sort=hot)
#[derive(serde::Deserialize)]
pub struct Params {
	pub sort: Option<String>,
}

// Make a request to a Reddit API and parse the JSON response
#[allow(dead_code)]
pub async fn request(url: String) -> serde_json::Value {

	// --- actix-web::client ---
	// let client = actix_web::client::Client::default();
	// let res = client
	// 	.get(url)
	// 	.send()
	// 	.await?
	// 	.body()
	// 	.limit(1000000)
	// 	.await?;

	// let body = std::str::from_utf8(res.as_ref())?; // .as_ref converts Bytes to [u8]

	// --- surf ---
	// let req = surf::get(url);
	// let client = surf::client().with(surf::middleware::Redirect::new(5));
	// let mut res = client.send(req).await.unwrap();
	// let body = res.body_string().await.unwrap();

	// --- reqwest ---
	let resp: String = reqwest::get(&url).await.unwrap().text().await.unwrap();

	// Parse the response from Reddit as JSON
	let json: serde_json::Value = serde_json::from_str(resp.as_str()).expect("Failed to parse JSON");
	
	json
}
