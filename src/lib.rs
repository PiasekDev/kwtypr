use thiserror::Error;
use wayland_client::{ConnectError, DispatchError};
use wayland_protocols_plasma::fake_input::client::org_kde_kwin_fake_input::OrgKdeKwinFakeInput;

use crate::{
	wayland::{Globals, InitializationState, WaylandSession},
	xkb::Xkb,
};

mod typing;
mod wayland;
mod xkb;

pub use crate::xkb::XkbInitError;

#[derive(Debug, Error)]
pub enum KwtyprError {
	#[error("failed to connect to the Wayland compositor")]
	WaylandConnect(#[from] ConnectError),
	#[error("failed to dispatch Wayland events")]
	WaylandDispatch(#[from] DispatchError),
	#[error("failed to initialize XKB state from the provided keymap")]
	XkbInit(#[from] XkbInitError),
}

pub struct Kwtypr<State> {
	wayland: WaylandSession,
	state: State,
}

pub struct Uninitialized;
pub struct Ready {
	components: Components,
}

impl Kwtypr<Uninitialized> {
	pub fn new() -> Result<Self, KwtyprError> {
		Ok(Self {
			wayland: WaylandSession::new()?,
			state: Uninitialized,
		})
	}

	pub fn initialize(mut self) -> Result<Kwtypr<Ready>, KwtyprError> {
		let queue_handle = self.wayland.event_queue.handle();
		let display = self.wayland.connection.display();
		let _registry = display.get_registry(&queue_handle, ());

		while !self.wayland.state.all_globals_bound() {
			self.wayland
				.event_queue
				.blocking_dispatch(&mut self.wayland.state)?;

			match self.wayland.state {
				InitializationState::Binding(_) => continue,
				InitializationState::Failed(error) => return Err(error),
			}
		}

		let components: Components = self
			.wayland
			.state
			.take_bound_globals()
			.expect("bound globals to be available after initialization")
			.try_into()
			.expect("all components to be bound");

		components
			.fake_input
			.authenticate("kwtypr".to_owned(), "KDE Virtual Keyboard Input".to_owned());

		Ok(Kwtypr {
			wayland: self.wayland,
			state: Ready { components },
		})
	}
}

impl Kwtypr<Ready> {
	pub fn send_text(&mut self, text: &str) {
		let Components { fake_input, xkb } = &self.state.components;
		typing::send_text(fake_input, xkb, text);

		self.wayland
			.event_queue
			.roundtrip(&mut self.wayland.state)
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

impl TryFrom<Globals> for Components {
	type Error = UnintializedComponentsError;

	fn try_from(value: Globals) -> Result<Self, Self::Error> {
		Ok(Self {
			fake_input: value.fake_input.ok_or(UnintializedComponentsError)?,
			xkb: value.xkb.ok_or(UnintializedComponentsError)?,
		})
	}
}
