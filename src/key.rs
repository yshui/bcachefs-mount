use log::info;
macro_rules! c_str {
	($lit:expr) => {
		std::ffi::CStr::from_ptr(concat!($lit, "\0").as_ptr() as *const std::os::raw::c_char)
			       .to_bytes_with_nul()
			       .as_ptr() as *const std::os::raw::c_char
	};
}

#[derive(Debug)]
struct KeyError(errno::Errno);
impl std::fmt::Display for KeyError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
		self.0.fmt(f)
	}
}
impl std::error::Error for KeyError {}
fn wait_for_key(uuid: &uuid::Uuid) -> anyhow::Result<()> {
	use crate::keyutils::{self, keyctl_search};
	let key_name = std::ffi::CString::new(format!("bcachefs:{}", uuid)).unwrap();
	loop {
		let key_id = unsafe {
			keyctl_search(
				keyutils::KEY_SPEC_USER_KEYRING,
				c_str!("logon"),
				key_name.as_c_str().to_bytes_with_nul().as_ptr() as *const _,
				0,
			)
		};
		if key_id > 0 {
			info!("Key has became avaiable");
			break Ok(());
		}
		if errno::errno().0 != libc::ENOKEY {
			Err(KeyError(errno::errno()))?;
		}

		std::thread::sleep(std::time::Duration::from_secs(1));
	}
}

pub(crate) fn prepare_key(uuid: &uuid::Uuid, password: crate::PasswordInput) -> anyhow::Result<()> {
	use crate::PasswordInput::*;
	use anyhow::anyhow;
	match password {
		Fail => Err(anyhow!("no key available")),
		Wait => Ok(wait_for_key(uuid)?),
		Ask => unimplemented!(),
	}
}
