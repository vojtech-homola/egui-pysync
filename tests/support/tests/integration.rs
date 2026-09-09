use egui_states_test_support::{fixture::Fixture, free_port, scenarios::SCENARIOS};

#[test]
fn generated_rust_server_scenarios() {
    for scenario in SCENARIOS {
        let port = free_port();
        let fixture = Fixture::new(port).unwrap();
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_client-probe"))
            .arg(port.to_string())
            .arg(scenario)
            .output()
            .unwrap();
        fixture.server.stop();
        fixture.assert_no_errors();
        assert!(
            output.status.success(),
            "{scenario}: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
