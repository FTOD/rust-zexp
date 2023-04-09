use clap::Parser;
use log::{error, info};
use simplelog::{ColorChoice, Config, LevelFilter, TermLogger, TerminalMode};
use std::fs;
use std::path::Path;
use std::process::{exit, Command, Output};
use std::sync::{Arc, Mutex};

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

fn run_tasks_concurrently(tasks: &Vec<Task>, num_cores: usize) {
    // Create a thread pool with the specified number of cores
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_cores)
        .build()
        .unwrap();

    let tasks = Arc::new(Mutex::new(tasks.clone()));
    for _ in 0..num_cores {
        let tasks = Arc::clone(&tasks);
        pool.scope(|s| {
            while let Some(task) = {
                let mut tasks_guard = tasks.lock().unwrap();
                tasks_guard.pop()
            } {
                s.spawn(move |_| {
                    info!("Running task: {}", &task.name);
                    let output = Command::new(&task.cmd)
                        .args(&task.args)
                        .output()
                        .expect("Failed to execute command");
                    log_into_file(&task.name, &output);
                    info!("Task {} terminated", task.name);
                })
            }
        })
    }
}

fn log_into_file(name: &str, output: &Output) {
    let mut to_log = output.stderr.clone();
    to_log.extend(&output.stdout);
    // open a file with name, write the output in it
    let mut out_filename = name.to_string();
    out_filename.push_str(".out");
    fs::write(out_filename, to_log).expect("cannot write to file");
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
    run_tasks_concurrently(&cmd, num_cores);
}
