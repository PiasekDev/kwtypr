use kwtypr::{Kwtypr, KwtyprError};

fn main() -> Result<(), KwtyprError> {
	let kwtypr = Kwtypr::new()?;
	let mut kwtypr = kwtypr.initialize()?;
	kwtypr.send_text("Zażółć gęślą jaźń");
	Ok(())
}
