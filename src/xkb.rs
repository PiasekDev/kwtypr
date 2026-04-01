use std::{io, os::fd::OwnedFd};

use thiserror::Error;
use xkbcommon::xkb;

use crate::xkb::{mapping::Modifiers, modifier::AvailableModifiers};

pub mod mapping;
mod modifier;

pub struct Xkb {
	state: xkb::State,
	keymap: xkb::Keymap,
	available_modifiers: AvailableModifiers,
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
		let available_modifiers = AvailableModifiers::from_keymap(&keymap);
		Ok(Self {
			state,
			keymap,
			available_modifiers,
		})
	}

	fn modifiers_for_key_level(
		&self,
		layout: xkb::LayoutIndex,
		keycode: xkb::Keycode,
		level: xkb::LevelIndex,
	) -> Option<Modifiers> {
		const MAX_MODIFIER_MASKS: usize = 100; // Matches the buffer size used in the upstream xkbcommon Rust example.
		let mut modifier_masks = [xkb::ModMask::default(); MAX_MODIFIER_MASKS];
		let num_masks =
			self.keymap
				.key_get_mods_for_level(keycode, layout, level, &mut modifier_masks);

		modifier_masks
			.into_iter()
			.take(num_masks)
			.find(|&mask| self.available_modifiers.can_represent(mask))
			.map(|mask| self.available_modifiers.modifiers_for_mask(mask))
	}
}
