use thiserror::Error;
use xkbcommon::xkb;

use super::Xkb;

pub struct MappedKey {
	pub raw_keycode: RawKeycode,
	pub modifiers: Modifiers,
}

pub struct Modifiers {
	pub shift: bool,
	pub altgr: bool,
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
	#[error("XKB keycode {:?} for {character:?} cannot be converted to a raw keycode", .source.0)]
	InvalidRawKeycode {
		character: char,
		source: RawKeycodeFromXkbError,
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
		let modifiers = modifiers_for_key_level(&keymap, layout, &keycode_match).ok_or(
			XkbMappingError::UnsupportedModifiers {
				character,
				keycode: keycode_match.keycode,
				level: keycode_match.level,
			},
		)?;
		let raw_keycode = RawKeycode::try_from(keycode_match.keycode).map_err(|e| {
			XkbMappingError::InvalidRawKeycode {
				character,
				source: e,
			}
		})?;

		Ok(MappedKey {
			raw_keycode,
			modifiers,
		})
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

// Matches the scratch buffer size used in the upstream xkbcommon Rust example.
const MAX_MODIFIER_MASKS: usize = 100;

fn modifiers_for_key_level(
	keymap: &xkb::Keymap,
	layout: xkb::LayoutIndex,
	keycode_match: &KeycodeMatch,
) -> Option<Modifiers> {
	let mut masks = [xkb::ModMask::default(); MAX_MODIFIER_MASKS];
	let num_masks = keymap.key_get_mods_for_level(
		keycode_match.keycode,
		layout,
		keycode_match.level,
		&mut masks,
	);

	let shift_mask = modifier_mask(keymap, xkb::MOD_NAME_SHIFT);
	let altgr_mask = modifier_mask(keymap, xkb::MOD_NAME_ISO_LEVEL3_SHIFT);
	let unsupported_modifiers_mask = !(shift_mask | altgr_mask);

	for mask in &masks[..num_masks.min(masks.len())] {
		if has_unsupported_modifiers(*mask, unsupported_modifiers_mask) {
			continue;
		}

		return Some(Modifiers {
			shift: has_modifier(*mask, shift_mask),
			altgr: has_modifier(*mask, altgr_mask),
		});
	}

	None
}

fn modifier_mask(keymap: &xkb::Keymap, modifier_name: &str) -> xkb::ModMask {
	let modifier_index = keymap.mod_get_index(modifier_name);
	mod_mask_bit(modifier_index).unwrap_or(0)
}

fn mod_mask_bit(index: xkb::ModIndex) -> Option<xkb::ModMask> {
	if index == xkb::MOD_INVALID || index >= xkb::ModMask::BITS {
		return None;
	}

	Some(1u32 << index)
}

fn has_unsupported_modifiers(mask: xkb::ModMask, unsupported_modifiers_mask: xkb::ModMask) -> bool {
	(mask & unsupported_modifiers_mask) != 0
}

fn has_modifier(mask: xkb::ModMask, modifier_mask: xkb::ModMask) -> bool {
	mask & modifier_mask != 0
}

/// A platform-specific key code that can be interpreted by feeding it to the keyboard mapping
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RawKeycode(u32);

impl RawKeycode {
	pub fn raw(self) -> u32 {
		self.0
	}
}

impl From<RawKeycode> for u32 {
	fn from(value: RawKeycode) -> Self {
		value.0
	}
}

#[derive(Debug, Error)]
#[error("XKB keycode {0:?} is out of range for conversion to a raw keycode")]
pub struct RawKeycodeFromXkbError(xkb::Keycode);

/// Converts an XKB keycode into the raw keycode used by Wayland key events.
///
/// The compositor-provided XKB keymap uses XKB keycodes, while key events are
/// sent using raw platform keycodes. For the same physical key, the XKB
/// keycode is the raw keycode plus `8`, so this conversion subtracts `8`.
///
/// Returns an error if the XKB keycode is below the representable raw-keycode range.
impl TryFrom<xkb::Keycode> for RawKeycode {
	type Error = RawKeycodeFromXkbError;

	fn try_from(value: xkb::Keycode) -> Result<Self, Self::Error> {
		value
			.raw()
			.checked_sub(8)
			.map(RawKeycode)
			.ok_or(RawKeycodeFromXkbError(value))
	}
}
