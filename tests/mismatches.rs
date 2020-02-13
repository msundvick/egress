use std::fs;

#[test]
fn mismatches() {
    let mut meta_egress = egress::egress!();
    let meta_artifact = meta_egress.artifact("mismatches");

    // Refresh so that we're starting from a clean slate
    fs::remove_file("tests/mismatches/egress/artifacts/mismatches/test.json").unwrap();

    let reference_mismatches = {
        let mut egress = egress::egress!("tests/mismatches");
        let artifact = egress.artifact("test");
        artifact
            .insert_serialize("fruits", &vec!["apples", "bananas", "oranges"])
            .unwrap();
        egress.close().unwrap()
    };

    let new_mismatches = {
        let mut egress = egress::egress!("tests/mismatches");
        let artifact = egress.artifact("test");
        artifact
            .insert_serialize("fruits", &vec!["apples", "pears", "oranges"])
            .unwrap();
        egress.close().unwrap()
    };

    meta_artifact
        .insert_serialize("reference_mismatches", &reference_mismatches)
        .unwrap();
    meta_artifact
        .insert_serialize("new_mismatches", &new_mismatches)
        .unwrap();

    meta_egress.close().unwrap().assert_unregressed();
}
