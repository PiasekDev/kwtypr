use thiserror::Error;
use wayland_client::{
	ConnectError,
	protocol::{wl_keyboard::WlKeyboard, wl_seat::WlSeat},
};
use wayland_protocols_plasma::fake_input::client::org_kde_kwin_fake_input::OrgKdeKwinFakeInput;

use crate::xkb::Xkb;

mod keyboard;
mod registry;

#[derive(Debug, Error)]
#[error("failed to connect to the Wayland compositor")]
pub struct WaylandConnectError(#[from] ConnectError);

pub struct WaylandSession {
	pub connection: wayland_client::Connection,
	pub event_queue: wayland_client::EventQueue<BoundGlobals>,
	pub globals: BoundGlobals,
}

#[derive(Default)]
pub struct BoundGlobals {
	pub fake_input: Option<OrgKdeKwinFakeInput>,
	pub seat: Option<WlSeat>,
	pub keyboard: Option<WlKeyboard>,
	pub xkb: Option<Xkb>,
}

impl BoundGlobals {
	pub fn all_bound(&self) -> bool {
		self.fake_input.is_some()
			&& self.seat.is_some()
			&& self.keyboard.is_some()
			&& self.xkb.is_some()
	}
}

impl WaylandSession {
	pub fn new() -> Result<Self, WaylandConnectError> {
		let connection = wayland_client::Connection::connect_to_env()?;
		let event_queue = connection.new_event_queue();
		Ok(Self {
			connection,
			event_queue,
			globals: BoundGlobals::default(),
		})
	}
}
