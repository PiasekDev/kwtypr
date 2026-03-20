use wayland_client::{Connection, Dispatch, EventQueue, QueueHandle, protocol::wl_registry};
use wayland_protocols_plasma::fake_input::client::__interfaces::ORG_KDE_KWIN_FAKE_INPUT_INTERFACE;

struct AppState;

impl Dispatch<wl_registry::WlRegistry, ()> for AppState {
	fn event(
		_state: &mut Self,
		_: &wl_registry::WlRegistry,
		event: wl_registry::Event,
		_: &(),
		_: &Connection,
		_: &QueueHandle<AppState>,
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
			}
		}
	}
}

fn main() {
	let connection = Connection::connect_to_env().unwrap();
	let display = connection.display();

	let mut event_queue: EventQueue<AppState> = connection.new_event_queue();
	let queue_handle = event_queue.handle();

	let registry = display.get_registry(&queue_handle, ());

	println!("Advertised globals:");
	let _ = event_queue.roundtrip(&mut AppState);
}
