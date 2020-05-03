use std::path::PathBuf;
mod bcachefs {
	#![allow(non_upper_case_globals)]
	#![allow(non_camel_case_types)]
	#![allow(non_snake_case)]
	#![allow(unused)]

	include!(concat!(env!("OUT_DIR"), "/bcachefs.rs"));

	use bitfield::bitfield;
	bitfield! {
		pub struct bch_scrypt_flags(u64);
		N, _: 16, 0;
		R, _: 32, 16;
		P, _: 48, 32;
	}
	bitfield! {
		pub struct bch_crypt_flags(u64);
		TYPE, _: 4, 0;
	}
	use memoffset::offset_of;
	impl bch_sb_field_crypt {
		pub fn get_scrypt_flags(&self) -> Option<bch_scrypt_flags> {
			let t = bch_crypt_flags(self.flags);
			if t.TYPE() != bch_kdf_types::BCH_KDF_SCRYPT as u64 {
				None
			} else {
				Some(bch_scrypt_flags(self.kdf_flags))
			}
		}
	}
	impl bch_sb {
		pub fn get_crypt(&self) -> Option<&bch_sb_field_crypt> {
			unsafe {
				let ptr = bch2_sb_field_get(
					self as *const _ as *mut _,
					bch_sb_field_type::BCH_SB_FIELD_crypt,
				) as *const u8;
				if ptr.is_null() {
					None
				} else {
					let offset = offset_of!(bch_sb_field_crypt, field);
					Some(&*((ptr.sub(offset)) as *const _))
				}
			}
		}
		pub fn get_uuid(&self) -> uuid::Uuid {
			uuid::Uuid::from_bytes(self.user_uuid.b)
		}
	}
	impl bch_sb_handle {
		pub fn get_sb(&self) -> &bch_sb {
			unsafe { &*self.sb }
		}
	}
}

extern "C" {
	pub static stdout: *mut libc::FILE;
}

struct FileSystem {
	/// External UUID of the bcachefs
	uuid: uuid::Uuid,
	/// Whether filesystem is encrypted
	encrypted: bool,
	/// Super block
	sb: bcachefs::bch_sb_handle,
	/// Member devices for this filesystem
	devices: Vec<PathBuf>,
}

impl FileSystem {
	fn new(sb: bcachefs::bch_sb_handle) -> Self {
		Self {
			uuid: sb.get_sb().get_uuid(),
			encrypted: sb.get_sb().get_crypt().is_some(),
			sb,
			devices: vec![],
		}
	}
}

fn main() -> anyhow::Result<()> {
	use std::collections::HashMap;
	use std::os::unix::ffi::OsStrExt;
	let mut udev = udev::Enumerator::new()?;
	let mut fss = HashMap::new();
	udev.match_subsystem("block")?;

	{
		// Stop libbcachefs from spamming the output
		let _gag = gag::Gag::stdout().unwrap();
		for dev in udev.scan_devices()? {
			if let Some(p) = dev.devnode() {
				let path =
					std::ffi::CString::new(p.as_os_str().as_bytes()).unwrap();
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
					Ok((_, sb)) => match fss.get_mut(&sb.get_sb().get_uuid()) {
						None => {
							let mut fs = FileSystem::new(sb);
							fs.devices.push(p.to_owned());
							fss.insert(fs.uuid, fs);
						}
						Some(fs) => {
							fs.devices.push(p.to_owned());
						}
					},
					Err(e) if e.kind()
						!= std::io::ErrorKind::PermissionDenied =>
					{
						()
					}
					e @ Err(_) => {
						e?;
					}
				}
			}
		}
		// Flush stdout so buffered output don't get printed after we remove the gag
		unsafe {
			libc::fflush(stdout);
		}
	}

	println!("Found {} bcachefs filesystems: ", fss.len());
	for fs in fss.values() {
		print!(
			"{} ({}): ",
			fs.uuid,
			if fs.encrypted {
				"encrypted"
			} else {
				"unencrypted"
			}
		);
		for dev in &fs.devices {
			print!("{} ", dev.display());
		}
		println!("");
	}
	Ok(())
}
