use thiserror::Error;
use xkbcommon::xkb;

use super::{
	Xkb,
	mapping::{KeyMappingError, MappedKey, Modifiers},
	modifier::AvailableModifiers,
};

pub struct UnicodeFallbackKeys {
	pub prefix: MappedKey,
	pub confirm: MappedKey,
}

#[derive(Debug, Error)]
pub enum UnicodeFallbackInitError {
	#[error(
		"Unicode fallback requires both Ctrl and Shift modifier keys in the current keyboard layout"
	)]
	MissingModifiers,
	#[error("Unicode fallback prefix key ('u') is unavailable in the current keyboard layout")]
	PrefixKey(#[source] KeyMappingError),
	#[error(
		"Unicode fallback confirmation key (Space) is unavailable in the current keyboard layout"
	)]
	ConfirmKey(#[source] KeyMappingError),
}

impl Xkb {
	pub fn unicode_fallback_keys(&self) -> Result<UnicodeFallbackKeys, UnicodeFallbackInitError> {
		let fallback_modifiers = unicode_fallback_modifiers(&self.available_modifiers)
			.ok_or(UnicodeFallbackInitError::MissingModifiers)?;
		let mut prefix = self
			.key_for_keysym(xkb::keysyms::KEY_u.into())
			.map_err(UnicodeFallbackInitError::PrefixKey)?;
		prefix.modifiers = Modifiers {
			ctrl: fallback_modifiers.ctrl,
			shift: fallback_modifiers.shift,
			..prefix.modifiers
		};
		let confirm = self
			.key_for_keysym(xkb::keysyms::KEY_space.into())
			.map_err(UnicodeFallbackInitError::ConfirmKey)?;
		Ok(UnicodeFallbackKeys { prefix, confirm })
	}
}

fn unicode_fallback_modifiers(available_modifiers: &AvailableModifiers) -> Option<Modifiers> {
	Some(Modifiers {
		ctrl: Some(available_modifiers.0.ctrl?),
		shift: Some(available_modifiers.0.shift?),
		altgr: None,
	})
}
