//! A super simple, extremely bare-bones regression test library.
//!
//! This is very minimal and currently doesn't support all that much
//! in the way of API, but if all you want is some super basic regression
//! testing, here it is.
//!
//! ## Example
//!
//! ```rust
//! # use egress::egress;
//! # fn main() {
//! let mut egress = egress!();
//! let artifact = egress.artifact("basic_arithmetic");
//!
//! let super_complex_test_output_that_could_change_at_any_time = 1 + 1;
//!
//! // using `serde::Serialize`:
//! artifact.insert_serialize("1 + 1 (serde)", &super_complex_test_output_that_could_change_at_any_time);
//!
//! // or using `fmt::Debug`:
//! artifact.insert_debug("1 + 1 (fmt::Debug)", &super_complex_test_output_that_could_change_at_any_time);
//!
//! // or using `fmt::Display`:
//! artifact.insert_display("1 + 1 (fmt::Display)", &super_complex_test_output_that_could_change_at_any_time);
//!
//! // More options available; please check the docs.
//!
//! egress.close().unwrap().assert_unregressed();
//! # }
//! ```
//!
//! To see the artifacts produced by this example, check `egress/artifacts/rust_out/basic_arithmetic.json`.
//!

#![deny(missing_docs)]

use ::{
    fs2::FileExt,
    serde::{Deserialize, Serialize},
    std::{
        collections::HashMap,
        fs::{self, File, OpenOptions},
        io::{Read, Write},
        path::PathBuf,
    },
};

mod artifact;
mod error;

use artifact::Mismatch;

pub use artifact::{Artifact, Entry};
pub use error::ErrorKind;
#[doc(hidden)]
pub use std::path::Path; // for macros

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EgressConfig {
    artifact_dir: PathBuf,
    atol: Option<f64>,
    rtol: Option<f64>,
}

impl EgressConfig {
    fn new() -> Self {
        EgressConfig {
            artifact_dir: PathBuf::from("egress/artifacts/"),
            atol: Some(0.0),
            rtol: Some(0.0),
        }
    }
}

/// Comparison report for newly generated artifacts versus the artifacts stored in
/// `artifacts_subdir`.
#[must_use]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(transparent)]
pub struct Report {
    mismatches: Vec<Mismatch>,
}

impl Report {
    /// If any mismatches were found, this function will iterate through and print info
    /// about them to stdout, before panicking.
    pub fn assert_unregressed(self) {
        if !self.mismatches.is_empty() {
            for mismatch in self.mismatches {
                match mismatch {
                    Mismatch::NotEq(k, new_value, reference) => {
                        eprintln!(
                            "MISMATCH: entry `{}` not the same as the reference value",
                            k
                        );

                        eprintln!(
                            "Reference value:\n{}",
                            serde_json::to_string(&reference).unwrap()
                        );

                        eprintln!("New value:\n{}", serde_json::to_string(&new_value).unwrap());
                    }
                    Mismatch::NotInReference(k, _) => {
                        eprintln!("MISMATCH: entry `{}` does not exist in the reference", k)
                    }
                    Mismatch::NotProduced(k, _) => eprintln!(
                        "MISMATCH: entry `{}` exists in the reference but was not found here",
                        k
                    ),
                    Mismatch::LengthMismatch(k, len, len_ref) => eprintln!("MISMATCH: entry `{}` has length `{}` in the reference but length `{}` in the newly produced artifact.", k, len_ref, len),
                }
            }
            panic!("End found mismatches; panicking to fail the test.");
        }
    }
}

/// A testing context. You can open as many as you want, but make sure their `artifact_subdir`s don't collide.
#[derive(Debug)]
pub struct Egress {
    artifact_subdir: PathBuf,
    artifacts: HashMap<PathBuf, Artifact>,
    /// Set the absolute tolerance (absolute(a - b) <= atol)
    pub atol: Option<f64>,
    /// Set the relative tolerance (absolute(a - b) <= rtol * absolute(b))
    pub rtol: Option<f64>,
}

