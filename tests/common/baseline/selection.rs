pub fn filtered_by(args: impl IntoIterator<Item = String>, tt_cases: Option<&str>) -> bool {
    if tt_cases.is_some_and(|value| !value.is_empty()) {
        return true;
    }
    const SELECTING: [&str; 4] = ["--ignored", "--exclude-should-panic", "--bench", "--list"];
    const VALUED: [&str; 6] = [
        "--logfile",
        "--test-threads",
        "--color",
        "--format",
        "--shuffle-seed",
        "-Z",
    ];
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == "--" {
            return args.next().is_some();
        }
        let (name, value) = match arg.split_once('=') {
            Some((name, value)) if name.starts_with("--") => (name, Some(value)),
            _ => (arg.as_str(), None),
        };
        if name == "--skip" || SELECTING.contains(&name) {
            return true;
        }
        if VALUED.contains(&name) {
            if value.is_none() {
                args.next();
            }
            continue;
        }
        if !arg.starts_with('-') {
            return true;
        }
    }
    false
}
