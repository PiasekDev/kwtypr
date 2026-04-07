use std::{error::Error, process::ExitCode, time::Duration};

use clap::{Args, Parser};
use kwtypr::{InitializeError, Kwtypr, KwtyprConfig, SendTextError};
use thiserror::Error;

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

#[derive(Debug, Error)]
enum KwtyprError {
	#[error("failed to connect to the Wayland compositor")]
	WaylandConnect(#[from] wayland_client::ConnectError),
	#[error(transparent)]
	Initialize(#[from] InitializeError),
	#[error(transparent)]
	SendText(#[from] SendTextError),
}

fn main() -> ExitCode {
	let Cli { config, text } = Cli::parse();
	let config = KwtyprConfig::from(config);
	let text = text.join(" ");
	match run(&text, config) {
		Ok(()) => ExitCode::SUCCESS,
		Err(error) => handle_error(error),
	}
}

fn run(text: &str, config: KwtyprConfig) -> Result<(), KwtyprError> {
	let kwtypr = Kwtypr::with_config(config)?;
	let mut kwtypr = kwtypr.initialize()?;
	kwtypr.send_text(text)?;
	Ok(())
}

fn handle_error(error: KwtyprError) -> ExitCode {
	eprintln!("kwtypr: {error}");

	let mut source = error.source();
	while let Some(cause) = source {
		eprintln!("caused by: {cause}");
		source = cause.source();
	}

	ExitCode::FAILURE
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
