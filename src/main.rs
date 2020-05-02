#![feature(maybe_uninit_ref)]
mod bcachefs {
	#![allow(non_upper_case_globals)]
	#![allow(non_camel_case_types)]
	#![allow(non_snake_case)]
	#![allow(unused)]

	include!(concat!(env!("OUT_DIR"), "/bcachefs.rs"));
}
fn main() {
	let path = std::ffi::CString::new("/dev/sdb").unwrap();
	let (opts, sb) = unsafe {
		let mut opts = std::mem::MaybeUninit::zeroed();
		let mut sb = std::mem::MaybeUninit::zeroed();
		bcachefs::bch2_read_super(path.as_ptr(), opts.as_mut_ptr(), sb.as_mut_ptr());
		(opts.assume_init(), sb.assume_init())
	};
}
