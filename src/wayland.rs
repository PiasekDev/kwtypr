use thiserror::Error;
use wayland_client::ConnectError;

use crate::Components;

mod keyboard;
mod registry;

#[derive(Debug, Error)]
#[error("failed to connect to the Wayland compositor")]
pub struct WaylandConnectError(#[from] ConnectError);

pub struct WaylandSession<State> {
	pub connection: wayland_client::Connection,
	pub event_queue: wayland_client::EventQueue<State>,
}

impl WaylandSession<Components> {
	pub fn new() -> Result<Self, WaylandConnectError> {
		let connection = wayland_client::Connection::connect_to_env()?;
		let event_queue = connection.new_event_queue();
		Ok(Self {
			connection,
			event_queue,
		})
	}
}
