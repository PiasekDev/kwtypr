use thiserror::Error;
use wayland_client::protocol::wl_keyboard::KeyState;
use wayland_protocols_plasma::fake_input::client::org_kde_kwin_fake_input::OrgKdeKwinFakeInput;

use crate::xkb::{
	Xkb,
	mapping::{MappedKey, Modifiers, RawKeycode, XkbMappingError},
};

pub fn send_text(fake_input: &OrgKdeKwinFakeInput, xkb: &Xkb, text: &str) {
	let mut typer = Typer::new(fake_input, xkb);

	for character in text.chars() {
		if let Err(error) = typer.type_char(character) {
			eprintln!("Failed to type character {character:?} with the current layout: {error}");
		}
	}

	typer.release_all_modifiers();
}

#[derive(Debug, Error)]
#[error(transparent)]
struct TypingError(#[from] XkbMappingError);

struct Typer<'a> {
	fake_input: &'a OrgKdeKwinFakeInput,
	xkb: &'a Xkb,
	active_modifiers: ActiveModifiers,
}

#[derive(Default)]
struct ActiveModifiers {
	shift: Option<RawKeycode>,
	altgr: Option<RawKeycode>,
}

impl<'a> Typer<'a> {
	fn new(fake_input: &'a OrgKdeKwinFakeInput, xkb: &'a Xkb) -> Self {
		Self {
			fake_input,
			xkb,
			active_modifiers: ActiveModifiers::default(),
		}
	}

	fn type_char(&mut self, character: char) -> Result<(), TypingError> {
		let mapped_key = self.xkb.key_for_char(character)?;
		self.send_mapped_key(&mapped_key);
		Ok(())
	}

	fn send_mapped_key(&mut self, mapped_key: &MappedKey) {
		self.transition_modifiers(mapped_key.modifiers);
		self.send_key(mapped_key.raw_keycode, KeyState::Pressed);
		self.send_key(mapped_key.raw_keycode, KeyState::Released);
	}

	fn transition_modifiers(&mut self, modifiers: Modifiers) {
		let target_modifiers = ActiveModifiers {
			shift: modifiers.shift.map(|shift| shift.raw_keycode()),
			altgr: modifiers.altgr.map(|altgr| altgr.raw_keycode()),
		};

		self.active_modifiers.altgr =
			self.transition_modifier(self.active_modifiers.altgr, target_modifiers.altgr);
		self.active_modifiers.shift =
			self.transition_modifier(self.active_modifiers.shift, target_modifiers.shift);
	}

	fn transition_modifier(
		&self,
		previous: Option<RawKeycode>,
		next: Option<RawKeycode>,
	) -> Option<RawKeycode> {
		match (previous, next) {
			(None, None) => None,
			(None, Some(next)) => {
				self.send_key(next, KeyState::Pressed);
				Some(next)
			}
			(Some(previous), None) => {
				self.send_key(previous, KeyState::Released);
				None
			}
			(Some(previous), Some(next)) if previous == next => Some(next),
			(Some(previous), Some(next)) => {
				self.send_key(previous, KeyState::Released);
				self.send_key(next, KeyState::Pressed);
				Some(next)
			}
		}
	}

	fn release_all_modifiers(&mut self) {
		self.active_modifiers.altgr = self.transition_modifier(self.active_modifiers.altgr, None);
		self.active_modifiers.shift = self.transition_modifier(self.active_modifiers.shift, None);
	}

	fn send_key(&self, raw_keycode: RawKeycode, state: KeyState) {
		self.fake_input
			.keyboard_key(raw_keycode.into(), state.into());
	}
}
