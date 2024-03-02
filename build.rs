use std::process::{Command, ExitStatus, Output};

#[cfg(not(target_os = "windows"))]
use std::os::unix::process::ExitStatusExt;

#[cfg(target_os = "windows")]
use std::os::windows::process::ExitStatusExt;

fn main() {
	let output = String::from_utf8(
		Command::new("git")
			.args(["rev-parse", "HEAD"])
			.output()
			.unwrap_or(Output {
				stdout: vec![],
				stderr: vec![],
				status: ExitStatus::from_raw(0),
			})
			.stdout,
	)
	.unwrap_or_default();
	let git_hash = if output == String::default() { "dev".into() } else { output };
	println!("cargo:rustc-env=GIT_HASH={git_hash}");
}
