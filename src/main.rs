use clap::Parser;
use log::{debug, error, info, trace, warn};
use simplelog::{ColorChoice, Config, LevelFilter, TermLogger, TerminalMode};
use std::path::Path;
use std::process::{exit, Command};

mod script;
mod tacle;
use crate::script::script::Script;

/// Run experince with ZExp!
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The path to the script (in TOML format) to run
    #[arg(short, long)]
    script: String,

    /// Number of cores you want to use
    #[arg(short, default_value_t = 1)]
    j: u8,
}

fn main() {
    TermLogger::init(
        LevelFilter::Trace,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .unwrap();

    let args = Args::parse();
    let script_path = args.script;
    let cores_number = args.j;
    info!("script path: {:?}", script_path);
    info!("cores number: {:?}", cores_number);

    match Path::new(&script_path).try_exists() {
        Ok(true) => debug!("script file exists"),
        Ok(false) => {
            error!("script file does not exist, Aborting...");
            exit(-1);
        }
        Err(_) => {
            error!("Error when checking if script file exists");
            exit(-1);
        }
    }
    // debug!("{:?}", otawa_app_name);
    // for arg in otawa_props_args {
    //     debug!("{:?}", arg);
    // }

    // let output = Command::new(script_config["APP_PATH"].as_str())
    //     .arg("-l")
    //     .output()
    //     .expect("failed to execute process");
}
