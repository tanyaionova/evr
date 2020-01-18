use serde_derive::{ Serialize, Deserialize };
use crate::backends::{ Backend, RunStatus };
use std::process::{ Command, Stdio };
use std::path::Path;
use std::fs::File;
use log::{ warn, error };
use wait_timeout::ChildExt;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PythonBackend {
    template: Option<String>,
    version: Option<String>,
    timeout: Option<f32>,
}


impl PythonBackend {
    fn get_interpreter(&self) -> String {
        format!(
            "python{}",
            self.version
                .as_ref()
                .unwrap_or(&String::new())
        )
    }
}


impl Backend for PythonBackend {
    fn get_template(&self) -> Option<&str> {
        match self.template {
            Some(ref t) => Some(t),
            None => None
        }
    }

    fn run(&self, fname: &Path) -> std::io::Result<RunStatus> {
        let stdio = match self.try_guess_test_file(fname) {
            Some(test_filename) => match File::open(test_filename) {
                Ok(test_content) => Stdio::from(test_content),
                Err(err) => {
                    warn!("Could not open test file. Fallback to piped: {}", err);
                    Stdio::piped()
                }
            },
            None => Stdio::piped()
        };

        let timer = std::time::SystemTime::now();

        let mut child = Command::new(self.get_interpreter())
            .arg(fname.as_os_str())
            .stdin(stdio)
            .spawn()?;

        let timeout = std::time::Duration::from_secs_f32(self.timeout.unwrap_or(1.0));
        match child.wait_timeout(timeout) {
            Ok(maybe_status) => match maybe_status {
                Some(status) => {
                    if !status.success() {
                        return Ok(RunStatus::ErrorCode(status.code().unwrap_or(0)));
                    }
                    Ok(RunStatus::Success)
                },
                None => {
                    let elapsed = timer.elapsed().unwrap_or(Default::default());
                    child.kill()?;
                    child.wait()?;
                    Ok(RunStatus::TimedOut(elapsed))
                }
            },
            Err(err) => {
                error!("could not wait for child: {}", err);
                child.kill()?;
                child.wait()?; // Wait defunct
                Err(err)
            }
        }
    }
}