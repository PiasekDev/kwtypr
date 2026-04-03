use std::time::Duration;

use clap::{Args, Parser};
use kwtypr::{Kwtypr, KwtyprConfig, KwtyprError};

/// KWtype, but blazingly fast™
///
/// Type text using the KDE fake input interface on Wayland.
/// Uses the current keyboard layout to emit key events through the KDE fake input protocol.
#[derive(Parser)]
#[command(version)]
struct Cli {
	#[command(flatten)]
	config: ConfigArgs,
	/// Text to type
	#[arg(required = true, value_name = "TEXT")]
	text: Vec<String>,
}

#[derive(Args)]
struct ConfigArgs {
	/// Delay N milliseconds between typing each character
	#[arg(
		short = 'd',
		long = "character-delay",
		default_value_t = 0,
		value_name = "MS"
	)]
	character_delay_ms: u64,
	/// Hold each character for N milliseconds before releasing it
	#[arg(
		short = 'H',
		long = "character-hold",
		default_value_t = 0,
		value_name = "MS"
	)]
	character_hold_ms: u64,
}

fn main() -> Result<(), KwtyprError> {
	let Cli { config, text } = Cli::parse();
	let config = KwtyprConfig::from(config);
	let text = text.join(" ");
	run(&text, config)
}

fn run(text: &str, config: KwtyprConfig) -> Result<(), KwtyprError> {
	let kwtypr = Kwtypr::with_config(config)?;
	let mut kwtypr = kwtypr.initialize()?;
	kwtypr.send_text(text);
	Ok(())
}

impl From<ConfigArgs> for KwtyprConfig {
	fn from(args: ConfigArgs) -> Self {
		Self {
			character_delay: Duration::from_millis(args.character_delay_ms),
			character_hold: Duration::from_millis(args.character_hold_ms),
		}
	}
}
