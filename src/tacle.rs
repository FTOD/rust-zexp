use log::{debug, error, info, trace, warn};
use simplelog::{ColorChoice, Config, LevelFilter, TermLogger, TerminalMode};
use std::fs::File;
use std::io::prelude::*;
use std::{
    fmt,
    path::{Path, PathBuf},
};
use toml::Table;

pub struct Bench {
    name: String,
    path: PathBuf,
    entry_point: String,
}

pub struct BenchSet {
    name: String,
    path: PathBuf,
    benchs: Vec<Bench>,
}

pub struct TACLe {
    root_path: PathBuf,
    benchsets: Vec<BenchSet>,
}

impl TACLe {
    pub fn from_script(script_path: &str) -> TACLe {
        let mut script_file =
            File::open(script_path).expect(&format!("tacle script file not found"));
        let mut script = String::new();
        script_file.read_to_string(&mut script).expect(&format!(
            "something went wrong reading the tacle script file {}",
            script_path
        ));

        let script_config = script
            .parse::<Table>()
            .expect("script file opened but error parsing it");

        let tacle_root_path = PathBuf::from(
            script_config["tacle_root_path"]
                .as_str()
                .expect("tacle_root_path must be a string"),
        );

        let benchset_list = Table::try_from(&script_config["TACLE_BENCHSET_LIST"])
            .expect("cannot convert TACLEBENCHSET_List to table, check your script file");

        let mut tacle = Vec::new();
        for (key, value) in &benchset_list {
            let benchs_in_script = value.as_table().expect("each bench set must be a table")
                ["benchs"]
                .as_array()
                .expect("benchs must be an array of benchs");
            let mut benchs = Vec::new();
            for bench in benchs_in_script {
                let bench = bench.as_table().expect("bench name must be a table");
                let new_bench = Bench {
                    name: bench["name"]
                        .as_str()
                        .expect("bench name must be a string")
                        .to_string(),
                    path: PathBuf::from(
                        bench["exec"]
                            .as_str()
                            .expect("benchset_path must be a string"),
                    ),
                    entry_point: bench["entry_point"]
                        .as_str()
                        .expect("bench entry_point must be a string")
                        .to_string(),
                };
                benchs.push(new_bench);
            }
            let mut new_benchset = BenchSet {
                name: key.clone(),
                path: PathBuf::from(
                    value.as_table().expect("each bench set must be a table")["benchset_path"]
                        .as_str()
                        .expect("benchset_path must be a string"),
                ),
                benchs,
            };
            tacle.push(new_benchset);
        }

        TACLe {
            root_path: tacle_root_path,
            benchsets: tacle,
        }
    }

    /// generate pairs of executable and entry point for the given benche set
    pub fn gen_exec_entry_pair(&self, benchset_name: &str) -> Vec<(String, String)> {
        let mut res = Vec::new();
        for benchset in &self.benchsets {
            if benchset.name == benchset_name {
                for bench in &benchset.benchs {
                    let full_exec_path = self.root_path.join(&benchset.path).join(&bench.path);
                    res.push((
                        full_exec_path
                            .to_str()
                            .expect("failed to convert path to string?")
                            .to_string(),
                        bench.entry_point.clone(),
                    ));
                }
            }
            return res;
        }
        return Vec::new();
    }
}
#[cfg(test)]
#[test]
fn test_tacle() {
    let script_path = "/home/acac/rust-zexp/scripts/otawa-tacle-exp/tacle.toml";
    let tacle = TACLe::from_script(script_path);
    debug!("{:?}", tacle.gen_exec_entry_pair("kernel"));
}
