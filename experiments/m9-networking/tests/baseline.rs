use crosslab_m9_networking::{
    Command,
    config::EvalConfig,
    metrics::{MetricKind, TransportKind},
};

#[tokio::test]
async fn quinn_baseline_records_connect_auth_control_bulk_and_shutdown() {
    let report = crosslab_m9_networking::baseline::run_quinn_loopback_sample(EvalConfig::test())
        .await
        .expect("Quinn baseline should complete");

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
        assert!(
            report.contains_transport(transport),
            "missing {transport:?}"
        );
    }
}

#[tokio::test]
async fn local_benchmark_records_platform_resource_observations() {
    let config = EvalConfig::with_limits(1, 64 * 1024).expect("valid local benchmark limits");
    let report = crosslab_m9_networking::baseline::run_local(Command::LocalQuinn(config))
        .await
        .expect("local Quinn benchmark should complete");
    let output = report.to_tsv();
    let rss_kib = header_value(&output, "rss_kib").expect("RSS observation header");
    let fd_count = header_value(&output, "fd_count").expect("FD observation header");

    #[cfg(target_os = "linux")]
    {
        assert!(rss_kib.parse::<u64>().expect("numeric Linux RSS") > 0);
        assert!(fd_count.parse::<usize>().expect("numeric Linux FD count") > 0);
    }

    #[cfg(not(target_os = "linux"))]
    {
        assert_eq!(rss_kib, "-");
        assert_eq!(fd_count, "-");
    }
}

fn header_value<'a>(output: &'a str, name: &str) -> Option<&'a str> {
    output
        .lines()
        .find_map(|line| line.strip_prefix(&format!("# {name}=")))
}
