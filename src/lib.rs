use wayland_client::protocol::{wl_keyboard::WlKeyboard, wl_seat::WlSeat};
use wayland_protocols_plasma::fake_input::client::org_kde_kwin_fake_input::OrgKdeKwinFakeInput;

use crate::{wayland::WaylandSession, xkb::Xkb};

mod typing;
mod wayland;
mod xkb;

pub use crate::wayland::WaylandConnectError;

pub struct Kwtypr {
	wayland: WaylandSession<Components>,
	components: Components,
}

impl Kwtypr {
	pub fn new() -> Result<Self, WaylandConnectError> {
		Ok(Self {
			wayland: WaylandSession::new()?,
			components: Components::default(),
		})
	}

	pub fn initialize(&mut self) {
		let queue_handle = self.wayland.event_queue.handle();
		let display = self.wayland.connection.display();
		let _registry = display.get_registry(&queue_handle, ());
		println!("Advertised globals:");
		let _ = self.wayland.event_queue.roundtrip(&mut self.components);

		while !self.all_components_available() {
			let _ = self
				.wayland
				.event_queue
				.blocking_dispatch(&mut self.components);
		}

		let Some(fake_input) = &self.components.fake_input else {
			eprintln!("Fake input interface is not available, cannot authenticate");
			std::process::exit(1);
		};

		fake_input.authenticate("kwtypr".to_owned(), "KDE Virtual Keyboard Input".to_owned());
	}

	fn all_components_available(&self) -> bool {
		self.components.fake_input.is_some()
			&& self.components.seat.is_some()
			&& self.components.keyboard.is_some()
			&& self.components.xkb.is_some()
	}

	pub fn send_text(&mut self, text: &str) {
		let Some(fake_input) = &self.components.fake_input else {
			eprintln!("Cannot send input events because the fake input interface is not available");
			return;
		};

		let Some(xkb_state) = &self.components.xkb else {
			eprintln!("Cannot send input events because XKB state is not available");
			return;
		};

		typing::send_text(fake_input, xkb_state, text);

		self.wayland
			.event_queue
			.roundtrip(&mut self.components)
			.unwrap();
	}
}

#[derive(Default)]
struct Components {
	fake_input: Option<OrgKdeKwinFakeInput>,
	seat: Option<WlSeat>,
	keyboard: Option<WlKeyboard>,
	xkb: Option<Xkb>,
}
