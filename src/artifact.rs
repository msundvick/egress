use ::{
    serde::{Deserialize, Serialize},
    serde_json::Value,
    std::{
        collections::BTreeMap,
        fmt::{self},
    },
};

fn diff_json(mismatches: &mut Vec<Mismatch>, prefix: String, value: &Value, reference: &Value) {
    use Value::*;
    match (value, reference) {
        (Object(map), Object(reference_map)) => {
            for (k, v) in map {
                let v_ref = match reference_map.get(k) {
                    Some(it) => it,
                    None => {
                        mismatches.push(Mismatch::NotInReference(
                            format!("{}.{}", prefix, k),
                            Entry::Json(v.clone()),
                        ));

                        continue;
                    }
                };

                diff_json(&mut *mismatches, format!("{}.{}", prefix, k), v, v_ref);
            }

            for (k, v_ref) in reference_map.iter() {
                if !map.contains_key(k) {
                    mismatches.push(Mismatch::NotProduced(
                        format!("{}.{}", prefix, k),
                        Entry::Json(v_ref.clone()),
                    ));
                }
            }
        }
        (Array(array), Array(array_ref)) => {
            if array.len() != array_ref.len() {
                if array.len() > array_ref.len() {
                    for (i, elem) in array.iter().enumerate().skip(array_ref.len()) {
                        mismatches.push(Mismatch::NotInReference(
                            format!("{}[{}]", prefix, i),
                            Entry::Json(elem.clone()),
                        ));
                    }
                } else if array.len() < array_ref.len() {
                    for (i, elem_ref) in array_ref.iter().enumerate().skip(array.len()) {
                        mismatches.push(Mismatch::NotProduced(
                            format!("{}[{}]", prefix, i),
                            Entry::Json(elem_ref.clone()),
                        ));
                    }
                }

                mismatches.push(Mismatch::LengthMismatch(
                    format!("{}.len()", prefix),
                    array.len(),
                    array_ref.len(),
                ));
            } else {
                for (i, (elem, elem_ref)) in array.iter().zip(array_ref.iter()).enumerate() {
                    diff_json(
                        &mut *mismatches,
                        format!("{}[{}]", prefix, i),
                        elem,
                        elem_ref,
                    );
                }
            }
        }
        (other, other_ref) => {
            if other != other_ref {
                mismatches.push(Mismatch::NotEq(
                    prefix,
                    Entry::Json(other.clone()),
                    Entry::Json(other_ref.clone()),
                ));
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Entry {
    Str(String),
    Json(Value),
    Bytes(Vec<u8>),
    Artifact(Artifact),
}

#[serde(transparent)]
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Artifact {
    entries: BTreeMap<String, Entry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Mismatch {
    NotEq(String, Entry, Entry),
    NotInReference(String, Entry),
    NotProduced(String, Entry),
    LengthMismatch(String, usize, usize),
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

    pub fn insert_debug<T: fmt::Debug>(&mut self, name: &str, value: &T) {
        self.insert(name, Entry::Str(format!("{:#?}", value)));
    }

    pub fn insert_display<T: fmt::Display>(&mut self, name: &str, value: &T) {
        self.insert(name, Entry::Str(value.to_string()));
    }

    pub fn insert_serialize<T: Serialize>(&mut self, name: &str, value: &T) {
        self.insert_json(name, serde_json::to_value(value).unwrap());
    }

    pub fn insert_json(&mut self, name: &str, json_value: Value) {
        self.insert(name, Entry::Json(json_value));
    }

    fn compare_against_reference(&self, prefix: String, reference: &Artifact) -> Vec<Mismatch> {
        let mut mismatches = Vec::new();

        for (k, v) in self.entries.iter() {
            let v_ref = match reference.entries.get(k) {
                Some(it) => it,
                None => {
                    mismatches.push(Mismatch::NotInReference(
                        format!("{}::{}", prefix, k),
                        v.clone(),
                    ));
                    continue;
                }
            };

            use Entry::*;
            match (v, v_ref) {
                (Artifact(art), Artifact(art_ref)) => {
                    mismatches.extend(
                        art.compare_against_reference(format!("{}::{}", prefix, k), art_ref),
                    );
                }
                (Json(json), Json(json_ref)) => {
                    diff_json(
                        &mut mismatches,
                        format!("{}::{}", prefix, k),
                        json,
                        json_ref,
                    );
                }
                (other, other_ref) => {
                    if other != other_ref {
                        mismatches.push(Mismatch::NotEq(
                            format!("{}::{}", prefix, k),
                            other.clone(),
                            other_ref.clone(),
                        ));
                    }
                }
            }
        }

        for (k_ref, v_ref) in reference.entries.iter() {
            if !self.entries.contains_key(k_ref) {
                mismatches.push(Mismatch::NotProduced(
                    format!("{}::{}", prefix, k_ref),
                    v_ref.clone(),
                ));
            }
        }

        mismatches
    }

    pub fn report_mismatches(&self, prefix: String, reference: &Artifact) -> Vec<Mismatch> {
        self.compare_against_reference(prefix, reference)
    }
}
