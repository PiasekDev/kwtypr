use input_linux::Key;
use thiserror::Error;
use wayland_client::{
	ConnectError, Connection, Dispatch, Proxy, QueueHandle,
	protocol::{
		wl_keyboard::{self, KeyState, WlKeyboard},
		wl_registry,
		wl_seat::{self, WlSeat},
	},
};
use wayland_protocols_plasma::fake_input::client::org_kde_kwin_fake_input::OrgKdeKwinFakeInput;

use crate::xkb::{
	Xkb,
	mapping::{MappedKey, Modifiers},
};

mod xkb;

pub struct Kwtypr {
	wayland: WaylandSession<Components>,
	components: Components,
}

#[derive(Debug, Error)]
#[error("failed to connect to the Wayland compositor")]
pub struct WaylandConnectError(#[from] ConnectError);

impl Kwtypr {
	pub fn new() -> Result<Self, WaylandConnectError> {
		Ok(Self {
			wayland: WaylandSession::new()?,
			components: Components::default(),
		})
	}

	pub fn initialize(&mut self) {
		let queue_handle = self.wayland.event_queue.handle();
		let display = self.wayland.connection.display();
		let _registry = display.get_registry(&queue_handle, ());
		println!("Advertised globals:");
		let _ = self.wayland.event_queue.roundtrip(&mut self.components);

		while !self.all_components_available() {
			let _ = self
				.wayland
				.event_queue
				.blocking_dispatch(&mut self.components);
		}

		let Some(fake_input) = &self.components.fake_input else {
			eprintln!("Fake input interface is not available, cannot authenticate");
			std::process::exit(1);
		};

		fake_input.authenticate("kwtypr".to_owned(), "KDE Virtual Keyboard Input".to_owned());
	}

	fn all_components_available(&self) -> bool {
		self.components.fake_input.is_some()
			&& self.components.seat.is_some()
			&& self.components.keyboard.is_some()
			&& self.components.xkb.is_some()
	}

	pub fn send_text(&mut self, text: &str) {
		let Some(fake_input) = &self.components.fake_input else {
			eprintln!("Cannot send input events because the fake input interface is not available");
			return;
		};

		for character in text.chars() {
			if let Some(xkb_state) = &self.components.xkb {
				match xkb_state.key_for_char(character) {
					Ok(mapped_key) => send_mapped_key(fake_input, &mapped_key),
					Err(e) => eprintln!(
						"Failed to map character {:?} to a key event: {}",
						character, e
					),
				}
			} else {
				eprintln!(
					"Cannot map character {:?} to a key event because XKB state is not available",
					character
				);
			}
		}

		self.wayland
			.event_queue
			.roundtrip(&mut self.components)
			.unwrap();
	}
}

fn send_mapped_key(fake_input: &OrgKdeKwinFakeInput, mapped_key: &MappedKey) {
	press_modifiers(fake_input, &mapped_key.modifiers);
	fake_input.keyboard_key(mapped_key.raw_keycode.into(), KeyState::Pressed.into());
	fake_input.keyboard_key(mapped_key.raw_keycode.into(), KeyState::Released.into());
	release_modifiers(fake_input, &mapped_key.modifiers);
}

fn press_modifiers(fake_input: &OrgKdeKwinFakeInput, modifiers: &Modifiers) {
	if modifiers.shift {
		fake_input.keyboard_key(Key::LeftShift.code().into(), KeyState::Pressed.into());
	}

	if modifiers.altgr {
		fake_input.keyboard_key(Key::RightAlt.code().into(), KeyState::Pressed.into());
	}
}

fn release_modifiers(fake_input: &OrgKdeKwinFakeInput, modifiers: &Modifiers) {
	if modifiers.shift {
		fake_input.keyboard_key(Key::LeftShift.code().into(), KeyState::Released.into());
	}

	if modifiers.altgr {
		fake_input.keyboard_key(Key::RightAlt.code().into(), KeyState::Released.into());
	}
}

struct WaylandSession<State> {
	connection: wayland_client::Connection,
	event_queue: wayland_client::EventQueue<State>,
}

impl WaylandSession<Components> {
	fn new() -> Result<Self, ConnectError> {
		let connection = wayland_client::Connection::connect_to_env()?;
		let event_queue = connection.new_event_queue();
		Ok(Self {
			connection,
			event_queue,
		})
	}
}

#[derive(Default)]
struct Components {
	fake_input: Option<OrgKdeKwinFakeInput>,
	seat: Option<WlSeat>,
	keyboard: Option<WlKeyboard>,
	xkb: Option<Xkb>,
}

const SUPPORTED_SEAT_VERSION: u32 = 10;
const SUPPORTED_FAKE_INPUT_VERSION: u32 = 6;

impl Dispatch<wl_registry::WlRegistry, ()> for Components {
	fn event(
		components: &mut Self,
		registry: &wl_registry::WlRegistry,
		event: wl_registry::Event,
		_: &(),
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
				let version = version.min(SUPPORTED_SEAT_VERSION);
				let proxy: WlSeat = registry.bind(name, version, qh, ());
				components.seat = Some(proxy);
				println!("Bound a seat!");
			}
			_ if interface == OrgKdeKwinFakeInput::interface().name => {
				let version = version.min(SUPPORTED_FAKE_INPUT_VERSION);
				let proxy: OrgKdeKwinFakeInput = registry.bind(name, version, qh, ());
				components.fake_input = Some(proxy);
				println!("Bound the fake input interface!");
			}
			_ => (),
		}
	}
}

impl Dispatch<WlSeat, ()> for Components {
	fn event(
		components: &mut Self,
		seat: &WlSeat,
		event: wl_seat::Event,
		_user_data: &(),
		_connection: &Connection,
		qh: &QueueHandle<Components>,
	) {
		let wl_seat::Event::Capabilities { capabilities } = event else {
			return;
		};

		let Ok(capabilities) = capabilities.into_result() else {
			return;
		};

		if capabilities.contains(wl_seat::Capability::Keyboard) {
			components.keyboard.get_or_insert_with(|| {
				let proxy: WlKeyboard = seat.get_keyboard(qh, ());
				println!("Got a keyboard! {:?}", proxy);
				proxy
			});
		} else {
			components.keyboard = None;
		}
	}
}

impl Dispatch<WlKeyboard, ()> for Components {
	fn event(
		components: &mut Self,
		_: &WlKeyboard,
		event: wl_keyboard::Event,
		_: &(),
		_: &Connection,
		_: &QueueHandle<Self>,
	) {
		if let wl_keyboard::Event::Keymap { format, fd, size } = event
			&& let Ok(format) = format.into_result()
			&& format == wl_keyboard::KeymapFormat::XkbV1
		{
			let Ok(xkb_state) = Xkb::from_wayland_keymap(fd, size) else {
				eprintln!("Failed to initialize XKB state from the provided keymap");
				std::process::exit(1);
			};

			components.xkb = Some(xkb_state);
		}
	}
}

impl Dispatch<OrgKdeKwinFakeInput, ()> for Components {
	fn event(
		_: &mut Self,
		_: &OrgKdeKwinFakeInput,
		_: <OrgKdeKwinFakeInput as wayland_client::Proxy>::Event,
		_: &(),
		_: &Connection,
		_: &QueueHandle<Self>,
	) {
		eprintln!("Received an unexpected event from the fake input interface!");
	}
}
