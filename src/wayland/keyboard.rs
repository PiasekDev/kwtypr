use wayland_client::{
	Connection, Dispatch, QueueHandle,
	protocol::{
		wl_keyboard::{self, KeymapFormat, WlKeyboard},
		wl_seat::{self, WlSeat},
	},
};

use crate::{Components, xkb::Xkb};

impl Dispatch<WlSeat, ()> for Components {
	fn event(
		components: &mut Self,
		seat: &WlSeat,
		event: wl_seat::Event,
		_user_data: &(),
		_connection: &Connection,
		qh: &QueueHandle<Components>,
	) {
		let wl_seat::Event::Capabilities { capabilities } = event else {
			return;
		};

		let Ok(capabilities) = capabilities.into_result() else {
			return;
		};

		if capabilities.contains(wl_seat::Capability::Keyboard) {
			components.keyboard.get_or_insert_with(|| {
				let proxy: WlKeyboard = seat.get_keyboard(qh, ());
				println!("Got a keyboard! {:?}", proxy);
				proxy
			});
		} else {
			components.keyboard = None;
		}
	}
}

impl Dispatch<WlKeyboard, ()> for Components {
	fn event(
		components: &mut Self,
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

			components.xkb = Some(xkb_state);
		}
	}
}
