use xkbcommon::xkb;

use crate::xkb::mapping::PlatformKeycode;

pub struct AvailableModifiers(pub Modifiers);

impl AvailableModifiers {
	pub fn from_keymap(keymap: &xkb::Keymap) -> Self {
		Self(Modifiers {
			ctrl: ModifierMapping::from_name(keymap, xkb::MOD_NAME_CTRL),
			shift: ModifierMapping::from_name(keymap, xkb::MOD_NAME_SHIFT),
			altgr: ModifierMapping::from_name(keymap, xkb::MOD_NAME_ISO_LEVEL3_SHIFT),
		})
	}

	pub fn modifiers_for_mask(&self, mask: xkb::ModMask) -> Modifiers {
		let ctrl = self
			.0
			.ctrl
			.filter(|mapping| mask.has_modifier(mapping.mod_mask));
		let shift = self
			.0
			.shift
			.filter(|mapping| mask.has_modifier(mapping.mod_mask));
		let altgr = self
			.0
			.altgr
			.filter(|mapping| mask.has_modifier(mapping.mod_mask));
		Modifiers { ctrl, shift, altgr }
	}

	pub fn can_represent(&self, modifier_mask: xkb::ModMask) -> bool {
		self.mask() & modifier_mask == modifier_mask
	}

	fn mask(&self) -> xkb::ModMask {
		let mut mask = 0;
		if let Some(ctrl_mapping) = self.0.ctrl {
			mask |= ctrl_mapping.mod_mask
		}
		if let Some(shift_mapping) = self.0.shift {
			mask |= shift_mapping.mod_mask
		}
		if let Some(altgr_mapping) = self.0.altgr {
			mask |= altgr_mapping.mod_mask
		}
		mask
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Modifiers {
	pub ctrl: Option<ModifierMapping>,
	pub shift: Option<ModifierMapping>,
	pub altgr: Option<ModifierMapping>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModifierMapping {
	pub mod_mask: xkb::ModMask,
	pub keycode: PlatformKeycode,
}

impl ModifierMapping {
	fn from_name(keymap: &xkb::Keymap, modifier_name: &str) -> Option<Self> {
		let mod_mask = modifier_mask(keymap, modifier_name)?;
		let keycode = find_keycode_for_modifier_mask(keymap, mod_mask)?;
		Some(Self { mod_mask, keycode })
	}
}

fn modifier_mask(keymap: &xkb::Keymap, modifier_name: &str) -> Option<xkb::ModMask> {
	let modifier_index = keymap.mod_get_index(modifier_name);
	if modifier_index == xkb::MOD_INVALID || modifier_index >= xkb::ModMask::BITS {
		return None;
	}

	Some(1u32 << modifier_index)
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
