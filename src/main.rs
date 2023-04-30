use regex::Regex;
use std::fs::File;
use std::io::{prelude::*, BufReader};
use std::process::{self, Command};

#[derive(PartialEq, Debug)]
enum Target {
	Cargo,
	Make,
	Cpp(String),
	C(String),
	Rust(String),
	Js(String),
	Bash(String),
}

impl Target {
	fn get_filename(&self) -> Option<String> {
		match self {
			Target::Bash(x) | Target::Js(x) | Target::Cpp(x) | Target::Rust(x) | Target::C(x) => {
				Some(x.to_string())
			}
			Target::Make => Some("Makefile".to_owned()),
			Target::Cargo => Some("Cargo.toml".to_owned()),
		}
	}
	fn get_binary(&self) -> Option<String> {
		match self {
			Target::Bash(x) | Target::Js(x) => return Some(x.to_string()),
			Target::Cpp(x) | Target::Rust(x) | Target::C(x) => {
				let mut bin = x.clone();
				bin.truncate(bin.find(".").unwrap());
				return Some(bin);
			}
			Target::Make => {
				let file = File::open("Makefile").unwrap();
				let lines = BufReader::new(file).lines();
				let target = Regex::new("^TARGET\\s*:=\\s*(\\w+)").expect("Regex error");

				for line in lines {
					if let Ok(line) = line {
						let mat = target.captures(&line);
						if let Some(mat) = mat {
							let val = mat.get(1);
							if let Some(val) = val {
								return Some(val.as_str().to_owned());
							}
						}
					}
				}
				return None;
			}
			Target::Cargo => {
				let file = File::open("Cargo.toml").unwrap();
				let lines = BufReader::new(file).lines();
				let target = Regex::new("^name\\s*=\\s*\"(\\w+)\"").expect("Regex error");

				for line in lines {
					if let Ok(line) = line {
						let mat = target.captures(&line);
						if let Some(mat) = mat {
							let val = mat.get(1);
							if let Some(val) = val {
								return Some(val.as_str().to_owned());
							}
						}
					}
				}
				return None;
			}
		}
	}

	fn handle_build_result(&self, return_code: i32, _stdout: Option<u8>) -> bool {
		match self {
			_ => {
				if return_code != 0 {
					return false;
				}
				return true;
			}
		}
	}
}

fn update_target(old_target: Option<Target>, new_target: Option<Target>) -> Option<Target> {
	match (old_target, new_target) {
		(Some(Target::Make), _) => return Some(Target::Make),
		(_, Some(Target::Make)) => return Some(Target::Make),
		(Some(Target::Cargo), _) => return Some(Target::Cargo),
		(_, Some(Target::Cargo)) => return Some(Target::Cargo),
		(_, Some(x)) => return Some(x),
		(_, _) => return None,
	}
}

fn endings(file_name: &str) -> Option<Target> {
	if file_name.ends_with(".js") {
		return Some(Target::Js(file_name.to_string()));
	} else if file_name.ends_with(".cpp") {
		return Some(Target::Cpp(file_name.to_string()));
	} else if file_name.ends_with(".bash") || file_name.ends_with(".sh") {
		return Some(Target::Bash(file_name.to_string()));
	} else if file_name.ends_with(".rs") {
		return Some(Target::Rust(file_name.to_string()));
	} else if file_name.ends_with(".c") {
		return Some(Target::C(file_name.to_string()));
	}
	None
}

fn main() -> Result<(), Box<(dyn std::error::Error + 'static)>> {
	let mut target = None;

	let mut run = false;
	let mut build = false;
	let mut release = false;

	for arg in std::env::args() {
		match arg.as_str() {
			"build" => build = true,
			"run" => run = true,
			"release" => release = true,
			_ => continue,
		}
	}

	for entry in std::fs::read_dir(".").expect("Faild to read dir") {
		let entry = entry?.file_name();

		if let Some(entry) = entry.to_str() {
			match entry {
				"Makefile" => {
					target = update_target(target, Some(Target::Make));
					break;
				}
				"Cargo.toml" => target = update_target(target, Some(Target::Cargo)),
				_ => {
					if target == None {
						if entry.starts_with("main.") {
							target = update_target(target,endings(entry));
						} else if entry.starts_with("index.") {
							target = update_target(target, endings(entry));
						} else if entry.starts_with("test.") {
							target = update_target(target, endings(entry));
						}
					}
				}
			}
		}
	}

	if build || release {
		if let Some(ref target) = target {
			println!("==== Build target ({})", target.get_filename().unwrap());

			let mut command = match target {
				Target::Make => {
					let mut command = Command::new("make");
					if release {
						command.arg("release");
					}
					command
				}
				Target::Cargo => {
					let mut command = Command::new("cargo");
					command.arg("build");
					if release {
						command.arg("--release");
					}
					command
				}

				Target::Cpp(ref file) => {
					let mut command = Command::new("g++");
					command.arg(file);
					command.arg("-o");
					command.arg(target.get_binary().unwrap());
					if release {
						command.arg("-O3");
					}
					command
				}
				Target::C(ref file) => {
					let mut command = Command::new("gcc");
					command.arg(file);
					command.arg("-o");
					command.arg(target.get_binary().unwrap());
					if release {
						command.arg("-O3");
					}
					command
				}
				Target::Rust(ref file) => {
					let mut command = Command::new("rustc");
					command.arg(file);
					command
				}
				Target::Js(ref file) => {
					let mut command = Command::new("eslint");
					command.arg("--env").arg("es6").arg(file);
					command
				}
				Target::Bash(ref file) => {
					let mut command = Command::new("shellcheck");
					command.arg(file);
					command
				}
			};

			let child = command.spawn().map_or(None, |c| Some(c));
			if let Some(mut child) = child {
				let ret = child
					.wait()
					.map_or(127, |code| code.code().expect("==== Build terminated"));

				if target.handle_build_result(ret, None) {
					println!("==== Build Successfull");
				} else {
					run = false;
					println!("==== Build Failed");
				}
			} else {
				println!("==== Failed to run build command")
			}
		} else {
			println!("==== No build target found");
			process::exit(2);
		}
	}

	if run {
		if let Some(ref target) = target  {

			let binary = target.get_binary();
			if binary == None {
				println!("==== No target to run found {:?}",target);
				process::exit(2);
			}
			let binary = binary.unwrap();
			println!("==== Run target ({})", target.get_binary().unwrap());

			let mut command = match target {
				Target::Make | Target::C(_) | Target::Cpp(_) | Target::Rust(_) => {
					let command = Command::new(format!("./{}", binary));
					command
				}
				Target::Cargo => {
					let mut command = Command::new("cargo");
					command.arg("run");
					if release {
						command.arg("--release");
					}
					command
				}
				Target::Js(_) => {
					let mut command = Command::new("node");
					command.arg(format!("./{}",binary));
					command
				}
				Target::Bash(_) => {
					let mut command = Command::new("bash");
					command.arg(format!("./{}",binary));
					command
				}
			};

			let child = command.spawn().map_or(None, |c| Some(c));
			if let Some(mut child) = child {
				let ret = child
					.wait()
					.map_or(127, |code| code.code().expect("==== Build terminated"));

				println!("==== Run return code [{}]",ret);
			} else {
				println!("==== Failed to run programm");
			}
		} else {
			println!("==== No target to run found");
			process::exit(2);
		}
	}

	Ok(())
}
