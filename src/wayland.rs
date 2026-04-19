use std::{
	io::{Error, ErrorKind},
	os::fd::{AsFd, OwnedFd},
	time::{Duration, Instant},
};

use rustix::{
	event::{PollFd, PollFlags, Timespec, poll},
	io::Errno,
};
use thiserror::Error;
use wayland_client::{
	ConnectError, Connection, Dispatch, DispatchError, Proxy, QueueHandle,
	backend::{ReadEventsGuard, WaylandError},
	delegate_noop,
	protocol::{
		wl_keyboard::{self, KeymapFormat, WlKeyboard},
		wl_registry,
		wl_seat::{self, WlSeat},
	},
};
use wayland_protocols_plasma::fake_input::client::org_kde_kwin_fake_input::OrgKdeKwinFakeInput;

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
	pub const fn all_bound(&self) -> bool {
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
	#[error(
		"timed out while waiting for Wayland initialization to reach the ready state \
		- make sure the application desktop entry is installed and its Exec property points to the executable"
	)]
	TimeoutElapsed(#[source] TimeoutElapsed),
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

	pub fn flush_blocking(&self) -> Result<(), WaylandError> {
		loop {
			match self.connection.flush() {
				Ok(()) => return Ok(()),
				Err(WaylandError::Io(error)) if error.kind() == ErrorKind::WouldBlock => {
					// `EAGAIN` maps to `WouldBlock`, so wait until the connection is writable and try again.
					wait_until_connection_writable(&self.connection)?;
				}
				Err(error) => return Err(error),
			}
		}
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

fn wait_until_connection_writable(connection: &Connection) -> Result<(), WaylandError> {
	let fd = connection.as_fd();
	let mut poll_fds = [PollFd::new(&fd, PollFlags::OUT | PollFlags::ERR)];

	loop {
		match poll(&mut poll_fds, None) {
			Ok(_) => return Ok(()),
			Err(Errno::INTR) => continue,
			Err(errno) => return Err(WaylandError::from(Error::from(errno))),
		}
	}
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

impl Dispatch<wl_registry::WlRegistry, ()> for Bindings {
	fn event(
		bindings: &mut Self,
		registry: &wl_registry::WlRegistry,
		event: wl_registry::Event,
		(): &(),
		_: &Connection,
		qh: &QueueHandle<Self>,
	) {
		let wl_registry::Event::Global {
			name,
			interface,
			version,
		} = event
		else {
			return;
		};

		match interface {
			_ if interface == WlSeat::interface().name => {
				bind_seat_proxy(bindings, registry, qh, name, version);
			}
			_ if interface == OrgKdeKwinFakeInput::interface().name => {
				bind_fake_input_proxy(bindings, registry, qh, name, version);
			}
			_ => (),
		}
	}
}

const SUPPORTED_SEAT_VERSION: u32 = 10;

fn bind_seat_proxy(
	bindings: &mut Bindings,
	registry: &wl_registry::WlRegistry,
	qh: &QueueHandle<Bindings>,
	name: u32,
	version: u32,
) {
	let version = version.min(SUPPORTED_SEAT_VERSION);
	let proxy: WlSeat = registry.bind(name, version, qh, ());
	bindings.seat = Some(proxy);
}

const SUPPORTED_FAKE_INPUT_VERSION: u32 = 6;

fn bind_fake_input_proxy(
	bindings: &mut Bindings,
	registry: &wl_registry::WlRegistry,
	qh: &QueueHandle<Bindings>,
	name: u32,
	version: u32,
) {
	let version = version.min(SUPPORTED_FAKE_INPUT_VERSION);
	let proxy: OrgKdeKwinFakeInput = registry.bind(name, version, qh, ());
	bindings.fake_input = Some(proxy);
}

delegate_noop!(Bindings: OrgKdeKwinFakeInput);

impl Dispatch<WlSeat, ()> for Bindings {
	fn event(
		bindings: &mut Self,
		seat: &WlSeat,
		event: wl_seat::Event,
		_user_data: &(),
		_connection: &Connection,
		qh: &QueueHandle<Self>,
	) {
		if let wl_seat::Event::Capabilities { capabilities } = event
			&& let Ok(capabilities) = capabilities.into_result()
			&& capabilities.contains(wl_seat::Capability::Keyboard)
		{
			bindings.keyboard = Some(seat.get_keyboard(qh, ()));
		}
	}
}

impl Dispatch<WlKeyboard, ()> for Bindings {
	fn event(
		bindings: &mut Self,
		_: &WlKeyboard,
		event: wl_keyboard::Event,
		(): &(),
		_: &Connection,
		_: &QueueHandle<Self>,
	) {
		if let wl_keyboard::Event::Keymap { format, fd, size } = event
			&& let Ok(KeymapFormat::XkbV1) = format.into_result()
		{
			bindings.keymap_fd = Some(KeymapFd { fd, size });
		}
	}
}
