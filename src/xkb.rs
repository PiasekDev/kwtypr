use std::{io, os::fd::OwnedFd};

use thiserror::Error;
use xkbcommon::xkb;

pub mod mapping;
pub mod modifier;

pub struct Xkb {
	state: xkb::State,
}

#[derive(Debug, Error)]
pub enum XkbInitError {
	#[error("failed to read or map the Wayland keymap fd")]
	Io(#[from] io::Error),
	#[error("xkbcommon failed to compile the Wayland keymap")]
	KeymapCompileFailed,
}

impl Xkb {
	pub fn from_wayland_keymap(fd: OwnedFd, size: u32) -> Result<Self, XkbInitError> {
		let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);

		// SAFETY: We trust the compositor to provide us with a valid keymap fd and size.
		let keymap = unsafe {
			xkb::Keymap::new_from_fd(
				&context,
				fd,
				size as usize,
				xkb::KEYMAP_FORMAT_TEXT_V1,
				xkb::COMPILE_NO_FLAGS,
			)
		}?
		.ok_or(XkbInitError::KeymapCompileFailed)?;

		let state = xkb::State::new(&keymap);
		Ok(Self { state })
	}
}
