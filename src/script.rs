pub mod script {

    use log::{debug, error, info, trace, warn};
    use simplelog::{ColorChoice, Config, LevelFilter, TermLogger, TerminalMode};
    use std::collections::HashSet;
    use std::fs::File;
    use std::io::prelude::*;
    use std::path::Path;
    use std::process::{exit, Command};
    use toml::Table;

    type ArgvTerm = Vec<Vec<String>>;

    pub struct Script {
        script_config: toml::map::Map<String, toml::Value>,
        loaded_opts: HashSet<String>,
        loaders: Vec<Box<dyn CmdTermLoaderTrait>>,
    }

    impl Script {
        /// loaders should be registered in the order of options when run the full command
        /// each config option should be loaded by only one loader, return error if not
        pub fn register_loader(&mut self, loader: Box<dyn CmdTermLoaderTrait>) -> Result<(), String> {
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

        fn gen_terms(&mut self) -> Result<Vec<ArgvTerm>, String> {
            let mut cmd = Vec::new();
            for loader in &mut self.loaders {
                match loader.loads_opts(&self.script_config) {
                    Ok(argv_term) => {
                        cmd.push(argv_term);
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
        ) -> Result<ArgvTerm, String>;
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
        ) -> Result<ArgvTerm, String> {
            // get the option for otawa props
            let otawa_props = match script_config["OTAWA_PROPS"].as_array() {
                Some(otawa_props) => otawa_props,
                None => {
                    return Err(
                        "OTAWA_PROPS not found as an array liked field in script file, Aborting..."
                            .to_string(),
                    );
                }
            };

            let mut otawa_props_args: Vec<String> = Vec::new();
            for prop in otawa_props {
                if !prop.is_str() {
                    return Err("Each element of OTAWA_PROPS should be a String, which is not the case, Aborting...".to_string());
                }
                otawa_props_args.push("--add-prop".to_string());
                otawa_props_args.push(prop.as_str().unwrap().to_string());
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

            let mut otawa_log_level_args: Vec<String> = Vec::new();
            otawa_log_level_args.push("--log".to_string());
            otawa_log_level_args.push(otawa_log_level.to_string());

            return Ok(vec![otawa_props_args, otawa_log_level_args]);
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
        ) -> Result<ArgvTerm, String> {
            let otawa_app_path = match script_config["OTAWA_APP_PATH"].as_str() {
                Some(otawa_app_path) => otawa_app_path,
                None => {
                    return Err(
                        "OTAWA_APP_PATH not a String liked field in script file, Aborting..."
                            .to_string(),
                    );
                }
            };

            let mut otawa_app_path_args: Vec<String> = Vec::new();
            otawa_app_path_args.push(otawa_app_path.to_string());
            return Ok(vec![otawa_app_path_args]);
        }
    }

    struct TACLeOptsLoader {
        tacle_path: String,
        tacle_bench_set: String,
    }
    impl TACLeOptsLoader {
        /// return a vector of (app_path, entry_fun_name) to simplify the ArgvTerm build afterwards    
        fn kernel_benchs_pairs(&mut self) -> Vec<(String, String)> {
            assert_eq!(self.tacle_bench_set, "kernel");
            self.tacle_path.push_str("/bench/kernel");
            let kernel_path = Path::new(&self.tacle_path);
            assert_eq!(kernel_path.is_dir(), true);

            let mut res = Vec::new();
            match kernel_path.read_dir() {
                Ok(dir) => {
                    for entry in dir {
                        if let Ok(entry) = entry {
                            let app_path =
                                kernel_path.join(entry.path()).to_str().unwrap().to_string();
                            let entry_fun_name = entry.file_name().to_str().unwrap().to_string();
                            res.push((app_path, entry_fun_name));
                        }
                    }
                    res
                }
                Err(_) => {
                    error!("Error when reading kernel benchs dir");
                    exit(-1);
                }
            }
        }
    }

    impl CmdTermLoaderTrait for TACLeOptsLoader {
        fn loading_opts(&self) -> Vec<String> {
            vec!["TACLE_PATH".to_string(), "TACLE_BENCH_SET".to_string()]
        }

        fn loads_opts(
            &mut self,
            script_config: &toml::map::Map<String, toml::Value>,
        ) -> Result<ArgvTerm, String> {
            // get the option for otawa propse
            self.tacle_path = match script_config["TACLE_PATH"].as_str() {
                Some(tacle_path) => tacle_path.to_string(),
                None => {
                    return Err(
                        "TACLE_PATH not found as a String liked field in script file, Aborting..."
                            .to_string(),
                    );
                }
            };
            let pairs = self.kernel_benchs_pairs();
            let mut res = Vec::new();
            for (app_path, entry_fun_name) in pairs {
                let mut run_vec = Vec::new();
                run_vec.push(app_path);
                run_vec.push(entry_fun_name);
                res.push(run_vec);
            }
            return Ok(res);
        }
    }

    struct CommandComposition {
        argv_terms: Vec<ArgvTerm>,
    }

    impl CommandComposition {
        fn from_script(script: &mut Script) -> Self {
            let mut argvs: Vec<ArgvTerm> = Vec::new();

            // generation all terms
            let generated_terms = script.gen_terms().unwrap();
            for term in generated_terms {
                argvs.push(term);
            }

            let only_multiple: Vec<&ArgvTerm> = argvs.iter().filter(|x| x.len() > 1).collect();
            if only_multiple.len() > 1 {
                panic!("So far, i can't handle multiple argv terms that are variable");
            }
            CommandComposition { argv_terms: argvs }
        }

        fn build_all_command_to_run(&self) -> Vec<(String, Vec<String>)> {
            let mut res: Vec<(String, Vec<String>)> = Vec::new();
            let app_name = &self.argv_terms[0][0][0];
            let empty_cmd = (app_name.clone(), Vec::new());
            res.push(empty_cmd);

            for argvs in &self.argv_terms[1..] {
                if argvs.len() == 1 {
                    for (_, existing_argv) in &mut res {
                        existing_argv.extend(argvs[0].clone());
                    }
                } else {
                    for argv in argvs {
                        let mut new_res: Vec<(String, Vec<String>)> = Vec::new();
                        for (app_name, existing_argv) in &res {
                            let mut new_cmd = existing_argv.clone();
                            new_cmd.extend(argv.clone());
                            new_res.push((app_name.clone(), new_cmd));
                        }
                        res = new_res;
                    }
                }
            }
            return res;
        }

        fn run(&self) {
            debug!("command to be run:");
            let cmds = self.build_all_command_to_run();
            for (cmd_str, cmd_opts) in cmds {
                debug!("CMD={}", cmd_str);
                debug!("OPT=");
                for cmd_opt in cmd_opts {
                    debug!("{}", cmd_opt);
                }
            }
        }
    }

    #[macro_use]
    macro_rules! otawa_tacle_scrpit {
        ($file_name:expr) => {{
            let mut script = Script::from_file($file_name);
            let otawa_app_loader = OTAWAAppLoader::new();
            if let Err(err) = script.register_loader(Box::new(otawa_app_loader)) {
                panic!("{}", err);
            }
            let otawa_opt_loader = OTAWAPropsLoader::new();
            if let Err(err) = script.register_loader(Box::new(otawa_opt_loader)) {
                panic!("{}", err);
            }
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
