use std::thread;

use thiserror::Error;
use wayland_client::{backend::WaylandError, protocol::wl_keyboard::KeyState};
use wayland_protocols_plasma::fake_input::client::org_kde_kwin_fake_input::OrgKdeKwinFakeInput;

use crate::{KwtyprConfig, Ready, UnicodeFallback};
use crate::{
	wayland::WaylandSession,
	xkb::{
		Xkb,
		mapping::{CharacterMappingError, MappedKey, Modifiers, PlatformKeycode},
		unicode_fallback::UnicodeFallbackKeys,
	},
};

pub struct Typer<'a> {
	wayland: &'a WaylandSession,
	fake_input: &'a OrgKdeKwinFakeInput,
	xkb: &'a Xkb,
	config: &'a KwtyprConfig,
	unicode_fallback: &'a UnicodeFallback,
	active_modifiers: ActiveModifiers,
}

#[derive(Debug, Error)]
enum TypeCharError {
	#[error(transparent)]
	Mapping(#[from] CharacterMappingError),
	#[error(transparent)]
	Flush(#[from] WaylandError),
}

#[derive(Default)]
struct ActiveModifiers {
	ctrl: Option<PlatformKeycode>,
	shift: Option<PlatformKeycode>,
	altgr: Option<PlatformKeycode>,
}

pub enum TypingOutcome {
	Complete,
	Partial { failed_characters: usize },
}

impl<'a> Typer<'a> {
	pub fn new(wayland: &'a WaylandSession, state: &'a Ready, config: &'a KwtyprConfig) -> Self {
		let Ready {
			fake_input,
			xkb,
			unicode_fallback,
		} = state;
		Self {
			wayland,
			fake_input,
			xkb,
			config,
			unicode_fallback,
			active_modifiers: ActiveModifiers::default(),
		}
	}

	pub fn type_text(&mut self, text: &str) -> Result<TypingOutcome, WaylandError> {
		if !self.config.initial_delay.is_zero() {
			thread::sleep(self.config.initial_delay);
		}

		let mut failed_characters = 0;
		let mut queued_characters = 0;
		let has_character_delay = !self.config.character_delay.is_zero();

		for character in text.chars() {
			match self.type_char(character) {
				Ok(()) => {
					queued_characters += 1;
				}
				Err(TypeCharError::Mapping(error)) => {
					eprintln!(
						"Failed to type character {character:?} with the current layout: {error}"
					);
					failed_characters += 1;
				}
				Err(TypeCharError::Flush(error)) => return Err(error),
			}

			if self.should_flush_after_character(queued_characters) || has_character_delay {
				self.wayland.flush_blocking()?;
				queued_characters = 0;
			}

			if has_character_delay {
				thread::sleep(self.config.character_delay);
			}
		}

		self.release_all_modifiers();

		// Do a final flush of queued key events (loop + modifiers) in the flush_every case
		if self.config.flush_every.is_some() {
			self.wayland.flush_blocking()?;
		}

		Ok(if failed_characters == 0 {
			TypingOutcome::Complete
		} else {
			TypingOutcome::Partial { failed_characters }
		})
	}

	fn type_char(&mut self, character: char) -> Result<(), TypeCharError> {
		match self.xkb.key_for_char(character) {
			Ok(mapped_key) => {
				self.send_mapped_key(&mapped_key)?;
				Ok(())
			}
			Err(error) => match self.unicode_fallback {
				UnicodeFallback::Disabled => Err(error.into()),
				UnicodeFallback::Enabled(unicode_fallback_keys) => {
					self.type_char_with_unicode_fallback(character, unicode_fallback_keys)
				}
			},
		}
	}

	fn send_mapped_key(&mut self, mapped_key: &MappedKey) -> Result<(), WaylandError> {
		self.transition_modifiers(mapped_key.modifiers);
		self.send_key(mapped_key.keycode, KeyState::Pressed);
		if !self.config.key_hold.is_zero() {
			self.wayland.flush_blocking()?;
			thread::sleep(self.config.key_hold);
		}
		self.send_key(mapped_key.keycode, KeyState::Released);
		if !self.config.key_hold.is_zero() {
			self.wayland.flush_blocking()?;
		}
		Ok(())
	}

	fn type_char_with_unicode_fallback(
		&mut self,
		character: char,
		unicode_fallback_keys: &UnicodeFallbackKeys,
	) -> Result<(), TypeCharError> {
		self.send_mapped_key(&unicode_fallback_keys.prefix)?;

		for hex_digit in format!("{:x}", character as u32).chars() {
			let mapped_key = self.xkb.key_for_char(hex_digit)?;
			self.send_mapped_key(&mapped_key)?;
		}

		self.send_mapped_key(&unicode_fallback_keys.confirm)?;
		Ok(())
	}

	fn transition_modifiers(&mut self, modifiers: Modifiers) {
		let target_modifiers = ActiveModifiers {
			ctrl: modifiers.ctrl.map(|ctrl| ctrl.keycode),
			shift: modifiers.shift.map(|shift| shift.keycode),
			altgr: modifiers.altgr.map(|altgr| altgr.keycode),
		};

		self.active_modifiers.ctrl =
			self.transition_modifier(self.active_modifiers.ctrl, target_modifiers.ctrl);
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
		self.active_modifiers.ctrl = self.transition_modifier(self.active_modifiers.ctrl, None);
		self.active_modifiers.altgr = self.transition_modifier(self.active_modifiers.altgr, None);
		self.active_modifiers.shift = self.transition_modifier(self.active_modifiers.shift, None);
	}

	fn send_key(&self, keycode: PlatformKeycode, state: KeyState) {
		self.fake_input.keyboard_key(keycode.into(), state.into());
	}

	fn should_flush_after_character(&self, queued_characters: u32) -> bool {
		self.config
			.flush_every
			.is_some_and(|flush_every| queued_characters >= flush_every.get())
	}
}
