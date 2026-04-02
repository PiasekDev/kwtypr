use wayland_client::{
	Connection, Dispatch, Proxy, QueueHandle, delegate_noop,
	protocol::{wl_registry, wl_seat::WlSeat},
};
use wayland_protocols_plasma::fake_input::client::org_kde_kwin_fake_input::OrgKdeKwinFakeInput;

use crate::wayland::BoundGlobals;

impl Dispatch<wl_registry::WlRegistry, ()> for BoundGlobals {
	fn event(
		globals: &mut Self,
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
				bind_seat_proxy(globals, registry, qh, name, version)
			}
			_ if interface == OrgKdeKwinFakeInput::interface().name => {
				bind_fake_input_proxy(globals, registry, qh, name, version)
			}
			_ => (),
		}
	}
}

const SUPPORTED_SEAT_VERSION: u32 = 10;

fn bind_seat_proxy(
	globals: &mut BoundGlobals,
	registry: &wl_registry::WlRegistry,
	qh: &QueueHandle<BoundGlobals>,
	name: u32,
	version: u32,
) {
	let version = version.min(SUPPORTED_SEAT_VERSION);
	let proxy: WlSeat = registry.bind(name, version, qh, ());
	globals.seat = Some(proxy);
	println!("Bound a seat!");
}

const SUPPORTED_FAKE_INPUT_VERSION: u32 = 6;

fn bind_fake_input_proxy(
	globals: &mut BoundGlobals,
	registry: &wl_registry::WlRegistry,
	qh: &QueueHandle<BoundGlobals>,
	name: u32,
	version: u32,
) {
	let version = version.min(SUPPORTED_FAKE_INPUT_VERSION);
	let proxy: OrgKdeKwinFakeInput = registry.bind(name, version, qh, ());
	globals.fake_input = Some(proxy);
	println!("Bound the fake input interface!");
}

delegate_noop!(BoundGlobals: OrgKdeKwinFakeInput);
