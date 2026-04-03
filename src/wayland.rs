use std::os::fd::OwnedFd;

use wayland_client::{
	ConnectError,
	protocol::{wl_keyboard::WlKeyboard, wl_seat::WlSeat},
};
use wayland_protocols_plasma::fake_input::client::org_kde_kwin_fake_input::OrgKdeKwinFakeInput;

mod keyboard;
mod registry;

pub struct WaylandSession {
	pub connection: wayland_client::Connection,
	pub event_queue: wayland_client::EventQueue<Bindings>,
	pub bindings: Bindings,
}

#[derive(Default)]
pub struct Bindings {
	pub fake_input: Option<OrgKdeKwinFakeInput>,
	pub seat: Option<WlSeat>,
	pub keyboard: Option<WlKeyboard>,
	pub keymap_fd: Option<KeymapFd>,
}

pub struct KeymapFd {
	pub fd: OwnedFd,
	pub size: u32,
}

impl Bindings {
	pub fn all_bound(&self) -> bool {
		self.fake_input.is_some()
			&& self.seat.is_some()
			&& self.keyboard.is_some()
			&& self.keymap_fd.is_some()
	}
}

impl WaylandSession {
	pub fn new() -> Result<Self, ConnectError> {
		let connection = wayland_client::Connection::connect_to_env()?;
		let event_queue = connection.new_event_queue();
		Ok(Self {
			connection,
			event_queue,
			bindings: Bindings::default(),
		})
	}
}
