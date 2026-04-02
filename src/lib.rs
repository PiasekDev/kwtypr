use std::mem;

use thiserror::Error;
use wayland_protocols_plasma::fake_input::client::org_kde_kwin_fake_input::OrgKdeKwinFakeInput;

use crate::{
	wayland::{BoundGlobals, WaylandSession},
	xkb::Xkb,
};

mod typing;
mod wayland;
mod xkb;

pub use crate::wayland::WaylandConnectError;

pub struct Kwtypr<State> {
	wayland: WaylandSession,
	state: State,
}

pub struct Uninitialized;
pub struct Ready {
	components: Components,
}

impl Kwtypr<Uninitialized> {
	pub fn new() -> Result<Self, WaylandConnectError> {
		Ok(Self {
			wayland: WaylandSession::new()?,
			state: Uninitialized,
		})
	}

	pub fn initialize(mut self) -> Kwtypr<Ready> {
		let queue_handle = self.wayland.event_queue.handle();
		let display = self.wayland.connection.display();
		let _registry = display.get_registry(&queue_handle, ());

		while !self.wayland.globals.all_bound() {
			let _ = self
				.wayland
				.event_queue
				.blocking_dispatch(&mut self.wayland.globals);
		}

		let components: Components = mem::take(&mut self.wayland.globals)
			.try_into()
			.expect("all components to be bound");

		components
			.fake_input
			.authenticate("kwtypr".to_owned(), "KDE Virtual Keyboard Input".to_owned());

		Kwtypr {
			wayland: self.wayland,
			state: Ready { components },
		}
	}
}

impl Kwtypr<Ready> {
	pub fn send_text(&mut self, text: &str) {
		let Components { fake_input, xkb } = &self.state.components;
		typing::send_text(fake_input, xkb, text);

		self.wayland
			.event_queue
			.roundtrip(&mut self.wayland.globals)
			.unwrap();
	}
}

struct Components {
	fake_input: OrgKdeKwinFakeInput,
	xkb: Xkb,
}

#[derive(Debug, Error)]
#[error("cannot transition to Ready state because some components are missing")]
struct UnintializedComponentsError;

impl TryFrom<BoundGlobals> for Components {
	type Error = UnintializedComponentsError;

	fn try_from(value: BoundGlobals) -> Result<Self, Self::Error> {
		Ok(Self {
			fake_input: value.fake_input.ok_or(UnintializedComponentsError)?,
			xkb: value.xkb.ok_or(UnintializedComponentsError)?,
		})
	}
}