impl Egress {
    /// Open a new `Egress` context, given the path to a directory containing an `Egress.toml` config file
    /// and a subpath for where this `Egress` context should place its artifacts relative to the configured
    /// `artifact_subdir`.
    ///
    /// If an `Egress.toml` file is not found, one will be initialized with the default values at the directory
    /// indicated by `config_dir`.
    pub fn open<P, Q>(config_dir: P, artifact_subdir: Q) -> Result<Self, ErrorKind>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let path = config_dir.as_ref().join("Egress.toml");

        if !path.exists() {
            fs::create_dir_all(&config_dir)?;
            let mut config_file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)?;

            config_file.lock_exclusive()?;

            let config_string = toml::ser::to_string_pretty(&EgressConfig::new())?;
            config_file.write_all(config_string.as_bytes())?;

            config_file.unlock()?;
        }

        let mut file = File::open(path)?;
        file.lock_shared()?;

        let config: EgressConfig = {
            let mut s = String::new();
            file.read_to_string(&mut s)?;
            toml::de::from_str(&s)?
        };

        let artifact_subdir = config_dir
            .as_ref()
            .join(&config.artifact_dir)
            .join(artifact_subdir.as_ref());

        let artifacts = HashMap::new();

        Ok(Self {
            artifact_subdir,
            artifacts,
            atol: config.atol,
            rtol: config.rtol,
        })
    }

    /// Construct a new `Artifact` reference. Any data inserted into the artifact returned
    /// will be written into a directory inside the `artifact_dir` configured in `Egress.toml`.
    pub fn artifact<P: AsRef<Path>>(&mut self, name: P) -> &mut Artifact {
        let path = name
            .as_ref()
            .file_stem()
            .expect("artifact name must be a file stem!")
            .to_owned();
        assert_eq!(&path, name.as_ref(), "artifact name must be a file stem!");

        use std::collections::hash_map::Entry::*;
        match self.artifacts.entry(PathBuf::from(path)) {
            Occupied(_) => panic!(
                "only one artifact allowed with the name `{}`!",
                name.as_ref().display()
            ),
            Vacant(vacant) => vacant.insert(Artifact::new()),
        }
    }

    /// Close the testing context and write new artifacts to disk before reporting
    /// any artifacts which don't match the reference values stored in the `egress/artifacts`
    /// folder.
    pub fn close(self) -> Result<Report, ErrorKind> {
        let mut mismatches = Vec::new();

        fs::create_dir_all(&self.artifact_subdir)?;
        for (path, artifact) in self.artifacts.iter() {
            let mut path_to_file = self.artifact_subdir.join(path);
            path_to_file.set_extension("json");

            if path_to_file.exists() {
                let mut file = File::open(&path_to_file)?;
                let reference = serde_json::from_reader(&mut file)?;
                mismatches.extend(artifact.report_mismatches(
                    path.to_string_lossy().into_owned(),
                    &reference,
                    self.atol,
                    self.rtol,
                ));
            } else {
                let mut file = File::create(&path_to_file)?;
                serde_json::to_writer_pretty(&mut file, artifact)?;
            }
        }

        Ok(Report { mismatches })
    }

    /// Shorthand for `.close()?.assert_unregressed()?`.
    pub fn close_and_assert_unregressed(self) -> Result<(), ErrorKind> {
        self.close()?.assert_unregressed();
        Ok(())
    }
}

/// Shorthand macro for opening an Egress context, keyed by the `module_path!()`
/// of the file it's called in.
///
/// It can also be called with a path, in which case the `Egress.toml` config file
/// and `egress` artifact folder will be placed at that path offset from the path
/// provided by the `CARGO_MANIFEST_DIR` environment variable, which by default is
/// wherever your `Cargo.toml` is.
#[macro_export]
macro_rules! egress {
    () => {{
        let path = module_path!().replace("::", "/");
        $crate::Egress::open(env!("CARGO_MANIFEST_DIR"), path)
            .expect("failed to open Egress context")
    }};
    ($path:literal) => {{
        let path = module_path!().replace("::", "/");
        let root_path = $crate::Path::new(env!("CARGO_MANIFEST_DIR")).join($path);
        $crate::Egress::open(root_path, path).expect("failed to open Egress context")
    }};
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn open() {
        let _ = egress!();
    }
}
