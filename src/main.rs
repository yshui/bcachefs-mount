use structopt::StructOpt;
use anyhow::anyhow;

#[macro_export]
macro_rules! c_str {
	($lit:expr) => {
		unsafe { std::ffi::CStr::from_ptr(concat!($lit, "\0").as_ptr() as *const std::os::raw::c_char)
			       .to_bytes_with_nul()
			       .as_ptr() as *const std::os::raw::c_char }
	};
}

#[derive(Debug)]
struct ErrnoError(errno::Errno);
impl std::fmt::Display for ErrnoError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
		self.0.fmt(f)
	}
}
impl std::error::Error for ErrnoError {}

#[derive(parse_display::FromStr, parse_display::Display, Debug)]
#[display(style = "snake_case")]
pub(crate) enum PasswordInput {
	Fail,
	Wait,
	Ask,
}

#[derive(StructOpt, Debug)]
struct Options {
	/// Where the password would be loaded from.
	///
	/// Possible values are:
	/// "fail" - don't ask for password, fail if filesystem is encrypted;
	/// "wait" - wait for password to become available before mounting;
	/// "ask" -  prompt the user for password;
	#[structopt(short, long, default_value = "fail")]
	password: PasswordInput,

	/// External UUID of the bcachefs filesystem
	uuid: uuid::Uuid,

	/// Where the filesystem should be mounted
	mountpoint: std::path::PathBuf,

	/// Mount options
	#[structopt(short, default_value = "")]
	options: String,
}

mod filesystem;
mod key;
mod keyutils {
	#![allow(non_upper_case_globals)]
	#![allow(non_camel_case_types)]
	#![allow(non_snake_case)]
	#![allow(unused)]

	include!(concat!(env!("OUT_DIR"), "/keyutils.rs"));
}

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
		pub fn scrypt_flags(&self) -> Option<bch_scrypt_flags> {
			let t = bch_crypt_flags(self.flags);
			if t.TYPE() != bch_kdf_types::BCH_KDF_SCRYPT as u64 {
				None
			} else {
				Some(bch_scrypt_flags(self.kdf_flags))
			}
		}
	}
	impl bch_sb {
		pub fn crypt(&self) -> Option<&bch_sb_field_crypt> {
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
		pub fn uuid(&self) -> uuid::Uuid {
			uuid::Uuid::from_bytes(self.user_uuid.b)
		}
	}
	impl bch_sb_handle {
		pub fn sb(&self) -> &bch_sb {
			unsafe { &*self.sb }
		}
	}
}

fn main() -> anyhow::Result<()> {
	use itertools::Itertools;
	use log::{info, trace};

	env_logger::init();
	let opt = Options::from_args();
	trace!("{:?}", opt);

	let fss = filesystem::probe_filesystems()?;
	info!("Found {} bcachefs filesystems: ", fss.len());
	for fs in fss.values() {
		info!(
			"{} ({}): {}",
			fs.uuid(),
			if fs.encrypted() {
				"encrypted"
			} else {
				"unencrypted"
			},
			fs.devices().iter().map(|d| d.display()).join(" ")
		);
	}

	if let Some(fs) = fss.get(&opt.uuid) {
		if fs.encrypted() {
			info!("Making sure key is loaded for this filesystem");
			key::prepare_key(fs.uuid(), opt.password)?;
		}

		fs.mount(&opt.mountpoint, &opt.options)
	} else {
		Err(anyhow!("Filesystem {} is not found", opt.uuid))
	}
}
