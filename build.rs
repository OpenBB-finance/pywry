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

	println!("cargo:rerun-if-changed=build.rs");
	println!("cargo:rerun-if-changed=Cargo.toml");

	if cfg!(target_os = "linux") {
		println!("cargo:rustc-link-search=native=../pywry.libs");
	}
}
