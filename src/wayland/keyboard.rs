use wayland_client::{
	Connection, Dispatch, QueueHandle,
	protocol::{
		wl_keyboard::{self, KeymapFormat, WlKeyboard},
		wl_seat::{self, WlSeat},
	},
};

use crate::{wayland::BoundGlobals, xkb::Xkb};

impl Dispatch<WlSeat, ()> for BoundGlobals {
	fn event(
		globals: &mut Self,
		seat: &WlSeat,
		event: wl_seat::Event,
		_user_data: &(),
		_connection: &Connection,
		qh: &QueueHandle<BoundGlobals>,
	) {
		if let wl_seat::Event::Capabilities { capabilities } = event
			&& let Ok(capabilities) = capabilities.into_result()
			&& capabilities.contains(wl_seat::Capability::Keyboard)
		{
			globals.keyboard = Some(seat.get_keyboard(qh, ()));
		}
	}
}

impl Dispatch<WlKeyboard, ()> for BoundGlobals {
	fn event(
		globals: &mut Self,
		_: &WlKeyboard,
		event: wl_keyboard::Event,
		_: &(),
		_: &Connection,
		_: &QueueHandle<Self>,
	) {
		if let wl_keyboard::Event::Keymap { format, fd, size } = event
			&& let Ok(KeymapFormat::XkbV1) = format.into_result()
		{
			let Ok(xkb_state) = Xkb::from_wayland_keymap(fd, size) else {
				eprintln!("Failed to initialize XKB state from the provided keymap");
				std::process::exit(1);
			};

			globals.xkb = Some(xkb_state);
		}
	}
}
