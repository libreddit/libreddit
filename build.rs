use std::process::Command;
fn main() {
	let output = String::from_utf8(Command::new("git").args(["rev-parse", "HEAD"]).output().unwrap().stdout).unwrap_or_default();
	let git_hash = if output == String::default() { "dev".into() } else { output };
	println!("cargo:rustc-env=GIT_HASH={git_hash}");
}
