use clap::Parser;

use techmino_replay_toolkit::cli::{
    clap::{CliCommand, CliParser},
    operations::handle_cli_op,
};
#[cfg(feature = "tui")]
use techmino_replay_toolkit::tui::start as start_tui;

fn main() {
    let cli_cmd = CliParser::parse();

    match cli_cmd.command {
        CliCommand::Cli { operation } => {
            let res = handle_cli_op(&operation);
            if let Err(e) = res {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        #[cfg(feature = "tui")]
        CliCommand::Tui { arguments } => start_tui(arguments),
    };
}
