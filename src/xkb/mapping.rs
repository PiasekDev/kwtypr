use thiserror::Error;
use xkbcommon::xkb;

use super::Xkb;

struct KeycodeMatch {
	keycode: xkb::Keycode,
	level: xkb::LevelIndex,
}

pub struct MappedKey {
	pub evdev_code: u32,
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
	#[error("keycode {keycode:?} for {character:?} cannot be converted to an evdev code")]
	InvalidEvdevKeycode {
		character: char,
		keycode: xkb::Keycode,
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
		let evdev_code = xkb_to_evdev(keycode_match.keycode.raw()).ok_or(
			XkbMappingError::InvalidEvdevKeycode {
				character,
				keycode: keycode_match.keycode,
			},
		)?;

		Ok(MappedKey {
			evdev_code,
			modifiers,
		})
	}

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

// Wayland/XKB client keycodes are offset by +8 compared to evdev keycodes.
fn xkb_to_evdev(keycode: u32) -> Option<u32> {
	keycode.checked_sub(8)
}
