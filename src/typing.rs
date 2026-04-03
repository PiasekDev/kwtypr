use std::thread;

use thiserror::Error;
use wayland_client::protocol::wl_keyboard::KeyState;
use wayland_protocols_plasma::fake_input::client::org_kde_kwin_fake_input::OrgKdeKwinFakeInput;

use crate::Components;
use crate::KwtyprConfig;
use crate::xkb::{
	Xkb,
	mapping::{MappedKey, Modifiers, PlatformKeycode, XkbMappingError},
};

#[derive(Debug, Error)]
#[error(transparent)]
struct TypingError(#[from] XkbMappingError);

pub struct Typer<'a> {
	fake_input: &'a OrgKdeKwinFakeInput,
	xkb: &'a Xkb,
	config: &'a KwtyprConfig,
	active_modifiers: ActiveModifiers,
}

#[derive(Default)]
struct ActiveModifiers {
	shift: Option<PlatformKeycode>,
	altgr: Option<PlatformKeycode>,
}

impl<'a> Typer<'a> {
	pub fn new(components: &'a Components, config: &'a KwtyprConfig) -> Self {
		let Components { fake_input, xkb } = components;
		Self {
			fake_input,
			xkb,
			config,
			active_modifiers: ActiveModifiers::default(),
		}
	}

	pub fn type_text(&mut self, text: &str) {
		for character in text.chars() {
			if let Err(error) = self.type_char(character) {
				eprintln!(
					"Failed to type character {character:?} with the current layout: {error}"
				);
			}

			if !self.config.character_delay.is_zero() {
				thread::sleep(self.config.character_delay);
			}
		}

		self.release_all_modifiers();
	}

	fn type_char(&mut self, character: char) -> Result<(), TypingError> {
		let mapped_key = self.xkb.key_for_char(character)?;
		self.send_mapped_key(&mapped_key);
		Ok(())
	}

	fn send_mapped_key(&mut self, mapped_key: &MappedKey) {
		self.transition_modifiers(mapped_key.modifiers);
		self.send_key(mapped_key.keycode, KeyState::Pressed);
		if !self.config.character_hold.is_zero() {
			thread::sleep(self.config.character_hold);
		}
		self.send_key(mapped_key.keycode, KeyState::Released);
	}

	fn transition_modifiers(&mut self, modifiers: Modifiers) {
		let target_modifiers = ActiveModifiers {
			shift: modifiers.shift.map(|shift| shift.keycode),
			altgr: modifiers.altgr.map(|altgr| altgr.keycode),
		};

		self.active_modifiers.altgr =
			self.transition_modifier(self.active_modifiers.altgr, target_modifiers.altgr);
		self.active_modifiers.shift =
			self.transition_modifier(self.active_modifiers.shift, target_modifiers.shift);
	}

	fn transition_modifier(
		&self,
		previous: Option<PlatformKeycode>,
		next: Option<PlatformKeycode>,
	) -> Option<PlatformKeycode> {
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

	fn send_key(&self, keycode: PlatformKeycode, state: KeyState) {
		self.fake_input.keyboard_key(keycode.into(), state.into());
	}
}
