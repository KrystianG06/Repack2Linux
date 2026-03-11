use std::ffi::OsStr;
use std::fmt;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};

pub struct CommandRunner {
    command: Command,
    allow_failure: bool,
}

#[allow(dead_code)]
pub struct CommandResult {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug)]
pub enum CommandError {
    Io(std::io::Error),
    NonZero {
        command: String,
        status: ExitStatus,
        stdout: String,
        stderr: String,
    },
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandError::Io(err) => write!(f, "I/O error: {}", err),
            CommandError::NonZero {
                command,
                status,
                stderr,
                stdout,
            } => {
                let stderr = trim_output(stderr);
                let stdout = trim_output(stdout);
                write!(
                    f,
                    "command `{}` exited ({}) stderr={} stdout={}",
                    command, status, stderr, stdout
                )
            }
        }
    }
}

impl std::error::Error for CommandError {}

impl CommandRunner {
    pub fn new<S: AsRef<OsStr>>(program: S) -> Self {
        Self {
            command: Command::new(program),
            allow_failure: false,
        }
    }

    #[allow(dead_code)]
    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
        self.command.arg(arg);
        self
    }

    pub fn args<I, S>(&mut self, iter: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.command.args(iter);
        self
    }

    #[allow(dead_code)]
    pub fn env<K, V>(&mut self, key: K, value: V) -> &mut Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.command.env(key, value);
        self
    }

    #[allow(dead_code)]
    pub fn current_dir<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.command.current_dir(path);
        self
    }

    pub fn allow_failure(&mut self, allow: bool) -> &mut Self {
        self.allow_failure = allow;
        self
    }

    pub fn run(&mut self) -> Result<CommandResult, CommandError> {
        let command_line = self.command_line();
        self.command.stdout(Stdio::piped());
        self.command.stderr(Stdio::piped());
        let output = self.command.output().map_err(CommandError::Io)?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if self.allow_failure || output.status.success() {
            Ok(CommandResult {
                status: output.status,
                stdout,
                stderr,
            })
        } else {
            Err(CommandError::NonZero {
                command: command_line,
                status: output.status,
                stdout,
                stderr,
            })
        }
    }

    pub fn spawn(mut self) -> Result<Child, CommandError> {
        self.command.stdin(Stdio::inherit());
        self.command.stdout(Stdio::inherit());
        self.command.stderr(Stdio::inherit());
        self.command.spawn().map_err(CommandError::Io)
    }

    fn command_line(&self) -> String {
        let mut parts = Vec::new();
        parts.push(self.command.get_program().to_string_lossy().to_string());
        for arg in self.command.get_args() {
            parts.push(arg.to_string_lossy().to_string());
        }
        parts.join(" ")
    }
}

fn trim_output(value: &str) -> String {
    const MAX: usize = 120;
    let fmt = value.trim().replace('\n', " ");
    if fmt.len() > MAX {
        format!("{}…", &fmt[..MAX])
    } else {
        fmt
    }
}
