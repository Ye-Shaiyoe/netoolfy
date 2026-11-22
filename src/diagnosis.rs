use crate::model::{CheckResult, Status, Verdict};

pub fn diagnose(results: &[CheckResult]) -> Verdict {
    if results.is_empty() {
        return Verdict::new(Status::Ok, "No checks were run.");
    }

    let overall = results
        .iter()
        .map(|r| r.status)
        .filter(|s| *s != Status::Skipped)
        .max()
        .unwrap_or(Status::Ok);

    if overall == Status::Ok {
        return Verdict::new(Status::Ok, "Your connection is healthy.");
    }

    let root_cause = results
        .iter()
        .filter(|r| r.status == Status::Fail || r.status == Status::Warn)
        .min_by_key(|r| r.layer);

    match root_cause {
        Some(rc) => {
            let summary = build_summary(rc, results);
            let fix = extract_first_fix(rc);
            let mut verdict = Verdict::new(overall, summary);
            if let Some(f) = fix {
                verdict = verdict.with_fix(f);
            }
            verdict
        }
        None => Verdict::new(Status::Ok, "Your connection is healthy."),
    }
}

fn build_summary(root: &CheckResult, all_results: &[CheckResult]) -> String {
    let skipped_count = all_results
        .iter()
        .filter(|r| r.status == Status::Skipped)
        .count();

    let mut summary = String::new();
    for finding in &root.findings {
        if !summary.is_empty() {
            summary.push(' ');
        }
        summary.push_str(&finding.message);
    }

    if summary.is_empty() {
        summary = format!("{} check reported status: {}", root.name, root.status);
    }

    if skipped_count > 0 {
        summary.push_str(&format!(
            " ({skipped_count} higher-layer check(s) skipped due to this failure)"
        ));
    }

    summary
}

fn extract_first_fix(result: &CheckResult) -> Option<String> {
    result.findings.iter().find_map(|f| f.fix.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Finding, FindingCode, Layer};

    fn ok_result(layer: Layer, name: &str) -> CheckResult {
        CheckResult::new(layer, name, Status::Ok).with_finding(Finding::new(
            FindingCode::AllGood,
            format!("{name} is healthy"),
        ))
    }

    fn fail_result(layer: Layer, name: &str, code: FindingCode, msg: &str) -> CheckResult {
        CheckResult::new(layer, name, Status::Fail).with_finding(Finding::new(code, msg))
    }

    fn warn_result(layer: Layer, name: &str, code: FindingCode, msg: &str) -> CheckResult {
        CheckResult::new(layer, name, Status::Warn).with_finding(Finding::new(code, msg))
    }

    fn skipped_result(layer: Layer, name: &str) -> CheckResult {
        CheckResult::skipped(layer, name, "Lower layer failed")
    }

    #[test]
    fn reports_healthy_when_all_checks_ok() {
        let results = vec![
            ok_result(Layer::Interface, "Interface"),
            ok_result(Layer::Gateway, "Gateway"),
            ok_result(Layer::Dns, "DNS"),
        ];
        let verdict = diagnose(&results);

        assert_eq!(verdict.overall_status, Status::Ok);
        assert!(verdict.summary.contains("healthy"));
    }

    #[test]
    fn handles_empty_check_results() {
        let verdict = diagnose(&[]);

        assert_eq!(verdict.overall_status, Status::Ok);
        assert!(verdict.summary.contains("No checks were run"));
    }

    #[test]
    fn identifies_gateway_failure_as_root_cause() {
        let results = vec![
            ok_result(Layer::Interface, "Interface"),
            fail_result(
                Layer::Gateway,
                "Gateway",
                FindingCode::GatewayUnreachable,
                "Gateway 192.168.1.1 is unreachable",
            ),
            skipped_result(Layer::Dns, "DNS"),
            skipped_result(Layer::Internet, "Internet"),
        ];
        let verdict = diagnose(&results);

        assert_eq!(verdict.overall_status, Status::Fail);
        assert!(verdict.summary.contains("Gateway"));
        assert!(verdict.summary.contains("unreachable"));
        assert!(verdict.summary.contains("skipped"));
    }

    #[test]
    fn flags_dns_latency_as_warning() {
        let results = vec![
            ok_result(Layer::Interface, "Interface"),
            ok_result(Layer::Gateway, "Gateway"),
            warn_result(
                Layer::Dns,
                "DNS",
                FindingCode::DnsSlow,
                "ISP DNS resolver is slow (240 ms)",
            ),
            ok_result(Layer::Internet, "Internet"),
        ];
        let verdict = diagnose(&results);

        assert_eq!(verdict.overall_status, Status::Warn);
        assert!(verdict.summary.contains("DNS"));
    }

    #[test]
    fn includes_recommended_fix_from_root_cause() {
        let finding = Finding::new(FindingCode::DnsSlow, "DNS is slow")
            .with_fix("Switch your DNS to 1.1.1.1 or 8.8.8.8");
        let results = vec![
            ok_result(Layer::Interface, "Interface"),
            ok_result(Layer::Gateway, "Gateway"),
            CheckResult::new(Layer::Dns, "DNS", Status::Warn).with_finding(finding),
        ];
        let verdict = diagnose(&results);

        assert_eq!(
            verdict.fix.as_deref(),
            Some("Switch your DNS to 1.1.1.1 or 8.8.8.8")
        );
    }

    #[test]
    fn prioritizes_interface_failure_over_dns() {
        let results = vec![
            fail_result(
                Layer::Interface,
                "Interface",
                FindingCode::NoInterface,
                "No active network interface found",
            ),
            skipped_result(Layer::Gateway, "Gateway"),
            skipped_result(Layer::Dns, "DNS"),
        ];
        let verdict = diagnose(&results);

        assert_eq!(verdict.overall_status, Status::Fail);
        assert!(verdict.summary.contains("No active network interface"));
    }

    #[test]
    fn picks_lowest_layer_as_root_cause_among_warnings() {
        let results = vec![
            ok_result(Layer::Interface, "Interface"),
            warn_result(
                Layer::Gateway,
                "Gateway",
                FindingCode::GatewayLatencyHigh,
                "Gateway latency is 50 ms",
            ),
            warn_result(Layer::Dns, "DNS", FindingCode::DnsSlow, "DNS is slow"),
        ];
        let verdict = diagnose(&results);

        assert_eq!(verdict.overall_status, Status::Warn);
        assert!(verdict.summary.contains("Gateway"));
    }
}
