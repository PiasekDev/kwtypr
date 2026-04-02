use kwtypr::{Kwtypr, WaylandConnectError};

fn main() -> Result<(), WaylandConnectError> {
	let kwtypr = Kwtypr::new()?;
	let mut kwtypr = kwtypr.initialize();
	kwtypr.send_text("Zażółć gęślą jaźń");
	Ok(())
}
