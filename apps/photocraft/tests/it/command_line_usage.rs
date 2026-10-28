//! `photocraft --help` and unknown options must not open the window (#6431): before the fix,
//! any argument the loop didn't know was taken as a file, so `--help` or a typo such as
//! `--safegpu` launched the GUI and logged "Couldn't open --help".
//!
//! These run the real binary, and every case must exit before the window opens: Windows and
//! macOS runners have a desktop, so a launch that gets that far hangs until the test times out.
//! Cases whose arguments are valid add a bad value before them: clap reports a bad value only
//! once the arguments after it read without an error, so stderr shows how they were taken.

/// The variables the command line reads: a test that wants one sets it.
const VARS: [&str; 1] = ["PHOTOCRAFT_IN_WINDOW_MENUS"];

/// A usage error reported only after every later argument is read.
const LATE_ERROR: [&str; 2] = ["--frame-latency", "9"];

/// How a run of `photocraft` ended.
struct Run {
    status: std::process::ExitStatus,
    stdout: String,
    stderr: String,
}

/// Run `photocraft` with `args` and `vars` in an isolated config dir.
fn run_with(args: &[&str], vars: &[(&str, &str)]) -> Run {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("photocraft-usage-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut cmd = std::process::Command::new(crate::binary());
    for var in VARS {
        cmd.env_remove(var);
    }
    let output = cmd
        .args(args)
        .envs(vars.iter().copied())
        .env("PHOTOCRAFT_CONFIG_DIR", &dir)
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("WAYLAND_SOCKET")
        .output()
        .expect("failed to run photocraft binary");
    std::fs::remove_dir_all(&dir).ok();
    Run { status: output.status, stdout: String::from_utf8_lossy(&output.stdout).into_owned(), stderr: String::from_utf8_lossy(&output.stderr).into_owned() }
}

fn run_photocraft(args: &[&str]) -> Run {
    run_with(args, &[])
}

/// [`LATE_ERROR`] then `args`: exit 2, and the stderr.
fn late_error(args: &[&str], vars: &[(&str, &str)]) -> String {
    let Run { status, stdout, stderr } = run_with(&[&LATE_ERROR[..], args].concat(), vars);
    assert_eq!(status.code(), Some(2), "{args:?}: {stderr}");
    assert!(stderr.contains("invalid value '9' for '--frame-latency <1-3>'"), "{args:?}: {stderr}");
    assert!(stdout.is_empty(), "{args:?}: {stdout}");
    stderr
}

#[test]
fn help_prints_usage_and_exits_0() {
    for flag in ["--help", "-h"] {
        let Run { status, stdout, stderr } = run_photocraft(&[flag]);
        assert_eq!(status.code(), Some(0), "{flag} must exit 0, stderr: {stderr}");
        assert!(stdout.contains("Usage: photocraft"), "{flag} stdout: {stdout}");
        for option in ["--safe-gpu", "--new-instance", "--frame-latency <1-3>", "--version", "`--`", "PHOTOCRAFT_NEW_INSTANCE"] {
            assert!(stdout.contains(option), "{flag} usage must list {option}: {stdout}");
        }
        assert!(!stderr.contains("Couldn't open"), "{flag} was taken as a file: {stderr}");
    }
}

#[test]
fn help_wins_over_later_arguments() {
    let Run { status, stdout, .. } = run_photocraft(&["--help", "picture.psd", "--safegpu"]);
    assert_eq!(status.code(), Some(0));
    assert!(stdout.contains("Usage: photocraft"), "stdout: {stdout}");
}

#[test]
fn version_prints_the_long_version() {
    let Run { status, stdout, .. } = run_photocraft(&["--version", "--safegpu"]);
    assert_eq!(status.code(), Some(0));
    assert_eq!(stdout, format!("photocraft {}\n", photocraft_engine::build_info::long_version()));
}

#[test]
fn unknown_options_exit_2_naming_them() {
    for (args, named) in [
        (&["--safegpu"][..], "unexpected argument '--safegpu'"),
        (&["-x"][..], "unexpected argument '-x'"),
        (&["--help=yes"][..], "unexpected value 'yes' for '--help'"),
        // An unknown flag after a bad value is reported first: clap drops the pending value's error.
        (&["--frame-latency", "nope", "--safegpu"][..], "unexpected argument '--safegpu'"),
        (&["--frame-latency", "nope"][..], "invalid value 'nope' for '--frame-latency <1-3>'"),
    ] {
        let Run { status, stdout, stderr } = run_photocraft(args);
        assert_eq!(status.code(), Some(2), "{args:?} must be a usage error, stderr: {stderr}");
        assert!(stderr.contains(named), "stderr: {stderr}");
        assert!(stderr.contains("--help"), "stderr should point at --help: {stderr}");
        assert!(stdout.is_empty(), "stdout: {stdout}");
    }
}

#[test]
fn arguments_after_double_dash_are_files() {
    // `--` ends the options: `--help` and `-x` after it are file names, so neither prints usage
    // nor is an unknown option; only the late error is reported.
    let stderr = late_error(&["--", "--help", "-x"], &[]);
    assert!(!stderr.contains("unexpected"), "stderr: {stderr}");
}

#[test]
fn macos_process_serial_number_is_still_ignored() {
    // Old macOS passes `-psn_<n>` when launching from Finder; it is neither a file nor an error.
    let stderr = late_error(&["-psn_0_12345"], &[]);
    assert!(!stderr.contains("unexpected"), "stderr: {stderr}");
}

#[test]
fn new_instance_is_a_known_option() {
    let stderr = late_error(&["--new-instance"], &[]);
    assert!(!stderr.contains("unexpected"), "stderr: {stderr}");
}
