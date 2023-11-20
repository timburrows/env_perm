use std::{process::Command, env};
use filepath::FilePath;

fn main() {
    // // Check if DUMMY is set, if not set it to 1
    // // export DUMMY=1
    // env_perm::check_or_set("DUMMY", 1).expect("Failed to find or set DUMMY");
    // // Append $HOME/some/cool/bin to $PATH
    // // export PATH= "$HOME/some/cool/bin:$PATH"
    // env_perm::append("PATH", "$HOME/some/cool/bin").expect("Couldn't find PATH");
    // // Sets a variable without checking if it exists.
    // // Note you need to use a raw string literal to include ""
    // // export DUMMY="/something"
    // env_perm::set("DUMMY", r#""/something""#).expect("Failed to set DUMMY");

    // Sets a variable and captures the File, for example, if you want to source it afterward
    let file = env_perm::check_or_set("DUMMY_SOURCE", 123).expect("Failed to find or set DUMMY_SOURCE");
    let shell_bin = env::var("SHELL").expect("SHELL environment variable was not found");
    let file_path = file.path().expect("Did not find path").display().to_string();

    let _ = Command::new(shell_bin)
        .arg("-c")
        .arg(format!("source {}", file_path))
        .output()
        .expect("Failed to source file");
}
