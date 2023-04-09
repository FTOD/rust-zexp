use serde::Deserialize;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;
use toml::Table;

#[derive(Debug, Deserialize, Clone)]
pub struct Bench {
    pub name: String,
    pub exec: String,
    pub entry_point: String,
}

#[derive(Deserialize)]
pub struct BenchSet {
    name: String,
    path_from_root: PathBuf,
    benchs: Vec<Bench>,
}

#[derive(Deserialize)]
pub struct TACLe {
    root_path: String,
    benchsets: Vec<BenchSet>,
}

impl TACLe {
    pub fn from_script(script_path: &str) -> TACLe {
        let mut file = File::open(script_path)
            .expect(format!("failed to open script file {}", script_path).as_str());

        let mut script_content = String::new();
        file.read_to_string(&mut script_content)
            .expect("Error when reading script file");

        let script_content = script_content
            .parse::<Table>()
            .expect("Error when parsing the script file, check you TOML syntax!");

        let mut res: TACLe = script_content.try_into().unwrap();
        res.patch_full_exec_name();
        res
    }

    /// the exec of each bench is only the path from the benchset root, so patch it to have absolute path
    fn patch_full_exec_name(&mut self) {
        for benchset in self.benchsets.iter_mut() {
            for bench in benchset.benchs.iter_mut() {
                let full_exec_name = PathBuf::from(&self.root_path)
                    .join(&benchset.path_from_root)
                    .join(&bench.exec);
                bench.exec = full_exec_name.to_str().unwrap().to_string();
            }
        }
    }

    /// return a vector of benchs with respect to the benchset name given
    pub fn select_bench(&self, benchset_name: &Vec<String>) -> Vec<Bench> {
        let benchsets: Vec<&BenchSet> = self
            .benchsets
            .iter()
            .filter(|x| benchset_name.contains(&x.name))
            .collect();
        let res: Vec<Bench> = benchsets.clone().iter().fold(Vec::new(), |mut acc, x| {
            acc.extend(x.benchs.clone());
            acc
        });
        res
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use log::debug;
    #[test]
    fn test_tacle() {
        let script_path = "/home/acac/rust-zexp/scripts/otawa-tacle-exp/tacle.toml";
        let tacle = TACLe::from_script(script_path);
        debug!("{:?}", tacle.select_bench(&vec!["kernel".to_string()]));
    }
}
