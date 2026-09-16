use crosslab_m9_networking::{config::EvalConfig, metrics::MetricKind};

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
    let report = crosslab_m9_networking::baseline::run_iroh_direct_sample(EvalConfig::test())
        .await
        .expect("Iroh direct benchmark should complete");

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
