use ::{
    serde::{Deserialize, Serialize},
    std::{
        collections::BTreeMap,
        fmt::{Debug, Display},
    },
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Entry {
    Str(String),
    Bytes(Vec<u8>),
    Artifact(Artifact),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Artifact {
    entries: BTreeMap<String, Entry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Mismatch {
    NotEq(String, Entry, Entry),
    NotInReference(String, Entry),
    NotProduced(String, Entry),
}

impl Artifact {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, name: &str, entry: Entry) {
        if self.entries.insert(name.to_string(), entry).is_some() {
            panic!(
                "Duplicate entries under the same name (`{}`) are not allowed!",
                name
            );
        }
    }

    pub fn insert_debug<T: Debug>(&mut self, name: &str, value: &T) {
        self.insert(name, Entry::Str(format!("{:?}", value)));
    }

    pub fn insert_display<T: Display>(&mut self, name: &str, value: &T) {
        self.insert(name, Entry::Str(value.to_string()));
    }

    pub fn insert_serialize<T: Serialize>(&mut self, name: &str, value: &T) {
        self.insert(
            name,
            Entry::Str(serde_json::to_string_pretty(value).unwrap()),
        );
    }

    fn compare_against_reference(&self, prefix: String, reference: &Artifact) -> Vec<Mismatch> {
        let mut mismatches = Vec::new();

        for (k, v) in self.entries.iter() {
            let v_ref = match reference.entries.get(k) {
                Some(it) => it,
                None => {
                    mismatches.push(Mismatch::NotInReference(k.clone(), v.clone()));
                    continue;
                }
            };

            use Entry::*;
            match (v, v_ref) {
                (Artifact(art), Artifact(art_ref)) => {
                    mismatches.extend(art.compare_against_reference(prefix.clone() + k, art_ref));
                }
                (other, other_ref) => {
                    if other != other_ref {
                        mismatches.push(Mismatch::NotEq(
                            prefix.clone() + k,
                            other.clone(),
                            other_ref.clone(),
                        ));
                    }
                }
            }
        }

        for (k_ref, v_ref) in reference.entries.iter() {
            if !self.entries.contains_key(k_ref) {
                mismatches.push(Mismatch::NotProduced(prefix.clone() + k_ref, v_ref.clone()));
            }
        }

        mismatches
    }

    pub fn report_mismatches(&self, reference: &Artifact) -> Vec<Mismatch> {
        self.compare_against_reference(String::new(), reference)
    }
}
