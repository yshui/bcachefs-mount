use log::info;

fn check_for_key(key_name: &std::ffi::CStr) -> anyhow::Result<bool> {
	use crate::keyutils::{self, keyctl_search};
	let key_name = key_name.to_bytes_with_nul().as_ptr() as *const _;
	let key_type = c_str!("logon");

	let key_id = unsafe {
		keyctl_search(keyutils::KEY_SPEC_USER_KEYRING, key_type, key_name, 0)
	};
	if key_id > 0 {
		info!("Key has became avaiable");
		Ok(true)
	} else if errno::errno().0 != libc::ENOKEY {
		Err(crate::ErrnoError(errno::errno()).into())
	} else {
		Ok(false)
	}

}

fn wait_for_key(uuid: &uuid::Uuid) -> anyhow::Result<()> {
	let key_name = std::ffi::CString::new(format!("bcachefs:{}", uuid)).unwrap();
	loop {
		if check_for_key(&key_name)? {
			break Ok(())
		}

		std::thread::sleep(std::time::Duration::from_secs(1));
	}
}

const BCH_KEY_MAGIC: &str = "bch**key";
use crate::filesystem::FileSystem;
fn ask_for_key(fs: &FileSystem) -> anyhow::Result<()> {
	use crate::bcachefs::{bch2_chacha_encrypt_key, bch_encrypted_key, bch_key};
	use anyhow::anyhow;
	use scrypt::ScryptParams;
	use std::convert::TryInto;
	use byteorder::{ReadBytesExt, LittleEndian};

	let key_name = std::ffi::CString::new(format!("bcachefs:{}", fs.uuid())).unwrap();
	if check_for_key(&key_name)? {
		return Ok(())
	}

	let bch_key_magic = BCH_KEY_MAGIC.as_bytes().read_u64::<LittleEndian>().unwrap();
	let crypt = fs.sb().sb().crypt().unwrap();
	let scrypt_flags = crypt.scrypt_flags().ok_or(anyhow!("Unsupported crypto"))?;
	let pass = rpassword::read_password_from_tty(Some("Enter passphrase"))?;
	let mut output: bch_key = Default::default();
	let output_slice = unsafe {
		std::slice::from_raw_parts_mut(
			&mut output as *mut bch_key as *mut u8,
			std::mem::size_of::<bch_key>(),
		)
	};

	scrypt::scrypt(
		pass.as_bytes(),
		"bcache".as_bytes(),
		&ScryptParams::new(
			scrypt_flags.N().try_into()?,
			1 << scrypt_flags.R(),
			1 << scrypt_flags.P(),
		)?,
		output_slice,
	)?;

	let mut key = crypt.key().clone();
	unsafe {
		bch2_chacha_encrypt_key(
			&mut output as *mut _,
			fs.sb().sb().nonce(),
			&mut key as *mut _ as *mut _,
			std::mem::size_of::<bch_encrypted_key>() as u64,
		)
	};
	if key.magic == bch_key_magic {
		Ok(())
	} else {
		Err(anyhow!("failed to verify the password"))
	}
}

pub(crate) fn prepare_key(fs: &FileSystem, password: crate::PasswordInput) -> anyhow::Result<()> {
	use crate::PasswordInput::*;
	use anyhow::anyhow;
	match password {
		Fail => Err(anyhow!("no key available")),
		Wait => Ok(wait_for_key(fs.uuid())?),
		Ask => ask_for_key(fs),
	}
}
