//! Whether a baseline suite's run counts as filtered, for every libtest filter form.

mod common;

use common::baseline::filtered_by;

fn filtered(args: &[&str]) -> bool {
    filtered_by(args.iter().map(|arg| arg.to_string()), None)
}

#[test]
fn every_libtest_filter_form_marks_the_run_filtered() {
    for args in [
        &["editor"][..],
        &["--skip", "editor"],
        &["--skip=editor"],
        &["--exact", "editor"],
        &["--ignored"],
        &["--exclude-should-panic"],
        &["--bench"],
        &["--list"],
        &["--test-threads", "2", "editor"],
        &["--test-threads=2", "editor"],
        &["--", "--skip"],
    ] {
        assert!(filtered(args), "{args:?}");
    }
    assert!(filtered_by(std::iter::empty(), Some("case")));
}

#[test]
fn options_and_their_values_leave_the_run_whole() {
    for args in [
        &[][..],
        &["--test-threads", "2"],
        &["--test-threads=2"],
        &["--color", "never", "--format", "terse"],
        &["--format=json", "-Z", "unstable-options"],
        &["-Zunstable-options", "--shuffle-seed", "7"],
        &[
            "--logfile",
            "run.log",
            "--nocapture",
            "-q",
            "--include-ignored",
        ],
        &["--"],
    ] {
        assert!(!filtered(args), "{args:?}");
    }
    assert!(!filtered_by(std::iter::empty(), Some("")));
}
