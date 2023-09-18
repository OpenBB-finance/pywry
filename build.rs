// build.rs

use std::env;
use std::fs;
use std::path::Path;

fn main() {
	let out_dir = Path::new("python").join("pywry");
	let init_path = Path::new(&out_dir).join("__init__.py");
	fs::create_dir_all(&out_dir).unwrap();

	let version = env::var("CARGO_PKG_VERSION").unwrap();
	let imports = "from .backend import find_pywry_bin  # noqa: F401
from .core import PyWry  # noqa: F401"
		.to_string();

	fs::write(&init_path, format!("__version__ = \"{}\"\n\n{}\n", version, imports))
		.unwrap();

	let cargo_toml = fs::read_to_string("Cargo.toml").unwrap();
	let wry_version = cargo_toml
		.lines()
		.find(|line| line.starts_with("wry = { version = "))
		.unwrap()
		.split('"')
		.nth(1)
		.unwrap()
		.split('.')
		.map(|s| s.parse::<u32>().unwrap())
		.take(2)
		.collect::<Vec<u32>>();

	if wry_version.iter().sum::<u32>() >= 31 {
		println!("cargo:rustc-cfg=wry_event_loop");
	}

	println!("cargo:rerun-if-changed=build.rs");
	println!("cargo:rerun-if-changed=Cargo.toml");
}
