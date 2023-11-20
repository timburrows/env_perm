//! This crate allows you to permanently set environment variables
//!
//! # Examples
//! ```rust
//! // Check if DUMMY is set, if not set it to 1
//! // export DUMMY=1
//! env_perm::check_or_set("DUMMY", 1).expect("Failed to find or set DUMMY");
//! // Append $HOME/some/cool/bin to $PATH
//! // export PATH= "$HOME/some/cool/bin:$PATH"
//! env_perm::append("PATH", "$HOME/some/cool/bin").expect("Couldn't find PATH");
//! // Sets a variable without checking if it exists.
//! // Note you need to use a raw string literal to include ""
//! // export DUMMY="/something"
//! env_perm::set("DUMMY", r#""/something""#).expect("Failed to set DUMMY");
//! ```

use std::borrow::Cow;

use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;
use std::{env, fs};
use std::{fmt, io};

use boolinator::Boolinator;
use phf::phf_map;

#[derive(Debug, PartialEq)]
enum ShellBin {
    Zsh,
    Bash,

    NotSupported,
}

impl FromStr for ShellBin {
    type Err = ShellBin;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_uppercase();
        let s = s.as_str();

        match s {
            "ZSH" => Ok(ShellBin::Zsh),
            "BASH" => Ok(ShellBin::Bash),
            _ => Err(ShellBin::NotSupported),
        }
    }
}

static SHELL: [ShellProfile; 2] = [
    ShellProfile {
        shell_bin: ShellBin::Zsh,
        shell_cfg_files: phf_map! {
            "profile" => Cow::Borrowed(".zprofile"),
            "login" => Cow::Borrowed(".zlogin"),
            "shellrc" => Cow::Borrowed(".zshrc"),
        },
    },
    ShellProfile {
        shell_bin: ShellBin::Bash,
        shell_cfg_files: phf_map! {
            "profile" => Cow::Borrowed(".bash_profile"),
            "login" => Cow::Borrowed(".bash_login"),
            "shellrc" => Cow::Borrowed(".bashrc"),
        },
    },
];

#[derive(Debug)]
struct ShellProfile {
    shell_bin: ShellBin,
    shell_cfg_files: phf::Map<&'static str, Cow<'static, str>>,
}

/// Checks if a environment variable is set.
/// If it is then nothing will happen.
/// If it's not then it will be added
/// to your profile.
pub fn check_or_set<T, U>(var: T, value: U) -> io::Result<()>
where
    T: fmt::Display + AsRef<std::ffi::OsStr>,
    U: fmt::Display,
{
    env::var(&var).map(|_| ()).or_else(|_| set(var, value))
}

/// Appends a value to an environment variable
/// Useful for appending a value to PATH
pub fn append<T: fmt::Display>(var: T, value: T) -> io::Result<()> {
    let mut profile = get_profile()?;
    writeln!(profile, "\nexport {}=\"{}:${}\"", var, value, var)?;
    profile.flush()
}

/// Sets an environment variable without checking
/// if it exists.
/// If it does you will end up with two
/// assignments in your profile.
/// It's recommended to use `check_or_set`
/// unless you are certain it doesn't exist.
pub fn set<T: fmt::Display, U: fmt::Display>(var: T, value: U) -> io::Result<()> {
    let mut profile = get_profile()?;
    writeln!(profile, "\nexport {}={}", var, value)?;
    profile.flush()
}

fn get_profile() -> io::Result<File> {
    let shell_bin = env::var("SHELL").expect("SHELL environment variable was not found");
    let mut shell_bin = shell_bin.as_str();

    dirs::home_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No home directory"))
        .and_then(|hd| {
            hd.clone()
                .as_path()
                .to_str()
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Failed to coerce Home directory as a valid Path",
                    )
                })
                .and_then(|profile| {
                    shell_bin = shell_bin
                        .split('/')
                        .last()
                        .expect("Unable to parse shell path in environment variables");

                    fs::metadata(profile)
                        .map_err(|_| {
                            io::Error::new(
                                io::ErrorKind::InvalidInput,
                                "Path to profile was invalid, or was not found",
                            )
                        })
                        .and_then(|md| {
                            let readonly = !md.permissions().readonly();
                            readonly.as_result(
                                md,
                                io::Error::new(
                                    io::ErrorKind::PermissionDenied,
                                    "Unable to write to home directory, cannot export env var",
                                ),
                            )
                        })
                        .map(|_| profile)
                })
                .map(PathBuf::from)
        })
        .and_then(|path| find_profile(path, shell_bin))
}

#[cfg(target_family = "unix")]
fn find_profile(mut profile: PathBuf, shell_bin: &str) -> io::Result<File> {
    let mut open_opts = std::fs::OpenOptions::new();
    open_opts.append(true).create(false);

    if let Some(sp) = SHELL.iter().find(|sp| {
        sp.shell_bin
            == ShellBin::from_str(shell_bin)
                .expect("Unable to match shell_bin with a supported ShellType")
    }) {
        let entries = sp.shell_cfg_files.entries();
        for (k, v) in entries {
            if !v.is_empty() {
                if k == &"profile" {
                    open_opts.create(true);
                }

                profile.push(v.as_ref());

                return match open_opts.open(profile.clone()) {
                    Ok(f) => {
                        println!("Selected: {}", profile.display());
                        Ok(f)
                    }
                    Err(_) => {
                        profile.pop();
                        continue;
                    }
                };
            }
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "No shell profiles were found",
    ))
}
