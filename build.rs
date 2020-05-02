fn main() {
	use std::path::PathBuf;
	use std::process::Command;
	let ncpus = num_cpus::get();
	let out_dir: PathBuf = std::env::var_os("OUT_DIR").unwrap().into();
	Command::new("make")
		.args(&[
			"-C",
			"libbcachefs",
			&format!("-j{}", ncpus),
			"libbcachefs.a",
		])
		.spawn()
		.unwrap();

	use std::fs::copy;
	copy("libbcachefs/libbcachefs.a", out_dir.join("libbcachefs.a")).unwrap();
	println!("cargo:rustc-link-lib=static=bcachefs");
	println!("cargo:rustc-link-search=native={}", out_dir.display());

	let libs = vec![
		"blkid",
		"uuid",
		"liburcu",
		"libsodium",
		"zlib",
		"liblz4",
		"libzstd",
		"libkeyutils",
	];
	for lib in libs {
		pkg_config::probe_library(lib).unwrap();
	}
	println!("cargo:rustc-link-lib=aio");

	let top_dir: PathBuf = std::env::var_os("CARGO_MANIFEST_DIR").unwrap().into();
	let bindings = bindgen::builder()
		.header(top_dir
			.join("libbcachefs")
			.join("libbcachefs")
			.join("super-io.h")
			.display()
			.to_string())
		.clang_arg(format!(
			"-I{}",
			top_dir.join("libbcachefs").join("include").display()
		))
		.clang_arg(format!("-I{}", top_dir.join("libbcachefs").display()))
		.clang_arg("-DZSTD_STATIC_LINKING_ONLY")
		.clang_arg("-DNO_BCACHEFS_FS")
		.whitelist_function("bch2_read_super")
		.opaque_type("gendisk")
		.generate()
		.unwrap();
	bindings.write_to_file(out_dir.join("bcachefs.rs")).unwrap();
}
