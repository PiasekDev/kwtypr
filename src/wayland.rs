use std::mem;

use wayland_client::{
	ConnectError,
	protocol::{wl_keyboard::WlKeyboard, wl_seat::WlSeat},
};
use wayland_protocols_plasma::fake_input::client::org_kde_kwin_fake_input::OrgKdeKwinFakeInput;

use crate::{KwtyprError, xkb::Xkb};

mod keyboard;
mod registry;

pub struct WaylandSession {
	pub connection: wayland_client::Connection,
	pub event_queue: wayland_client::EventQueue<InitializationState>,
	pub state: InitializationState,
}

pub enum InitializationState {
	Binding(Globals),
	Failed(KwtyprError),
}

#[derive(Default)]
pub struct Globals {
	pub fake_input: Option<OrgKdeKwinFakeInput>,
	pub seat: Option<WlSeat>,
	pub keyboard: Option<WlKeyboard>,
	pub xkb: Option<Xkb>,
}

impl Globals {
	pub fn all_bound(&self) -> bool {
		self.fake_input.is_some()
			&& self.seat.is_some()
			&& self.keyboard.is_some()
			&& self.xkb.is_some()
	}
}

impl Default for InitializationState {
	fn default() -> Self {
		Self::Binding(Globals::default())
	}
}

impl InitializationState {
	pub fn all_globals_bound(&self) -> bool {
		matches!(self, Self::Binding(globals) if globals.all_bound())
	}

	pub fn take_bound_globals(&mut self) -> Option<Globals> {
		if let Self::Binding(globals) = self
			&& globals.all_bound()
		{
			Some(mem::take(globals))
		} else {
			None
		}
	}
}

impl WaylandSession {
	pub fn new() -> Result<Self, ConnectError> {
		let connection = wayland_client::Connection::connect_to_env()?;
		let event_queue = connection.new_event_queue();
		Ok(Self {
			connection,
			event_queue,
			state: InitializationState::default(),
		})
	}
}
