extern "C" {
	pub static stdout: *mut libc::FILE;
}

use getset::{CopyGetters, Getters};
use std::path::PathBuf;
#[derive(Getters, CopyGetters)]
pub struct FileSystem {
	/// External UUID of the bcachefs
	#[getset(get = "pub")]
	uuid: uuid::Uuid,
	/// Whether filesystem is encrypted
	#[getset(get_copy = "pub")]
	encrypted: bool,
	/// Super block
	#[getset(get = "pub")]
	sb: bcachefs::bch_sb_handle,
	/// Member devices for this filesystem
	#[getset(get = "pub")]
	devices: Vec<PathBuf>,
}

/// Parse a comma-separated mount options and split out mountflags and filesystem
/// specific options.
fn parse_mount_options(options: impl AsRef<str>) -> (Option<String>, u64) {
	use either::Either::*;
	let (opts, flags) = options
		.as_ref()
		.split(",")
		.map(|o| match o {
			"dirsync" => Left(libc::MS_DIRSYNC),
			"lazytime" => Left(1 << 25), // MS_LAZYTIME
			"mand" => Left(libc::MS_MANDLOCK),
			"noatime" => Left(libc::MS_NOATIME),
			"nodev" => Left(libc::MS_NODEV),
			"nodiratime" => Left(libc::MS_NODIRATIME),
			"noexec" => Left(libc::MS_NOEXEC),
			"nosuid" => Left(libc::MS_NOSUID),
			"ro" => Left(libc::MS_RDONLY),
			"rw" => Left(0),
			"relatime" => Left(libc::MS_RELATIME),
			"strictatime" => Left(libc::MS_STRICTATIME),
			"sync" => Left(libc::MS_SYNCHRONOUS),
			"" => Left(0),
			o @ _ => Right(o),
		})
		.fold((Vec::new(), 0), |(mut opts, flags), next| match next {
			Left(f) => (opts, flags | f),
			Right(o) => {
				opts.push(o);
				(opts, flags)
			}
		});

	use itertools::Itertools;
	(
		if opts.len() == 0 {
			None
		} else {
			Some(opts.iter().join(","))
		},
		flags,
	)
}

impl FileSystem {
	pub(crate) fn new(sb: bcachefs::bch_sb_handle) -> Self {
		Self {
			uuid: sb.sb().uuid(),
			encrypted: sb.sb().crypt().is_some(),
			sb: sb,
			devices: vec![],
		}
	}

	pub fn mount(
		&self,
		target: impl AsRef<std::path::Path>,
		options: impl AsRef<str>,
	) -> anyhow::Result<()> {
		use itertools::Itertools;
		use std::ffi::c_void;
		use std::os::raw::c_char;
		use std::os::unix::ffi::OsStrExt;
		let src = self.devices.iter().map(|d| d.display()).join(":");
		let (data, mountflags) = parse_mount_options(options);
		let fstype = c_str!("bcachefs");

		let src = std::ffi::CString::new(src)?; // bind the CString to keep it alive
		let target = std::ffi::CString::new(target.as_ref().as_os_str().as_bytes())?; // ditto
		let data = data.map(|data| std::ffi::CString::new(data)).transpose()?; // ditto

		let src = src.as_c_str().to_bytes_with_nul().as_ptr() as *const c_char;
		let target = target.as_c_str().to_bytes_with_nul().as_ptr() as *const c_char;
		let data = data.as_ref().map_or(std::ptr::null(), |data| {
			data.as_c_str().to_bytes_with_nul().as_ptr() as *const c_void
		});

		let ret = unsafe { libc::mount(src, target, fstype, mountflags, data) };
		if ret == 0 {
			Ok(())
		} else {
			Err(crate::ErrnoError(errno::errno()).into())
		}
	}
}

use crate::bcachefs;
use std::collections::HashMap;
use uuid::Uuid;
pub fn probe_filesystems() -> anyhow::Result<HashMap<Uuid, FileSystem>> {
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
							"failed to read super",
						))
					} else {
						Ok((opts.assume_init(), sb.assume_init()))
					}
				};
				match result {
					Ok((_, sb)) => match fss.get_mut(&sb.sb().uuid()) {
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
	Ok(fss)
}
