mod happy_path {
    /// Test flow in `SevenZip` scenarios
    #[test]
    fn e2e_sevenzip() {
        let mut cmd = assert_cmd::Command::cargo_bin("shimiko").expect("binary does not exist ");
        let assert = cmd.args(["1", "1", "test_artifacts"]).assert();
        assert.success();
    }

    /// Test flow in `Zip` scenarios
    #[test]
    fn e2e_zip() {
        let mut cmd = assert_cmd::Command::cargo_bin("shimiko").expect("binary does not exist ");
        let assert = cmd.args(["1318", "1318", "test_artifacts"]).assert();
        assert.success();
    }

    /// Test flow in multiple archive scenarios
    #[test]
    fn e2e_multiple_archive() {
        let mut cmd = assert_cmd::Command::cargo_bin("shimiko").expect("binary does not exist ");
        let assert = cmd.args(["1299", "1300", "test_artifacts"]).assert();
        assert.success();
    }
}

mod sad_path {
    /// Test 0 range input validation
    #[test]
    fn zero_pack_range() {
        let mut cmd = assert_cmd::Command::cargo_bin("shimiko").expect("binary does not exist ");
        let assert = cmd.args(["0", "0", "test_artifacts"]).assert();
        assert.failure();
    }
}
