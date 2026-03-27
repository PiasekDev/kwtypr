use xkbcommon::xkb;

use crate::xkb::mapping::PlatformKeycode;

// Matches the buffer size used in the upstream xkbcommon Rust example.
const MAX_MODIFIER_MASKS: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Modifiers {
	pub shift: Option<ModifierMapping>,
	pub altgr: Option<ModifierMapping>,
}

impl Modifiers {
	pub fn for_key_level(
		keymap: &xkb::Keymap,
		layout: xkb::LayoutIndex,
		keycode: xkb::Keycode,
		level: xkb::LevelIndex,
	) -> Option<Self> {
		let mut modifier_masks = [xkb::ModMask::default(); MAX_MODIFIER_MASKS];
		let num_masks = keymap.key_get_mods_for_level(keycode, layout, level, &mut modifier_masks);

		let shift_modifier_mask = modifier_mask(keymap, xkb::MOD_NAME_SHIFT);
		let altgr_modifier_mask = modifier_mask(keymap, xkb::MOD_NAME_ISO_LEVEL3_SHIFT);
		let supported_modifiers_mask = shift_modifier_mask | altgr_modifier_mask;

		for &modifier_mask in &modifier_masks[..num_masks.min(modifier_masks.len())] {
			let uses_only_supported_modifiers =
				(modifier_mask & supported_modifiers_mask) == modifier_mask;

			if !uses_only_supported_modifiers {
				continue;
			}

			if let Some(modifiers) = try_build_modifiers(
				keymap,
				modifier_mask,
				shift_modifier_mask,
				altgr_modifier_mask,
			) {
				return Some(modifiers);
			}
		}

		None
	}
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

fn try_build_modifiers(
	keymap: &xkb::Keymap,
	modifier_mask: xkb::ModMask,
	shift_modifier_mask: xkb::ModMask,
	altgr_modifier_mask: xkb::ModMask,
) -> Option<Modifiers> {
	let shift = if modifier_mask.has_modifier(shift_modifier_mask) {
		Some(ModifierMapping::from_mask(keymap, shift_modifier_mask)?)
	} else {
		None
	};
	let altgr = if modifier_mask.has_modifier(altgr_modifier_mask) {
		Some(ModifierMapping::from_mask(keymap, altgr_modifier_mask)?)
	} else {
		None
	};

	Some(Modifiers { shift, altgr })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModifierMapping {
	mask: xkb::ModMask,
	keycode: PlatformKeycode,
}

impl ModifierMapping {
	pub fn keycode(self) -> PlatformKeycode {
		self.keycode
	}

	fn from_mask(keymap: &xkb::Keymap, mask: xkb::ModMask) -> Option<Self> {
		if mask == 0 {
			return None;
		}

		let keycode = find_keycode_for_modifier_mask(keymap, mask)?;
		Some(Self { mask, keycode })
	}
}

pub fn find_keycode_for_modifier_mask(
	keymap: &xkb::Keymap,
	target_mask: xkb::ModMask,
) -> Option<PlatformKeycode> {
	if target_mask == 0 {
		return None;
	}

	let mut state = xkb::State::new(keymap);
	let min_keycode = keymap.min_keycode().raw();
	let max_keycode = keymap.max_keycode().raw();

	for keycode in (min_keycode..=max_keycode).map(xkb::Keycode::new) {
		state.update_key(keycode, xkb::KeyDirection::Down);
		let active_modifiers = state.serialize_mods(xkb::STATE_MODS_EFFECTIVE);
		state.update_key(keycode, xkb::KeyDirection::Up);

		if (active_modifiers & target_mask) == target_mask {
			return PlatformKeycode::try_from(keycode).ok();
		}
	}

	None
}

trait ModMaskExt {
	fn has_modifier(self, modifier_mask: xkb::ModMask) -> bool;
}

impl ModMaskExt for xkb::ModMask {
	fn has_modifier(self, modifier_mask: xkb::ModMask) -> bool {
		self & modifier_mask != 0
	}
}
