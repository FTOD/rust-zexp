pub mod script {

    use crate::tacle::tacle::TACLe;
    use log::{debug, error, info, trace, warn};
    use simplelog::{ColorChoice, Config, LevelFilter, TermLogger, TerminalMode};
    use std::collections::HashSet;
    use std::fs::File;
    use std::io::prelude::*;
    use std::ops::Mul;
    use std::path::Path;
    use std::process::{exit, Command};
    use toml::Table;

    /// An alternative argv set represents a set of possible strings that can be places as one argv term
    type AltArgvTerm = HashSet<String>;

    /// A multiple argv is a sequence of AltArgvTerm
    type MultiArgv = Vec<AltArgvTerm>;

    pub struct Script {
        script_config: toml::map::Map<String, toml::Value>,
        loaded_opts: HashSet<String>,
        loaders: Vec<Box<dyn CmdTermLoaderTrait>>,
    }

    impl Script {
        /// loaders should be registered in the order of options when run the full command
        /// each config option should be loaded by only one loader, return error if not
        pub fn register_loader(
            &mut self,
            loader: Box<dyn CmdTermLoaderTrait>,
        ) -> Result<(), String> {
            let loading_opts = loader.loading_opts();
            for opt_name in loading_opts {
                if self.script_config.contains_key(&opt_name) {
                    if self.loaded_opts.contains(&opt_name) {
                        return Err(format!("Option {} already loaded", opt_name));
                    }
                    self.loaded_opts.insert(opt_name);
                } else {
                    return Err(format!("Option {} not found in script file", opt_name));
                }
            }
            self.loaders.push(loader);
            return Ok(());
        }

        // load the config from the script file
        pub fn from_file(path: &str) -> Self {
            let mut script_file = match File::open(path) {
                Ok(file) => {
                    debug!("script file \'{}\' opened", path);
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

            Self {
                script_config: script_config,
                loaded_opts: HashSet::new(),
                loaders: Vec::new(),
            }
        }

        fn gen_terms(&mut self) -> Result<MultiArgv, String> {
            let mut cmd = MultiArgv::new();
            for loader in &mut self.loaders {
                match loader.loads_opts(&self.script_config) {
                    Ok(argv_term) => {
                        cmd.extend(argv_term);
                    }
                    Err(why) => return Err(why),
                }
            }
            Ok(cmd)
        }
    }

    pub trait CmdTermLoaderTrait {
        /// return the options that this loader will load
        fn loading_opts(&self) -> Vec<String>;

        /// loads options from the script config, and return the possible argv terms
        fn loads_opts(
            &mut self,
            script_config: &toml::map::Map<String, toml::Value>,
        ) -> Result<MultiArgv, String>;
    }

    struct OTAWAPropsLoader {}
    impl OTAWAPropsLoader {
        fn new() -> OTAWAPropsLoader {
            OTAWAPropsLoader {}
        }
    }
    impl CmdTermLoaderTrait for OTAWAPropsLoader {
        fn loading_opts(&self) -> Vec<String> {
            vec!["OTAWA_PROPS".to_string(), "OTAWA_LOG_LEVEL".to_string()]
        }

        fn loads_opts(
            &mut self,
            script_config: &toml::map::Map<String, toml::Value>,
        ) -> Result<MultiArgv, String> {
            // get the option for otawa props
            let otawa_props = script_config["OTAWA_PROPS"].as_array().expect(
                "OTAWA_PROPS not found as an array liked field in script file, Aborting...",
            );

            let mut otawa_args = MultiArgv::new();
            for prop in otawa_props {
                if !prop.is_str() {
                    return Err("Each element of OTAWA_PROPS should be a String, which is not the case, Aborting...".to_string());
                }

                // add the "--add-prop" befor the OTAWA_PROPS
                let mut tmp_single_argv = AltArgvTerm::new();
                tmp_single_argv.insert("--add-prop".to_string());
                otawa_args.push(tmp_single_argv);

                tmp_single_argv = AltArgvTerm::new();
                tmp_single_argv.insert(prop.
                    as_str().
                    expect("Each element of OTAWA_PROPS should be a String, which is not the case, Aborting...")
                    .to_string());
                otawa_args.push(tmp_single_argv);
            }

            // get the OTAWA_LOG_LEVEL option
            let otawa_log_level = match script_config["OTAWA_LOG_LEVEL"].as_str() {
                Some(otawa_log_level) => otawa_log_level,
                None => {
                    return Err(
                        "OTAWA_LOG_LEVEL not a String liked field in script file, Aborting..."
                            .to_string(),
                    );
                }
            };

            // add the "--log" before the OTAWA_LOG_LEVEL
            let mut tmp_single_argv = AltArgvTerm::new();
            tmp_single_argv.insert("--log".to_string());
            otawa_args.push(tmp_single_argv);

            // add the log level
            let mut tmp_single_argv = AltArgvTerm::new();
            tmp_single_argv.insert(otawa_log_level.to_string());
            otawa_args.push(tmp_single_argv);

            return Ok(otawa_args);
        }
    }

    struct OTAWAAppLoader {}
    impl OTAWAAppLoader {
        fn new() -> OTAWAAppLoader {
            OTAWAAppLoader {}
        }
    }

    impl CmdTermLoaderTrait for OTAWAAppLoader {
        fn loading_opts(&self) -> Vec<String> {
            vec!["OTAWA_APP_PATH".to_string()]
        }

        fn loads_opts(
            &mut self,
            script_config: &toml::map::Map<String, toml::Value>,
        ) -> Result<MultiArgv, String> {
            let otawa_app_path = match script_config["OTAWA_APP_PATH"].as_str() {
                Some(otawa_app_path) => otawa_app_path,
                None => {
                    return Err(
                        "OTAWA_APP_PATH not a String liked field in script file, Aborting..."
                            .to_string(),
                    );
                }
            };

            let mut tmp_single_argv = AltArgvTerm::new();
            tmp_single_argv.insert(otawa_app_path.to_string());
            return Ok(vec![tmp_single_argv]);
        }
    }

    struct TACLeOptsLoader {}
    impl TACLeOptsLoader {
        fn new() -> TACLeOptsLoader {
            TACLeOptsLoader {}
        }
    }

    impl CmdTermLoaderTrait for TACLeOptsLoader {
        fn loading_opts(&self) -> Vec<String> {
            vec![
                "TACLE_SCRIPT_PATH".to_string(),
                "TACLE_BENCHSET_TO_RUN".to_string(),
            ]
        }

        fn loads_opts(
            &mut self,
            script_config: &toml::map::Map<String, toml::Value>,
        ) -> Result<MultiArgv, String> {
            // get the option in the script
            let tacle_script_path = script_config["TACLE_SCRIPT_PATH"]
                .as_str()
                .expect("cannot load TACLE_SCRIPT_PATH field in the script file, Aborting...");

            let tacle = TACLe::from_script(tacle_script_path);

            let tacle_benchset_to_run = script_config["TACLE_BENCHSET_TO_RUN"]
                .as_array()
                .expect("cannot load TACLE_BENCHSET_TO_RUN field in the script file, Aborting...")
                .clone();

            let mut argv_full_exec_path = AltArgvTerm::new();
            let mut argv_entry_point = AltArgvTerm::new();
            for benchset_to_run in &tacle_benchset_to_run {
                let benchset_name = benchset_to_run
                    .as_str()
                    .expect("each benchset to run must be a String in the script file");
                let run_pairs = tacle.gen_exec_entry_pair(benchset_name);
                for (full_exec_path, entry_point) in &run_pairs {
                    argv_full_exec_path.insert(full_exec_path.clone());
                    argv_entry_point.insert(entry_point.clone());
                }
            }
            return Ok(vec![argv_full_exec_path, argv_entry_point]);
        }
    }

    struct CommandComposition {
        argv_terms: MultiArgv,
    }

    impl CommandComposition {
        fn from_script(script: &mut Script) -> Self {
            let mut argvs = MultiArgv::new();

            // generation all terms
            let generated_terms = script.gen_terms().unwrap();
            for term in generated_terms {
                argvs.push(term);
            }

            CommandComposition { argv_terms: argvs }
        }

        fn build_all_command_to_run(&self) -> Vec<(String, Vec<String>)> {
            let mut res: Vec<(String, Vec<String>)> = Vec::new();
            // the app_name AltArgvTerm must have only one element
            assert_eq!(self.argv_terms[0].len(), 1);
            let cmd_name: Vec<&String> = self.argv_terms[0].iter().collect();
            let app_name = cmd_name[0].clone();

            let empty_cmd = (app_name, Vec::new());
            res.push(empty_cmd);

            for argv_term in &self.argv_terms[1..] {
                let mut new_res = Vec::new();
                // for each possible term, we need to append it to the existing command (res)
                for possible_term in argv_term {
                    for (cmd_name, cmd_opt) in &mut res {
                        let mut new_opt = cmd_opt.clone();
                        new_opt.push(possible_term.clone());
                        new_res.push((cmd_name.clone(), new_opt));
                    }
                }
                res = new_res;
            }
            return res;
        }

        fn run(&self) {
            debug!("command to be run:");
            let cmds = self.build_all_command_to_run();
            for (cmd_str, cmd_opts) in cmds {
                debug!("CMD={}, OPT={:?}", cmd_str, cmd_opts);
            }
        }
    }

    #[macro_use]
    macro_rules! otawa_tacle_scrpit {
        ($file_name:expr) => {{
            let mut script = Script::from_file($file_name);
            let otawa_app_loader = OTAWAAppLoader::new();
            script.register_loader(Box::new(otawa_app_loader)).unwrap();
            let tacle_opt_loader = TACLeOptsLoader::new();
            script.register_loader(Box::new(tacle_opt_loader)).unwrap();
            let otawa_opt_loader = OTAWAPropsLoader::new();
            script.register_loader(Box::new(otawa_opt_loader)).unwrap();
            script
        }};
    }

    #[cfg(test)]
    #[test]
    fn test_otawa_loader() {
        TermLogger::init(
            LevelFilter::Trace,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        )
        .unwrap();
        let mut script =
            otawa_tacle_scrpit!("/home/acac/rust-zexp/scripts/otawa-tacle-exp/exemple.toml");
        let mut command = CommandComposition::from_script(&mut script);
        command.run();
    }
}
