use wayland_client::{
	Connection, Dispatch, QueueHandle,
	protocol::{
		wl_keyboard::{self, KeymapFormat, WlKeyboard},
		wl_seat::{self, WlSeat},
	},
};

use crate::wayland::{Bindings, KeymapFd};

impl Dispatch<WlSeat, ()> for Bindings {
	fn event(
		bindings: &mut Self,
		seat: &WlSeat,
		event: wl_seat::Event,
		_user_data: &(),
		_connection: &Connection,
		qh: &QueueHandle<Bindings>,
	) {
		if let wl_seat::Event::Capabilities { capabilities } = event
			&& let Ok(capabilities) = capabilities.into_result()
			&& capabilities.contains(wl_seat::Capability::Keyboard)
		{
			bindings.keyboard = Some(seat.get_keyboard(qh, ()));
		}
	}
}

impl Dispatch<WlKeyboard, ()> for Bindings {
	fn event(
		bindings: &mut Self,
		_: &WlKeyboard,
		event: wl_keyboard::Event,
		_: &(),
		_: &Connection,
		_: &QueueHandle<Self>,
	) {
		if let wl_keyboard::Event::Keymap { format, fd, size } = event
			&& let Ok(KeymapFormat::XkbV1) = format.into_result()
		{
			bindings.keymap_fd = Some(KeymapFd { fd, size });
		}
	}
}
