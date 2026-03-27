use wayland_client::{
	Connection, Dispatch, Proxy, QueueHandle,
	protocol::{wl_registry, wl_seat::WlSeat},
};
use wayland_protocols_plasma::fake_input::client::org_kde_kwin_fake_input::{
	self, OrgKdeKwinFakeInput,
};

use crate::Components;

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
				bind_seat_proxy(components, registry, qh, name, version)
			}
			_ if interface == OrgKdeKwinFakeInput::interface().name => {
				bind_fake_input_proxy(components, registry, qh, name, version)
			}
			_ => (),
		}
	}
}

const SUPPORTED_SEAT_VERSION: u32 = 10;

fn bind_seat_proxy(
	components: &mut Components,
	registry: &wl_registry::WlRegistry,
	qh: &QueueHandle<Components>,
	name: u32,
	version: u32,
) {
	let version = version.min(SUPPORTED_SEAT_VERSION);
	let proxy: WlSeat = registry.bind(name, version, qh, ());
	components.seat = Some(proxy);
	println!("Bound a seat!");
}

const SUPPORTED_FAKE_INPUT_VERSION: u32 = 6;

fn bind_fake_input_proxy(
	components: &mut Components,
	registry: &wl_registry::WlRegistry,
	qh: &QueueHandle<Components>,
	name: u32,
	version: u32,
) {
	let version = version.min(SUPPORTED_FAKE_INPUT_VERSION);
	let proxy: OrgKdeKwinFakeInput = registry.bind(name, version, qh, ());
	components.fake_input = Some(proxy);
	println!("Bound the fake input interface!");
}

impl Dispatch<OrgKdeKwinFakeInput, ()> for Components {
	fn event(
		_: &mut Self,
		_: &OrgKdeKwinFakeInput,
		_: org_kde_kwin_fake_input::Event,
		_: &(),
		_: &Connection,
		_: &QueueHandle<Self>,
	) {
		eprintln!("Received an unexpected event from the fake input interface!");
	}
}
