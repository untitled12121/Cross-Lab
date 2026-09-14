use crosslab_m9_networking::metrics::{Measurement, MetricKind, Report, TransportKind};

#[test]
fn report_tsv_schema_is_stable() {
    let report = Report::new(vec![Measurement::new(
        TransportKind::Quinn,
        MetricKind::ProtectedConnectMicros,
        0,
        42,
    )]);

    assert_eq!(
        report.to_tsv(),
        "transport\tmetric\tsample\tvalue\nquinn\tprotected_connect_us\t0\t42\n"
    );
}
