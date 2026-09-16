use std::env;

use crosslab_m9_networking::{
    Command,
    config::EvalConfig,
    metrics::{Measurement, MetricKind, Report, TransportKind},
};

#[test]
fn local_command_parses_typed_sample_and_payload_limits() {
    let command = Command::parse([
        "local-all",
        "--samples",
        "3",
        "--payload-bytes",
        "1048576",
    ])
    .expect("typed local command");
    let config = command.eval_config().expect("local evaluation config");

    assert_eq!(config.samples(), 3);
    assert_eq!(config.bulk_payload_bytes(), 1_048_576);
}

#[test]
fn report_tsv_records_reproducibility_header() {
    let config = EvalConfig::test();
    let report = Report::for_run(
        config,
        vec![Measurement::new(
            TransportKind::Quinn,
            MetricKind::ProtectedConnectMicros,
            0,
            42,
        )],
    );
    let output = report.to_tsv();

    for expected in [
        format!("# os={}", env::consts::OS),
        format!("# arch={}", env::consts::ARCH),
        "# rust=1.98.1".to_owned(),
        format!("# samples={}", config.samples()),
        format!("# payload_bytes={}", config.bulk_payload_bytes()),
        "# quinn=0.11.11".to_owned(),
        "# iroh=1.2.0".to_owned(),
    ] {
        assert!(output.lines().any(|line| line == expected), "missing {expected}");
    }
    assert!(output.contains("transport\tmetric\tsample\tvalue\n"));
    assert!(output.contains("quinn\tprotected_connect_us\t0\t42\n"));
}
