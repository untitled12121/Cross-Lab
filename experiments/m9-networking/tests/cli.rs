use std::process::Command as ProcessCommand;

#[test]
fn local_quinn_binary_emits_reproducible_tsv() {
    let binary = option_env!("CARGO_BIN_EXE_m9-networking").expect("m9-networking binary target");
    let output = ProcessCommand::new(binary)
        .args(["local-quinn", "--samples", "1", "--payload-bytes", "1024"])
        .output()
        .expect("run m9-networking binary");

    assert!(
        output.status.success(),
        "binary failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("UTF-8 benchmark output");
    assert!(stdout.lines().any(|line| line == "# samples=1"));
    assert!(stdout.lines().any(|line| line == "# payload_bytes=1024"));
    assert!(stdout.lines().any(|line| line.starts_with("# rss_kib=")));
    assert!(stdout.lines().any(|line| line.starts_with("# fd_count=")));
    assert!(stdout.contains("quinn\tprotected_connect_us\t0\t"));
    assert!(stdout.contains("quinn\tbulk_bytes_per_second\t0\t"));
}
