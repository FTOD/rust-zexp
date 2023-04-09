use crate::tacle::TACLe;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::prelude::*;
use toml::Table;

/// One command, can be complete or incomplete,
/// incomplete meaning that there are still some "$var" not replaced, complete otherwise
#[derive(Debug, Clone)]
pub struct Task {
    pub name: String,
    pub cmd: String,
    pub args: Vec<String>,
}

impl Task {
    pub fn is_completed(&self) -> bool {
        !self.cmd.contains("$") && !self.args.iter().any(|arg| arg.contains("$"))
    }
}

/// The script has a main loader, the name of task
pub struct Script {
    script_config: toml::Table,
    loaders: Vec<Box<dyn ConfigLoaderTrait>>,
    main_loader: Option<Box<dyn MainLoaderTrait>>,
}

impl Script {
    /// add a loader
    pub fn register_loader<L: ConfigLoaderTrait + LoadableFromConfig + 'static>(&mut self) {
        let new_loader = Box::new(L::from(self.script_config.clone()));
        self.loaders.push(new_loader);
    }

    /// add main loader
    pub fn register_main_loader<ML: MainLoaderTrait + LoadableFromConfig + 'static>(&mut self) {
        self.main_loader = Some(Box::new(ML::from(self.script_config.clone())));
    }

    /// load the config from the script file
    /// do not check the validity of the config (because loaders are not loaded yet)
    pub fn from_file(path: &str) -> Self {
        let mut script_file = File::open(path).expect("Error when openning the script file");

        let mut script_config = String::new();
        script_file
            .read_to_string(&mut script_config)
            .expect("Error when reading script file");

        let script_config = script_config
            .parse::<Table>()
            .expect("Error when parsing the script file, check you TOML syntax!");

        Self {
            script_config: script_config,
            loaders: Vec::new(),
            main_loader: None,
        }
    }

    /// fill the command with all loaders, i.e. all static variables are replaced
    fn fill_static_vars(&self) -> Vec<String> {
        let mut static_vars = Vec::new();
        let cmd = self.script_config["CMD"]
            .as_str()
            .expect("CMD must be a string");
        let cmd = cmd.split_whitespace().collect::<Vec<&str>>();
        for term in &cmd {
            if term.starts_with("$") {
                let mut provided = false;
                for loader in &self.loaders {
                    if loader.provided_vars().contains(&term.to_string()) {
                        static_vars.extend(
                            loader
                                .get_terms(&term)
                                .expect("variable claimed to be provided but not ?"),
                        );
                        provided = true;
                    }
                }
                if !provided {
                    static_vars.push(term.to_string());
                }
            }
        }
        static_vars
    }

    pub fn gen_cmd(&mut self) -> Result<Vec<Task>, String> {
        let static_command = self.fill_static_vars();
        let full_command = self
            .main_loader
            .as_ref()
            .expect("you must register a main loader before using the script")
            .fill(&static_command)?;
        Ok(full_command)
    }
}

pub trait LoadableFromConfig {
    fn from(config: toml::Table) -> Self;
}

pub trait ConfigLoaderTrait {
    /// return the options that this loader will load
    fn provided_vars(&self) -> Vec<String>;

    /// return the possible terms to replace the given var_name
    fn get_terms(&self, var_name: &str) -> Result<Vec<String>, String>;
}

pub trait MainLoaderTrait {
    /// Fill the "static" command with the last variables related to the main loader
    /// return all commands to run, if the command not complete after filling, return an error
    fn fill(&self, cmd: &Vec<String>) -> Result<Vec<Task>, String>;
}

#[derive(Deserialize)]
struct OTAWAConfigLoader {
    PROVIDED_VARS: Vec<String>,
    app_path: String,
    props: Vec<String>,
    log_level: String,
}

impl LoadableFromConfig for OTAWAConfigLoader {
    fn from(config: toml::Table) -> Self {
        // get the corresponding sub-table
        let otawa_sub_table = config["OTAWA"]
            .as_table()
            .expect("the OTAWA subtable should be a table")
            .clone();
        // load the config with serde::Deserialize trait
        otawa_sub_table.try_into().unwrap()
    }
}
impl ConfigLoaderTrait for OTAWAConfigLoader {
    fn provided_vars(&self) -> Vec<String> {
        return self.PROVIDED_VARS.clone();
    }

    fn get_terms(&self, var_name: &str) -> Result<Vec<String>, String> {
        match var_name {
            "$otawa_app" => Ok(vec![self.app_path.clone()]),
            "$otawa_opts" => {
                let mut res = Vec::new();
                for prop in &self.props {
                    res.push("--add-prop".to_string());
                    res.push(prop.clone());
                }
                res.push("--log".to_string());
                res.push(self.log_level.clone());
                Ok(res)
            }
            _ => Err(format!("Unknown var_name: {}", var_name).to_string()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct TACLeConfigLoader {
    PROVIDED_VARS: Vec<String>,
    tacle_desc_path: String,
    tacle_run_benchset: Vec<String>,
}

impl LoadableFromConfig for TACLeConfigLoader {
    fn from(config: toml::Table) -> Self {
        // get the corresponding sub-table
        let tacle_sub_table = config["TACLE"]
            .as_table()
            .expect("the OTAWA subtable should be a table")
            .clone();
        // load the config with serde::Deserialize trait
        tacle_sub_table.try_into().unwrap()
    }
}

impl MainLoaderTrait for TACLeConfigLoader {
    fn fill(&self, cmd: &Vec<String>) -> Result<Vec<Task>, String> {
        let tacle = TACLe::from_script(&self.tacle_desc_path);
        let benchs = tacle.select_bench(&vec!["kernel".to_string()]);
        let mut res = Vec::new();
        for bench in &benchs {
            let mut cmd = cmd.clone();
            for term in cmd.iter_mut() {
                match term.as_str() {
                    "$tacle_exec" => *term = bench.exec.clone(),
                    "$tacle_entry_point" => *term = bench.entry_point.clone(),
                    _ => continue,
                }
            }

            let cmd = Task {
                name: bench.name.clone(),
                cmd: cmd[0].clone(),
                args: cmd[1..].to_vec(),
            };
            if !cmd.is_completed() {
                return Err(format!("Command not completed: {:?}", cmd.args).to_string());
            }
            res.push(cmd);
        }
        Ok(res)
    }
}

pub fn otawa_tacle_script(file_name: &str) -> Script {
    let mut script = Script::from_file(file_name);
    script.register_main_loader::<TACLeConfigLoader>();
    script.register_loader::<OTAWAConfigLoader>();
    script
}

#[cfg(test)]
mod test {
    use super::*;
    use log::debug;
    #[test]
    fn test_otawa_loader() {
        use simplelog::*;
        TermLogger::init(
            LevelFilter::Trace,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        )
        .unwrap();
        let mut script =
            otawa_tacle_script("/home/acac/rust-zexp/scripts/otawa-tacle-exp/example.toml");
        let cmds = script.gen_cmd().unwrap();
        debug!("{:?}", cmds)
    }
}
