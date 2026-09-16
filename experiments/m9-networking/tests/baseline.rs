use crosslab_m9_networking::{
    Command,
    config::EvalConfig,
    metrics::{MetricKind, TransportKind},
};

#[tokio::test]
async fn quinn_baseline_records_connect_control_bulk_and_shutdown() {
    let report = crosslab_m9_networking::baseline::run_quinn_loopback_sample(EvalConfig::test())
        .await
        .expect("Quinn baseline should complete");

    for metric in [
        MetricKind::ProtectedConnectMicros,
        MetricKind::ControlRttMicros,
        MetricKind::BulkBytesPerSecond,
        MetricKind::ShutdownMicros,
    ] {
        assert!(report.contains(metric), "missing {metric:?}");
    }
}

#[tokio::test]
async fn iroh_direct_records_connect_auth_control_bulk_and_shutdown() {
    let config = EvalConfig::with_limits(1, 64 * 1024).expect("valid diagnostic limits");
    let report = crosslab_m9_networking::baseline::run_iroh_direct_sample(config)
        .await
        .expect("Iroh direct benchmark should complete");
    eprintln!("direct benchmark diagnostics: {report:?}");

    for metric in [
        MetricKind::ProtectedConnectMicros,
        MetricKind::SessionAuthMicros,
        MetricKind::ControlRttMicros,
        MetricKind::BulkBytesPerSecond,
        MetricKind::ShutdownMicros,
    ] {
        assert!(report.contains(metric), "missing {metric:?}");
    }
}

#[tokio::test]
async fn local_all_combines_quinn_direct_and_relay_reports() {
    let config = EvalConfig::with_limits(1, 64 * 1024).expect("valid local benchmark limits");
    let report = crosslab_m9_networking::baseline::run_local(Command::LocalAll(config))
        .await
        .expect("local-all benchmark should complete");

    for transport in [
        TransportKind::Quinn,
        TransportKind::IrohDirect,
        TransportKind::IrohRelay,
    ] {
        assert!(report.contains_transport(transport), "missing {transport:?}");
    }
}
