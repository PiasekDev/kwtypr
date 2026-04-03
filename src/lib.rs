use std::mem;

use thiserror::Error;
use wayland_client::{ConnectError, DispatchError};
use wayland_protocols_plasma::fake_input::client::org_kde_kwin_fake_input::OrgKdeKwinFakeInput;

use crate::{
	wayland::{Bindings, WaylandSession},
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

		while !self.wayland.bindings.all_bound() {
			self.wayland
				.event_queue
				.blocking_dispatch(&mut self.wayland.bindings)?;
		}

		let bindings = mem::take(&mut self.wayland.bindings);
		let components = match bindings.into_components() {
			Ok(components) => components,
			Err(IntoComponentsError::XkbInit(err)) => return Err(err.into()),
			Err(IntoComponentsError::UninitializedFields) => {
				panic!("bindings should be fully initialized after all_bound()")
			}
		};

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
			.roundtrip(&mut self.wayland.bindings)
			.unwrap();
	}
}

struct Components {
	fake_input: OrgKdeKwinFakeInput,
	xkb: Xkb,
}

enum IntoComponentsError {
	UninitializedFields,
	XkbInit(XkbInitError),
}

impl Bindings {
	fn into_components(self) -> Result<Components, IntoComponentsError> {
		let fake_input = self
			.fake_input
			.ok_or(IntoComponentsError::UninitializedFields)?;
		let keymap_fd = self
			.keymap_fd
			.ok_or(IntoComponentsError::UninitializedFields)?;
		let xkb = Xkb::from_wayland_keymap(keymap_fd.fd, keymap_fd.size)
			.map_err(IntoComponentsError::XkbInit)?;
		Ok(Components { fake_input, xkb })
	}
}
