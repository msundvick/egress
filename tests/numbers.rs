use egress::egress;

#[test]
fn mismatches() {
    let mut egress = egress!();
    egress.atol = Some(0.001);
    let artifact = egress.artifact("basic_arithmetic");

    let super_complex_test_output_that_could_change_at_any_time = 2;

    // using `serde::Serialize`:
    artifact
        .insert_serialize(
            "1 + 1 (serde)",
            &super_complex_test_output_that_could_change_at_any_time,
        )
        .unwrap();

    // // or using `fmt::Debug`:
    // artifact.insert_debug(
    //     "1 + 1 (fmt::Debug)",
    //     &super_complex_test_output_that_could_change_at_any_time,
    // );

    // // or using `fmt::Display`:
    // artifact.insert_display(
    //     "1 + 1 (fmt::Display)",
    //     &super_complex_test_output_that_could_change_at_any_time,
    // );

    // More options available; please check the docs.

    egress.close().unwrap().assert_unregressed();
}
