use std::{
	error::Error,
	io,
	num::{NonZeroU32, NonZeroU64},
	process::ExitCode,
	time::Duration,
};

use clap::{Args, CommandFactory, Parser, Subcommand};
use clap_complete::{Shell, generate};
use kwtypr::{ChunkPacing, InitializeError, Kwtypr, KwtyprConfig, SendTextError, TypingOutcome};
use thiserror::Error;

/// KWtype, but blazingly fast™
///
/// Type text using the KDE fake input interface on Wayland.
/// Uses the current keyboard layout to emit key events through the KDE fake input protocol.
#[derive(Parser)]
#[command(
	version,
	args_conflicts_with_subcommands = true,
	subcommand_negates_reqs = true
)]
struct Cli {
	#[command(flatten)]
	config: ConfigArgs,
	#[command(subcommand)]
	command: Option<Command>,
	/// Text to type
	#[arg(required = true, value_name = "TEXT")]
	text: Vec<String>,
}

#[derive(Subcommand)]
enum Command {
	/// Generate shell completion scripts
	#[command(alias = "completion")]
	Completions {
		/// Shell to generate completions for
		shell: Shell,
	},
}

#[derive(Args)]
struct ConfigArgs {
	/// Delay N milliseconds before typing begins to improve application compatibility
	#[arg(long = "initial-delay", default_value_t = 0, value_name = "MS")]
	initial_delay_ms: u64,
	/// Wait N milliseconds after each typed character (equivalent to --chunk-size 1 --chunk-delay N)
	#[arg(
		short = 'd',
		long = "character-delay",
		alias = "key-delay",
		value_name = "MS",
		conflicts_with_all = ["chunk_size", "chunk_delay_ms"]
	)]
	character_delay_ms: Option<NonZeroU64>,
	/// Type text in chunks of N input characters
	#[arg(
		long = "chunk-size",
		value_name = "N",
		requires = "chunk_delay_ms",
		conflicts_with = "character_delay_ms"
	)]
	chunk_size: Option<NonZeroU32>,
	/// Wait N milliseconds after each typed chunk
	#[arg(
		long = "chunk-delay",
		value_name = "MS",
		requires = "chunk_size",
		conflicts_with = "character_delay_ms"
	)]
	chunk_delay_ms: Option<NonZeroU64>,
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
	let Cli {
		config,
		command,
		text,
	} = Cli::parse();
	if let Some(Command::Completions { shell }) = command {
		generate_completions(shell);
		return ExitCode::SUCCESS;
	}

	let config = KwtyprConfig::from(config);
	let text = text.join(" ");
	match run(&text, config) {
		Ok(TypingOutcome::Complete) => ExitCode::SUCCESS,
		Ok(TypingOutcome::Partial { failed_characters }) => {
			let suffix = if failed_characters == 1 { "" } else { "s" };
			eprintln!(
				"kwtypr: {failed_characters} character{suffix} could not be typed with the current layout"
			);
			ExitCode::from(2)
		}
		Err(error) => handle_error(&error),
	}
}

fn generate_completions(shell: Shell) {
	let mut command = Cli::command();
	generate(
		shell,
		&mut command,
		env!("CARGO_PKG_NAME"),
		&mut io::stdout(),
	);
}

fn run(text: &str, config: KwtyprConfig) -> Result<TypingOutcome, KwtyprError> {
	let kwtypr = Kwtypr::with_config(config)?;
	let mut kwtypr = kwtypr.initialize()?;
	Ok(kwtypr.send_text(text)?)
}

fn handle_error(error: &KwtyprError) -> ExitCode {
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
		let chunk_pacing = match (
			args.character_delay_ms,
			args.chunk_size,
			args.chunk_delay_ms,
		) {
			(Some(delay), None, None) => Some(ChunkPacing {
				size: NonZeroU32::new(1).expect("1 should always be non-zero"),
				delay: Duration::from_millis(delay.get()),
			}),
			(None, Some(size), Some(delay)) => Some(ChunkPacing {
				size,
				delay: Duration::from_millis(delay.get()),
			}),
			(None, None, None) => None,
			_ => unreachable!("chunk pacing arguments should be validated by clap"),
		};

		Self {
			initial_delay: Duration::from_millis(args.initial_delay_ms),
			key_hold: Duration::from_millis(args.key_hold_ms),
			chunk_pacing,
			unicode_fallback: args.unicode_fallback,
			ready_timeout: match args.ready_timeout_ms {
				0 => None,
				millis => Some(Duration::from_millis(millis)),
			},
		}
	}
}
