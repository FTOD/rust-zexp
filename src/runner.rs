use crate::script::*;
use std::process::Command;
type Exec = String;
type Opts = Vec<String>;
struct Runner {
    cmds: Vec<(Exec, Opts)>,
}

impl Runner {
    pub fn from_script(script: &mut Script) -> Self {
        let cmds = script.gen_cmd().unwrap();
        Runner { cmds }
    }

    // pub fn run(&self, nb_cores: u8) {
    //     for (exec, opts) in &self.cmds {
    //         let mut cmd = Command::new(exec).args(opts);
    //         let output = cmd.output().expect("failed to execute process");
    //         println!("status: {}", output.status);
    //         println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    //         println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    //     }
    // }
}
