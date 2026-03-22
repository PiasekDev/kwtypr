use kwtypr::{Kwtypr, WaylandConnectError};

fn main() -> Result<(), WaylandConnectError> {
	let mut kwtypr = Kwtypr::new()?;
	kwtypr.initialize();
	kwtypr.send_text("Hello, Wayland!");
	Ok(())
}
