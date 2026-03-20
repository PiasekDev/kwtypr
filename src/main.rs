use std::cell::OnceCell;

use wayland_client::{
	Connection, Dispatch, EventQueue, QueueHandle,
	protocol::{wl_keyboard::KeyState, wl_registry},
};
use wayland_protocols_plasma::fake_input::client::{
	__interfaces::ORG_KDE_KWIN_FAKE_INPUT_INTERFACE, org_kde_kwin_fake_input::OrgKdeKwinFakeInput,
};

struct AppState(OnceCell<OrgKdeKwinFakeInput>);

impl Dispatch<wl_registry::WlRegistry, ()> for AppState {
	fn event(
		state: &mut Self,
		registry: &wl_registry::WlRegistry,
		event: wl_registry::Event,
		_: &(),
		_: &Connection,
		qh: &QueueHandle<AppState>,
	) {
		// When receiving events from the wl_registry, we are only interested in the
		// `global` event, which signals a new available global.
		// When receiving this event, we just print its characteristics in this example.
		if let wl_registry::Event::Global {
			name,
			interface,
			version,
		} = event
		{
			println!("[{}] {} (v{})", name, interface, version);
			if interface == ORG_KDE_KWIN_FAKE_INPUT_INTERFACE.name {
				println!("Found the fake input interface!");
				let proxy: OrgKdeKwinFakeInput = registry.bind(name, version, qh, ());
				state.0.set(proxy).unwrap();
			}
		}
	}
}

impl Dispatch<OrgKdeKwinFakeInput, ()> for AppState {
	fn event(
		_: &mut Self,
		_: &OrgKdeKwinFakeInput,
		_: <OrgKdeKwinFakeInput as wayland_client::Proxy>::Event,
		_: &(),
		_: &Connection,
		_: &QueueHandle<Self>,
	) {
		unreachable!()
	}
}

fn main() {
	let connection = Connection::connect_to_env().unwrap();
	let display = connection.display();

	let mut event_queue: EventQueue<AppState> = connection.new_event_queue();
	let queue_handle = event_queue.handle();

	let registry = display.get_registry(&queue_handle, ());

	println!("Advertised globals:");
	let mut appstate = AppState(OnceCell::new());
	let _ = event_queue.roundtrip(&mut appstate);

	let fake_input = appstate.0.get().unwrap();
	fake_input.authenticate("kwtypr".to_owned(), "KDE Virtual Keyboard Input".to_owned());

	const KEY_A: u32 = 30;
	fake_input.keyboard_key(KEY_A, KeyState::Pressed.into());
	fake_input.keyboard_key(KEY_A, KeyState::Released.into());

	event_queue.roundtrip(&mut appstate).unwrap();
}
