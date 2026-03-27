use thiserror::Error;
use xkbcommon::xkb;

use super::Xkb;
pub use super::modifier::Modifiers;

pub struct MappedKey {
	pub keycode: PlatformKeycode,
	pub modifiers: Modifiers,
}

#[derive(Debug, Error)]
pub enum XkbMappingError {
	#[error("character {character:?} does not map to an XKB keysym")]
	NoSymbol { character: char },
	#[error("no key in the active layout produces {character:?}")]
	NoKeyMatch { character: char },
	#[error("key {keycode:?} at level {level} for {character:?} requires unsupported modifiers")]
	UnsupportedModifiers {
		character: char,
		keycode: xkb::Keycode,
		level: xkb::LevelIndex,
	},
	#[error("XKB keycode {keycode:?} for {character:?} cannot be converted to a platform keycode", keycode = .source.0)]
	InvalidPlatformKeycode {
		character: char,
		source: PlatformKeycodeFromXkbError,
	},
}

impl Xkb {
	pub fn key_for_char(&self, character: char) -> Result<MappedKey, XkbMappingError> {
		let char_utf32 = character as u32;
		let keysym = xkb::utf32_to_keysym(char_utf32);
		if keysym == xkb::keysyms::KEY_NoSymbol.into() {
			return Err(XkbMappingError::NoSymbol { character });
		}

		let keymap = self.state.get_keymap();
		let layout = self.state.serialize_layout(xkb::STATE_LAYOUT_EFFECTIVE);
		let keycode_match = self
			.find_keycode_match(&keymap, layout, keysym)
			.ok_or(XkbMappingError::NoKeyMatch { character })?;
		let modifiers =
			Modifiers::for_key_level(&keymap, layout, keycode_match.keycode, keycode_match.level)
				.ok_or(XkbMappingError::UnsupportedModifiers {
				character,
				keycode: keycode_match.keycode,
				level: keycode_match.level,
			})?;
		let keycode = PlatformKeycode::try_from(keycode_match.keycode).map_err(|e| {
			XkbMappingError::InvalidPlatformKeycode {
				character,
				source: e,
			}
		})?;

		Ok(MappedKey { keycode, modifiers })
	}
}

struct KeycodeMatch {
	keycode: xkb::Keycode,
	level: xkb::LevelIndex,
}

impl Xkb {
	fn find_keycode_match(
		&self,
		keymap: &xkb::Keymap,
		layout: xkb::LayoutIndex,
		keysym: xkb::Keysym,
	) -> Option<KeycodeMatch> {
		let min_keycode = keymap.min_keycode().raw();
		let max_keycode = keymap.max_keycode().raw();

		for keycode in min_keycode..=max_keycode {
			let keycode = xkb::Keycode::new(keycode);
			let num_levels = keymap.num_levels_for_key(keycode, layout);
			if num_levels == 0 {
				continue;
			}

			for level in 0..num_levels {
				let syms = keymap.key_get_syms_by_level(keycode, layout, level);

				if syms == [keysym] {
					return Some(KeycodeMatch { keycode, level });
				}
			}
		}

		None
	}
}

/// A platform-specific key code used in Wayland keyboard events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlatformKeycode(u32);

impl PlatformKeycode {
	pub fn raw(self) -> u32 {
		self.0
	}
}

impl From<PlatformKeycode> for u32 {
	fn from(value: PlatformKeycode) -> Self {
		value.0
	}
}

#[derive(Debug, Error)]
#[error("XKB keycode {0:?} is out of range for conversion to a platform keycode")]
pub struct PlatformKeycodeFromXkbError(xkb::Keycode);

impl TryFrom<xkb::Keycode> for PlatformKeycode {
	type Error = PlatformKeycodeFromXkbError;

	/// Converts an XKB keycode into the platform keycode used by Wayland key events.
	///
	/// The compositor-provided XKB keymap uses XKB keycodes, while key events are
	/// sent using platform keycodes. For the same physical key, the XKB
	/// keycode is the platform keycode plus `8`, so this conversion subtracts `8`.
	///
	/// Returns an error if the XKB keycode is below the representable platform-keycode range.
	fn try_from(value: xkb::Keycode) -> Result<Self, Self::Error> {
		value
			.raw()
			.checked_sub(8)
			.map(PlatformKeycode)
			.ok_or(PlatformKeycodeFromXkbError(value))
	}
}
