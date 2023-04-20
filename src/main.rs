use clap::Parser;
use log::{error, info};
use simplelog::{ColorChoice, Config, LevelFilter, TermLogger, TerminalMode};
use std::fs;
use std::path::Path;
use std::process::{exit, Command, Output};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use wait_timeout::ChildExt;

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
                    let mut fout = task.name.clone();
                    fout.push_str(".out");
                    let fout = fs::File::create(fout).unwrap();
                    let mut child = Command::new(&task.cmd)
                        .args(&task.args)
                        .stderr(fout.try_clone().unwrap())
                        .stdout(fout)
                        .spawn()
                        .expect("Failed to execute command");
                    info!("Task {} terminated", task.name);

                    // timeout for 2 hours
                    let two_hours = Duration::from_secs(3600);
                    match child.wait_timeout(two_hours).unwrap() {
                        Some(status) => {
                            return;
                        }
                        None => {
                            info!("Task {} timed out, killed", task.name);
                            // timeout, kill it
                            child.kill().unwrap();
                            child.wait().unwrap().code()
                        }
                    };
                })
            }
        })
    }
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
