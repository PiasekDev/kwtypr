use wayland_client::{
	Connection, Dispatch, QueueHandle,
	protocol::{
		wl_keyboard::{self, KeymapFormat, WlKeyboard},
		wl_seat::{self, WlSeat},
	},
};

use crate::{KwtyprError, wayland::InitializationState, xkb::Xkb};

impl Dispatch<WlSeat, ()> for InitializationState {
	fn event(
		state: &mut Self,
		seat: &WlSeat,
		event: wl_seat::Event,
		_user_data: &(),
		_connection: &Connection,
		qh: &QueueHandle<InitializationState>,
	) {
		if let InitializationState::Binding(globals) = state
			&& let wl_seat::Event::Capabilities { capabilities } = event
			&& let Ok(capabilities) = capabilities.into_result()
			&& capabilities.contains(wl_seat::Capability::Keyboard)
		{
			globals.keyboard = Some(seat.get_keyboard(qh, ()));
		}
	}
}

impl Dispatch<WlKeyboard, ()> for InitializationState {
	fn event(
		state: &mut Self,
		_: &WlKeyboard,
		event: wl_keyboard::Event,
		_: &(),
		_: &Connection,
		_: &QueueHandle<Self>,
	) {
		if let wl_keyboard::Event::Keymap { format, fd, size } = event
			&& let Ok(KeymapFormat::XkbV1) = format.into_result()
		{
			match Xkb::from_wayland_keymap(fd, size) {
				Ok(xkb_state) => {
					if let InitializationState::Binding(globals) = state {
						globals.xkb = Some(xkb_state);
					}
				}
				Err(err) => *state = InitializationState::Failed(KwtyprError::from(err)),
			}
		}
	}
}
