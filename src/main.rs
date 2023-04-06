use clap::Parser;
use log::{debug, error, info, trace, warn};
use rayon::prelude::*;
use simplelog::{ColorChoice, Config, LevelFilter, TermLogger, TerminalMode};
use std::fs;
use std::path::Path;
use std::process::{exit, Command, Output};
use std::sync::{Arc, Mutex};

mod runner;
mod script;
mod tacle;

use crate::script::*;

/// Run experince with ZExp!
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The path to the script (in TOML format) to run
    #[arg(short, long)]
    script: String,

    /// Number of cores you want to use
    #[arg(short, default_value_t = 1)]
    j: usize,
}

fn run_tasks_concurrently(tasks: Vec<(String, Vec<String>)>, num_cores: usize) {
    // Create a thread pool with the specified number of cores
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_cores)
        .build()
        .unwrap();

    let tasks = Arc::new(Mutex::new(tasks));
    let mut handles = Vec::new();

    for _ in 0..num_cores {
        let tasks = Arc::clone(&tasks);
        let handle = pool.install(move || {
            while let Some((cmd, opts)) = {
                let mut tasks_guard = tasks.lock().unwrap();
                tasks_guard.pop()
            } {
                // TODO, currently the name is derived from the options, which works only for tacle
                let name = Path::new(&opts[0]);
                let name = name.file_name().unwrap().to_str().unwrap();
                info!("Running task: {}", &name);
                debug!("Command: {} {:?}", &cmd, &opts);
                let output = Command::new(&cmd)
                    .args(&opts)
                    .output()
                    .expect("Failed to execute command");
                log_into_file(&name, &output);
                info!("Task {} terminated", name);
            }
        });
        handles.push(handle);
    }
}

fn log_into_file(name: &str, output: &Output) {
    // open a file with name, write the output in it
    let mut script_file = fs::write(name, &output.stdout).expect("cannot write to file");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
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
    let num_cores = args.j;
    info!("script path: {:?}", script_path);
    info!("cores number: {:?}", num_cores);

    match Path::new(&script_path).try_exists() {
        Ok(true) => {}
        Ok(false) => {
            error!("script file does not exist, Aborting...");
            exit(-1);
        }
        Err(_) => {
            error!("Error when checking if script file exists");
            exit(-1);
        }
    }

    let mut script = otawa_tacle_script(&script_path);
    let cmd = script.gen_cmd().unwrap();
    run_tasks_concurrently(cmd, num_cores)
}
