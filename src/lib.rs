use ::{
    fs2::FileExt,
    serde::{Deserialize, Serialize},
    std::{
        collections::HashMap,
        fs::{self, File, OpenOptions},
        io::{self, Read, Write},
        path::PathBuf,
    },
};

pub mod artifact;
pub mod error;

#[doc(hidden)]
pub use std::path::Path; // for macros

use crate::{
    artifact::{Artifact, Mismatch},
    error::ErrorKind,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EgressConfig {
    root: PathBuf,
    artifact_dir: PathBuf,
}

impl EgressConfig {
    pub fn new(path: &Path) -> Self {
        EgressConfig {
            root: path.to_owned(),
            artifact_dir: PathBuf::from("egress/artifacts/"),
        }
    }
}

#[must_use]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Report {
    mismatches: Vec<Mismatch>,
}

impl Report {
    pub fn assert_unregressed(self) {
        if !self.mismatches.is_empty() {
            for mismatch in self.mismatches {
                match mismatch {
                    Mismatch::NotEq(k, _, _) => println!(
                        "MISMATCH: entry `{}` not the same as the reference value",
                        k
                    ),
                    Mismatch::NotInReference(k, _) => {
                        println!("MISMATCH: entry `{}` does not exist in the reference", k)
                    }
                    Mismatch::NotProduced(k, _) => println!(
                        "MISMATCH: entry `{}` exists in the reference but was not found here",
                        k
                    ),
                }
            }

            panic!("Found mismatches.");
        }
    }
}

#[derive(Debug)]
pub struct Egress {
    file: File,
    config: EgressConfig,
    artifact_subdir: PathBuf,
    artifacts: HashMap<PathBuf, Artifact>,
}

impl Egress {
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

            let config_string =
                toml::ser::to_string_pretty(&EgressConfig::new(config_dir.as_ref()))?;
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

        let artifact_subdir = config
            .root
            .join(&config.artifact_dir)
            .join(artifact_subdir.as_ref());

        let artifacts = HashMap::new();

        Ok(Self {
            file,
            config,
            artifact_subdir,
            artifacts,
        })
    }

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

    pub fn close(self) -> Result<Report, ErrorKind> {
        let mut mismatches = Vec::new();

        fs::create_dir_all(&self.artifact_subdir)?;
        for (path, artifact) in self.artifacts.iter() {
            let mut path_to_file = self.artifact_subdir.join(path);
            path_to_file.set_extension("json");

            if path_to_file.exists() {
                let mut file = File::open(&path_to_file)?;
                let reference = serde_json::from_reader(&mut file)?;
                mismatches.extend(artifact.report_mismatches(&reference));
            } else {
                let mut file = File::create(&path_to_file)?;
                serde_json::to_writer_pretty(&mut file, artifact)?;
            }
        }

        Ok(Report { mismatches })
    }
}

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
