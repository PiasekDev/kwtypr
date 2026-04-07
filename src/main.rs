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
		alias = "key-delay",
		default_value_t = 0,
		value_name = "MS"
	)]
	character_delay_ms: u64,
	/// Hold each key for N milliseconds before releasing it
	#[arg(short = 'H', long = "key-hold", default_value_t = 0, value_name = "MS")]
	key_hold_ms: u64,
	/// Fall back to Ctrl+Shift+U Unicode input when a character cannot be typed directly
	#[arg(long)]
	unicode_fallback: bool,
	/// Fail if initialization does not reach the ready state within N milliseconds (0 disables the timeout)
	#[arg(long = "ready-timeout", default_value_t = 5_000, value_name = "MS")]
	ready_timeout_ms: u64,
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
	kwtypr.send_text(text)?;
	Ok(())
}

impl From<ConfigArgs> for KwtyprConfig {
	fn from(args: ConfigArgs) -> Self {
		Self {
			character_delay: Duration::from_millis(args.character_delay_ms),
			key_hold: Duration::from_millis(args.key_hold_ms),
			unicode_fallback: args.unicode_fallback,
			ready_timeout: match args.ready_timeout_ms {
				0 => None,
				millis => Some(Duration::from_millis(millis)),
			},
		}
	}
}
