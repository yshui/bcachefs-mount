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
		.clang_arg("-D_GNU_SOURCE")
		.derive_debug(false)
		.default_enum_style(bindgen::EnumVariation::Rust { non_exhaustive: true })
		.whitelist_function("bch2_read_super")
		.whitelist_function("bch2_sb_field_.*")
		.whitelist_var("BCH_.*")
		.whitelist_type("bch_kdf_types")
		.whitelist_type("bch_sb_field_.*")
		.rustified_enum("bch_kdf_types")
		.opaque_type("gendisk")
		.opaque_type("bkey")
		.generate()
		.unwrap();
	bindings.write_to_file(out_dir.join("bcachefs.rs")).unwrap();

	let keyutils = pkg_config::probe_library("libkeyutils").unwrap();
	let bindings = bindgen::builder()
		.header(top_dir.join("src").join("keyutils_wrapper.h").display().to_string())
		.clang_args(keyutils.include_paths.iter().map(|p| format!("-I{}", p.display())))
		.whitelist_function("request_key")
		.whitelist_function("add_key")
		.whitelist_function("keyctl_search")
		.whitelist_var("KEY_SPEC_.*")
		.generate()
		.unwrap();
	bindings.write_to_file(out_dir.join("keyutils.rs")).unwrap();
}
