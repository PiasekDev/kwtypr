use std::{mem, time::Duration};

use thiserror::Error;
use wayland_client::{ConnectError, DispatchError, backend::WaylandError};
use wayland_protocols_plasma::fake_input::client::org_kde_kwin_fake_input::OrgKdeKwinFakeInput;

use crate::{
	typing::Typer,
	wayland::{Bindings, WaylandInitializeError, WaylandSession},
	xkb::{
		Xkb,
		unicode_fallback::{UnicodeFallbackInitError, UnicodeFallbackKeys},
	},
};

mod typing;
mod wayland;
mod xkb;

pub use crate::xkb::XkbInitError;

pub struct Kwtypr<State> {
	config: KwtyprConfig,
	wayland: WaylandSession,
	state: State,
}

pub struct KwtyprConfig {
	pub ready_timeout: Option<Duration>,
	pub character_delay: Duration,
	pub key_hold: Duration,
	pub unicode_fallback: bool,
}

pub struct Uninitialized;
pub struct Ready {
	fake_input: OrgKdeKwinFakeInput,
	xkb: Xkb,
	unicode_fallback: UnicodeFallback,
}

enum UnicodeFallback {
	Disabled,
	Enabled(UnicodeFallbackKeys),
}

#[derive(Debug, Error)]
pub enum InitializeError {
	#[error("failed to initialize the Wayland session")]
	WaylandInitialize(#[from] WaylandInitializeError),
	#[error("failed to initialize XKB state from the provided keymap")]
	XkbInit(#[from] XkbInitError),
	#[error(transparent)]
	UnicodeFallbackInit(#[from] UnicodeFallbackInitError),
}

impl Kwtypr<Uninitialized> {
	pub fn with_config(config: KwtyprConfig) -> Result<Self, ConnectError> {
		Ok(Self {
			config,
			wayland: WaylandSession::new()?,
			state: Uninitialized,
		})
	}

	pub fn initialize(mut self) -> Result<Kwtypr<Ready>, InitializeError> {
		let queue_handle = self.wayland.event_queue.handle();
		let display = self.wayland.connection.display();
		let _registry = display.get_registry(&queue_handle, ());

		self.wayland.wait_until_ready(self.config.ready_timeout)?;

		let bindings = mem::take(&mut self.wayland.bindings);
		let ready = match bindings.into_ready(&self.config) {
			Ok(ready) => ready,
			Err(IntoReadyError::XkbInit(err)) => return Err(err.into()),
			Err(IntoReadyError::UnicodeFallbackInit(err)) => return Err(err.into()),
			Err(IntoReadyError::UninitializedFields) => {
				panic!("bindings should be fully initialized after all_bound()")
			}
		};

		ready
			.fake_input
			.authenticate("kwtypr".to_owned(), "KDE Virtual Keyboard Input".to_owned());

		Ok(Kwtypr {
			config: self.config,
			wayland: self.wayland,
			state: ready,
		})
	}
}

#[derive(Debug, Error)]
pub enum SendTextError {
	#[error("Wayland I/O failed")]
	WaylandIo(#[from] WaylandError),
	#[error("failed to dispatch Wayland events")]
	WaylandDispatch(#[from] DispatchError),
}

impl Kwtypr<Ready> {
	pub fn send_text(&mut self, text: &str) -> Result<(), SendTextError> {
		let mut typer = Typer::new(&self.wayland.connection, &self.state, &self.config);
		typer.type_text(text)?;

		self.wayland
			.event_queue
			.roundtrip(&mut self.wayland.bindings)?;
		Ok(())
	}
}

#[derive(Debug, Error)]
enum IntoReadyError {
	#[error("not all required Wayland objects were initialized")]
	UninitializedFields,
	#[error(transparent)]
	XkbInit(#[from] XkbInitError),
	#[error(transparent)]
	UnicodeFallbackInit(#[from] UnicodeFallbackInitError),
}

impl Bindings {
	fn into_ready(self, config: &KwtyprConfig) -> Result<Ready, IntoReadyError> {
		let fake_input = self.fake_input.ok_or(IntoReadyError::UninitializedFields)?;
		let keymap_fd = self.keymap_fd.ok_or(IntoReadyError::UninitializedFields)?;
		let xkb = Xkb::from_wayland_keymap(keymap_fd.fd, keymap_fd.size)?;
		let unicode_fallback = if config.unicode_fallback {
			UnicodeFallback::Enabled(xkb.unicode_fallback_keys()?)
		} else {
			UnicodeFallback::Disabled
		};
		Ok(Ready {
			fake_input,
			xkb,
			unicode_fallback,
		})
	}
}
