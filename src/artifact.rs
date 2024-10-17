//! Testing artifacts. These are bits of data produced by your tests that Egress will compare with
//! previously produced "reference" artifacts.

use ::{
    serde::{Deserialize, Serialize},
    serde_json::Value,
    std::{
        collections::BTreeMap,
        fmt::{self},
    },
};

use crate::ErrorKind;

fn compare_float(a: f64, b: f64, atol: Option<f64>, rtol: Option<f64>) -> bool {
    match (atol, rtol) {
        (None, None) => a == b,
        (None, Some(rtol)) => (a - b) <= rtol * b.abs(),
        (Some(atol), None) => (a - b).abs() <= atol,
        (Some(atol), Some(rtol)) => ((a - b) <= rtol * b.abs()) && ((a - b).abs() <= atol),
    }
}

fn diff_json(
    mismatches: &mut Vec<Mismatch>,
    prefix: String,
    value: &Value,
    reference: &Value,
    atol: Option<f64>,
    rtol: Option<f64>,
) {
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

                diff_json(
                    &mut *mismatches,
                    format!("{}.{}", prefix, k),
                    v,
                    v_ref,
                    atol,
                    rtol,
                );
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

                mismatches.push(Mismatch::LengthMismatch(
                    format!("{}.len()", prefix),
                    array.len(),
                    array_ref.len(),
                    Entry::Json(array.clone().into()),
                    Entry::Json(array_ref.clone().into()),
                ));
            } else {
                for (i, (elem, elem_ref)) in array.iter().zip(array_ref.iter()).enumerate() {
                    diff_json(
                        &mut *mismatches,
                        format!("{}[{}]", prefix, i),
                        elem,
                        elem_ref,
                        atol,
                        rtol,
                    );
                }
            }
        }
        (Number(a), Number(b)) => {
            if let (Some(a), Some(b)) = (a.as_i64(), b.as_i64()) {
                if a != b {
                    mismatches.push(Mismatch::NotEq(
                        prefix,
                        Entry::Json(a.into()),
                        Entry::Json(b.into()),
                    ));
                }
            } else if let (Some(a), Some(b)) = (a.as_f64(), b.as_f64()) {
                if !compare_float(a, b, atol, rtol) {
                    mismatches.push(Mismatch::NotEq(
                        prefix,
                        Entry::Json(a.into()),
                        Entry::Json(b.into()),
                    ));
                }
            } else {
                if a != b {
                    mismatches.push(Mismatch::NotEq(
                        prefix,
                        Entry::Json(a.clone().into()),
                        Entry::Json(b.clone().into()),
                    ));
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

/// Artifacts are maps from string keys to `Entry` objects. Entries in an
/// artifact can be strings, JSON values, byte buffers, or - because
/// artifacts are tree structured - another `Artifact`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Entry {
    /// A string entry.
    Str(String),

    /// A JSON entry. The `Value` type comes from the `serde_json` crate.
    Json(Value),

    /// A raw byte entry.
    Bytes(Vec<u8>),

    /// An artifact entry.
    Artifact(Artifact),
}

/// An `Artifact` is the main object that Egress uses to handle and compare
/// data produced from your tests. It's basically just a map from string keys
/// to `Entry`s.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(transparent)]
pub struct Artifact {
    entries: BTreeMap<String, Entry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Mismatch {
    NotEq(String, Entry, Entry),
    NotInReference(String, Entry),
    NotProduced(String, Entry),
    LengthMismatch(String, usize, usize, Entry, Entry),
}

impl Artifact {
    /// Create an empty `Artifact`. This is useful for building tree-structured
    /// artifacts, but the root artifact for a given test should always come from
    /// `Egress::artifact`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert an `Entry` into the artifact, with a given string name. The other
    /// `insert_*` methods are just convenient wrappers around this one.
    pub fn insert(&mut self, name: &str, entry: Entry) {
        if self.entries.insert(name.to_string(), entry).is_some() {
            panic!(
                "Duplicate entries under the same name (`{}`) are not allowed!",
                name
            );
        }
    }

    /// Convert a value to a string via the `fmt::Debug` formatter and then insert
    /// that into the `Artifact` with the given string key.
    pub fn insert_debug<T: fmt::Debug>(&mut self, name: &str, value: &T) {
        self.insert(name, Entry::Str(format!("{:#?}", value)));
    }

    /// Convert a value to a string via the `fmt::Display` formatter and then insert
    /// that into the `Artifact` with the given string key.
    pub fn insert_display<T: fmt::Display>(&mut self, name: &str, value: &T) {
        self.insert(name, Entry::Str(value.to_string()));
    }

    /// Convert a value to a JSON value via `serde_json` and then insert that into
    /// the `Artifact` with the given string key.
    ///
    /// Egress uses `serde` to do this, so if you want to be able to have nicely formatted
    /// diffs between your types, you'll want them to derive `serde::{Serialize}`.
    pub fn insert_serialize<T: Serialize>(
        &mut self,
        name: &str,
        value: &T,
    ) -> Result<(), ErrorKind> {
        self.insert_json(name, serde_json::to_value(value)?);
        Ok(())
    }

    /// Insert a JSON `Value` into the `Artifact` with the given string key.
    pub fn insert_json(&mut self, name: &str, json_value: Value) {
        self.insert(name, Entry::Json(json_value));
    }

    fn compare_against_reference(
        &self,
        prefix: String,
        reference: &Artifact,
        atol: Option<f64>,
        rtol: Option<f64>,
    ) -> Vec<Mismatch> {
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
                    mismatches.extend(art.compare_against_reference(
                        format!("{}::{}", prefix, k),
                        art_ref,
                        atol,
                        rtol,
                    ));
                }
                (Json(json), Json(json_ref)) => {
                    diff_json(
                        &mut mismatches,
                        format!("{}::{}", prefix, k),
                        json,
                        json_ref,
                        atol,
                        rtol,
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

    pub(crate) fn report_mismatches(
        &self,
        prefix: String,
        reference: &Artifact,
        atol: Option<f64>,
        rtol: Option<f64>,
    ) -> Vec<Mismatch> {
        self.compare_against_reference(prefix, reference, atol, rtol)
    }
}
