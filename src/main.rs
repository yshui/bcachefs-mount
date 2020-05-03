mod bcachefs {
	#![allow(non_upper_case_globals)]
	#![allow(non_camel_case_types)]
	#![allow(non_snake_case)]
	#![allow(unused)]

	include!(concat!(env!("OUT_DIR"), "/bcachefs.rs"));
}

fn print_sb(sb: &bcachefs::bch_sb_handle) {
	let user_uuid = unsafe {
		let sb_inner = &*sb.sb;
		uuid::Uuid::from_bytes(sb_inner.user_uuid.b)
	};
	println!("{}", user_uuid);
}
fn main() -> anyhow::Result<()> {
	use std::os::unix::ffi::OsStrExt;
	let mut udev = udev::Enumerator::new()?;
	udev.match_subsystem("block")?;

	for dev in udev.scan_devices()? {
		if let Some(p) = dev.devnode() {
			let path = std::ffi::CString::new(p.as_os_str().as_bytes()).unwrap();
			let result = unsafe {
				let mut opts = std::mem::MaybeUninit::zeroed();
				let mut sb = std::mem::MaybeUninit::zeroed();
				let ret = bcachefs::bch2_read_super(
					path.as_ptr(),
					opts.as_mut_ptr(),
					sb.as_mut_ptr(),
				);
				if ret == -libc::EACCES {
					Err(std::io::Error::new(
						std::io::ErrorKind::PermissionDenied,
						"no permission",
					))
				} else if ret != 0 {
					Err(std::io::Error::new(
						std::io::ErrorKind::Other,
						"Failed to read super",
					))
				} else {
					Ok((opts.assume_init(), sb.assume_init()))
				}
			};
			match result {
				Ok((_, sb)) => {
					println!("{} is bcachefs device:", p.display());
					print_sb(&sb);
				}
				Err(e) if e.kind() != std::io::ErrorKind::PermissionDenied => {
					println!("{} is not a bcachefs device", p.display());
				}
				e @ Err(_) => {
					e?;
				}
			}
		}
	}
	Ok(())
}
