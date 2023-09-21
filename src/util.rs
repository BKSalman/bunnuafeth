use std::ffi::OsStr;
use std::process::Command;

type Error = Box<dyn std::error::Error + Send + Sync>;

pub trait CommandExt {
    fn with_arg(self, arg: &str) -> Self;
    fn with_args<I, S>(self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>;
    fn run_with_output(self, msg: &str) -> Result<String, Error>;
}

impl CommandExt for Command {
    fn with_arg(mut self, arg: &str) -> Self {
        self.arg(arg);

        self
    }

    fn with_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.args(args);

        self
    }

    fn run_with_output(mut self, msg: &str) -> Result<String, Error> {
        println!("{msg}");
        let output = self.output()?;
        if let Err(e) = output.check() {
            println!("stderr:\n\t{}", String::from_utf8_lossy(&output.stderr));
            return Err(Box::new(e));
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

pub trait CheckStatus {
    fn check(&self) -> Result<(), ProcessError>;
}

impl CheckStatus for std::process::ExitStatus {
    fn check(&self) -> Result<(), ProcessError> {
        match self.success() {
            true => Ok(()),
            false => Err(ProcessError::ErrorCode(self.code().unwrap_or(1))),
        }
    }
}

impl CheckStatus for std::process::Output {
    fn check(&self) -> Result<(), ProcessError> {
        self.status.check()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProcessError {
    #[error("Process exited with error code {0}")]
    ErrorCode(i32),
}
