use log::info;

fn wait_for_key(uuid: &uuid::Uuid) -> anyhow::Result<()> {
	use crate::keyutils::{self, keyctl_search};
	let key_name = std::ffi::CString::new(format!("bcachefs:{}", uuid)).unwrap();
	let key_name = key_name.as_c_str().to_bytes_with_nul().as_ptr() as *const _;
	let key_type = c_str!("logon");
	loop {
		let key_id = unsafe {
			keyctl_search(
				keyutils::KEY_SPEC_USER_KEYRING,
				key_type,
				key_name,
				0,
			)
		};
		if key_id > 0 {
			info!("Key has became avaiable");
			break Ok(());
		}
		if errno::errno().0 != libc::ENOKEY {
			Err(crate::ErrnoError(errno::errno()))?;
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
