use std::{
	io::Error,
	os::fd::OwnedFd,
	time::{Duration, Instant},
};

use rustix::{
	event::{PollFd, PollFlags, Timespec, poll},
	io::Errno,
};
use thiserror::Error;
use wayland_client::{
	ConnectError, DispatchError,
	backend::{ReadEventsGuard, WaylandError},
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

#[derive(Debug, Error)]
pub enum WaylandInitializeError {
	#[error(transparent)]
	WaylandDispatch(#[from] DispatchError),
	#[error(transparent)]
	WaylandIo(#[from] WaylandError),
	#[error(transparent)]
	TimeoutElapsed(#[from] TimeoutElapsed),
}

impl WaylandSession {
	pub fn wait_until_ready(
		&mut self,
		timeout: Option<Duration>,
	) -> Result<(), WaylandInitializeError> {
		let timeout = timeout.map(Timeout::new);

		while !self.bindings.all_bound() {
			let dispatched_amount = self.dispatch_pending()?;
			if dispatched_amount > 0 {
				// retry, as we might have received the required Wayland objects in the dispatched events
				continue;
			}

			self.connection.flush()?;

			if let Some(read_guard) = self.connection.prepare_read() {
				blocking_read_with_timeout(read_guard, timeout.as_ref())?;
			}

			// dispatch the events we just read, like in wayland-client's blocking_dispatch()
			self.dispatch_pending()?;
		}

		Ok(())
	}

	fn dispatch_pending(&mut self) -> Result<usize, DispatchError> {
		self.event_queue.dispatch_pending(&mut self.bindings)
	}
}

#[derive(Debug, Error)]
enum BlockingReadWithTimeoutError {
	#[error(transparent)]
	WaylandIo(#[from] WaylandError),
	#[error(transparent)]
	TimeoutElapsed(#[from] TimeoutElapsed),
}

impl From<BlockingReadWithTimeoutError> for WaylandInitializeError {
	fn from(value: BlockingReadWithTimeoutError) -> Self {
		match value {
			BlockingReadWithTimeoutError::WaylandIo(error) => Self::WaylandIo(error),
			BlockingReadWithTimeoutError::TimeoutElapsed(error) => Self::TimeoutElapsed(error),
		}
	}
}

fn blocking_read_with_timeout(
	read_guard: ReadEventsGuard,
	timeout: Option<&Timeout>,
) -> Result<(), BlockingReadWithTimeoutError> {
	let fd = read_guard.connection_fd();
	let mut poll_fds = [PollFd::new(&fd, PollFlags::IN | PollFlags::ERR)];

	loop {
		let poll_timeout = timeout.map(Timeout::poll_timeout).transpose()?;
		match poll(&mut poll_fds, poll_timeout.as_ref()) {
			Ok(0) => {
				let duration = timeout
					.expect("poll returning 0 should only be possible if we provided a timeout")
					.duration;
				return Err(TimeoutElapsed(duration).into());
			}
			Ok(_) => break,
			Err(Errno::INTR) => continue,
			Err(errno) => return Err(WaylandError::from(Error::from(errno)).into()),
		}
	}

	let _dispatched_amount = read_guard.read()?;
	Ok(())
}

struct Timeout {
	duration: Duration,
	started_at: Instant,
}

#[derive(Debug, Error)]
#[error("timed out after {0:?}")]
pub struct TimeoutElapsed(Duration);

impl Timeout {
	fn new(duration: Duration) -> Self {
		Self {
			duration,
			started_at: Instant::now(),
		}
	}

	fn poll_timeout(&self) -> Result<Timespec, TimeoutElapsed> {
		let remaining = self.duration.saturating_sub(self.started_at.elapsed());
		if remaining.is_zero() {
			return Err(TimeoutElapsed(self.duration));
		}

		Ok(Timespec::try_from(remaining).expect("ready timeout should fit into a poll timespec"))
	}
}
