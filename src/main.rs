use clap::Parser;
use log::{debug, error, info, trace, warn};
use pretty_env_logger;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::process::{exit, Command};
use toml::Table;

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
    pretty_env_logger::init();
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

    let mut script_file = match File::open(script_path) {
        Ok(file) => {
            debug!("script file opened");
            file
        }
        Err(_) => {
            error!("Error when opening script file");
            exit(-1);
        }
    };

    let mut script_config = String::new();
    match script_file.read_to_string(&mut script_config) {
        Err(why) => {
            error!("Error when reading script file: {}", why);
            exit(-1);
        }
        Ok(_) => debug!("script file read"),
    }

    let script_config = match script_config.parse::<Table>() {
        Ok(script_config) => {
            debug!("script file parsed");
            script_config
        }
        Err(why) => {
            error!("Error when parsing script file: {}", why);
            exit(-1);
        }
    };

    for (key, value) in &script_config {
        info!("key: {:?}, value: {:?}", key, value);
    }

    let otawa_app_name = match script_config["APP_PATH"].as_str() {
        Some(app_name) => app_name,
        None => {
            error!("APP_PATH not found in script file, Aborting...");
            exit(-1);
        }
    };

    let otawa_props = match script_config["OTAWA_PROPS"].as_array() {
        Some(otawa_props) => otawa_props,
        None => {
            error!("OTAWA_PROPS not found or not an array liked field in script file, Aborting...");
            exit(-1);
        }
    };

    let mut otawa_props_args: Vec<&str> = Vec::new();
    for prop in otawa_props {
        otawa_props_args.push("--add-prop");
        let prop = match prop.as_str() {
            Some(prop) => prop,
            None => {
                error!("each element of OTAWA_PROPS must be a string, which is not the case, Aborting...");
                exit(-1);
            }
        };
        otawa_props_args.push(prop);
    }

    debug!("{:?}", otawa_app_name);
    for arg in otawa_props_args {
        debug!("{:?}", arg);
    }

    // let output = Command::new(script_config["APP_PATH"].as_str())
    //     .arg("-l")
    //     .output()
    //     .expect("failed to execute process");
}
