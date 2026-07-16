use clap::Parser;

use crate::cli::{
    clap::{CliCommand, CliParser},
    operations::handle_cli_op,
};

mod cli;
#[cfg(feature = "gui")]
mod gui;
#[cfg(feature = "tui")]
mod tui;

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
    };
}
