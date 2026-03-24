use thiserror::Error;
use wayland_client::protocol::wl_keyboard::KeyState;
use wayland_protocols_plasma::fake_input::client::org_kde_kwin_fake_input::OrgKdeKwinFakeInput;

use crate::xkb::{
	Xkb,
	mapping::{MappedKey, Modifiers, RawKeycode, XkbMappingError},
};

pub fn send_text(fake_input: &OrgKdeKwinFakeInput, xkb: &Xkb, text: &str) {
	let typer = Typer::new(fake_input, xkb);

	for character in text.chars() {
		if let Err(error) = typer.type_char(character) {
			eprintln!("Failed to type character {character:?} with the current layout: {error}");
		}
	}
}

#[derive(Debug, Error)]
#[error(transparent)]
struct TypingError(#[from] XkbMappingError);

struct Typer<'a> {
	fake_input: &'a OrgKdeKwinFakeInput,
	xkb: &'a Xkb,
}

impl<'a> Typer<'a> {
	fn new(fake_input: &'a OrgKdeKwinFakeInput, xkb: &'a Xkb) -> Self {
		Self { fake_input, xkb }
	}

	fn type_char(&self, character: char) -> Result<(), TypingError> {
		let mapped_key = self.xkb.key_for_char(character)?;
		self.send_mapped_key(&mapped_key);
		Ok(())
	}

	fn send_mapped_key(&self, mapped_key: &MappedKey) {
		self.press_modifiers(mapped_key.modifiers);
		self.send_key(mapped_key.raw_keycode, KeyState::Pressed);
		self.send_key(mapped_key.raw_keycode, KeyState::Released);
		self.release_modifiers(mapped_key.modifiers);
	}

	fn press_modifiers(&self, modifiers: Modifiers) {
		if let Some(shift) = modifiers.shift {
			self.send_key(shift.raw_keycode(), KeyState::Pressed);
		}

		if let Some(altgr) = modifiers.altgr {
			self.send_key(altgr.raw_keycode(), KeyState::Pressed);
		}
	}

	fn release_modifiers(&self, modifiers: Modifiers) {
		if let Some(altgr) = modifiers.altgr {
			self.send_key(altgr.raw_keycode(), KeyState::Released);
		}

		if let Some(shift) = modifiers.shift {
			self.send_key(shift.raw_keycode(), KeyState::Released);
		}
	}

	fn send_key(&self, raw_keycode: RawKeycode, state: KeyState) {
		self.fake_input
			.keyboard_key(raw_keycode.into(), state.into());
	}
}
