//! Identical client assertions for the generated Python and Rust servers.
use egui_states::{Client, ClientBuilder, ConnectionState};
use egui_states_test_schema::{State, integration::IntegrationState};
use std::time::{Duration, Instant};

pub const SCENARIOS: &[&str] = &[
    "sync",
    "takes",
    "blocking-value",
    "blocking-empty",
    "blocking-data",
    "blocking-multi",
    "cache",
];

pub fn wait<T>(label: &str, mut condition: impl FnMut() -> Option<T>) -> T {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(value) = condition() {
            return value;
        }
        assert!(Instant::now() < deadline, "timed out: {label}");
        std::thread::sleep(Duration::from_millis(5));
    }
}

pub struct Connection(pub Client);
impl Drop for Connection {
    fn drop(&mut self) {
        self.0.disconnect();
        let deadline = Instant::now() + Duration::from_secs(2);
        while self.0.get_state() == ConnectionState::Connected && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

fn connect(port: u16) -> (State, Connection) {
    let builder = ClientBuilder::<State>::new();
    let hash = builder.get_version_hash();
    let (state, client) = builder.version(hash).build(port);
    let connection = Connection(client);
    wait("client connection", || {
        connection.0.connect();
        (connection.0.get_state() == ConnectionState::Connected).then_some(())
    });
    wait("initial synchronized value", || {
        (state.integration.value.get() == 7).then_some(())
    });
    (state, connection)
}

fn once<T: PartialEq + std::fmt::Debug>(
    label: &str,
    expected: T,
    mut take: impl FnMut() -> Option<T>,
) {
    assert_eq!(wait(label, &mut take), expected, "{label}");
    assert_eq!(take(), None, "{label} consumed more than once");
}

fn phase(s: &IntegrationState, n: u32) {
    wait("server phase", || (s.phase.get() == n).then_some(()));
}

fn barrier(s: &IntegrationState) {
    s.command.set(99u32);
    phase(s, 99);
}

pub fn run(port: u16, scenario: &str) {
    let (state, connection) = connect(port);
    let s = &state.integration;
    match scenario {
        "sync" => {
            wait("initial collection", || {
                s.items.read(|v| (v == &[1, 2]).then_some(()))
            });
            s.command.set(1u32);
            wait("server update", || (s.value.get() == 21).then_some(()));
            s.value.set_signal(42);
            wait("value callback", || {
                (s.callback_value.get() == 42).then_some(())
            });
            s.command.set(2u32);
            wait("collection callback", || {
                s.items.read(|v| (v == &[1, 2, 42]).then_some(()))
            });
            wait("map callback", || {
                s.map.read(|v| (v.get(&9) == Some(&900)).then_some(()))
            });
        }
        "takes" => {
            s.command.set(3u32);
            once("value payload", String::from("payload"), || s.take.take());
            once("unit payload", (), || s.empty.take());
            once("data payload", vec![0, 1, 255], || s.data.take());
            once("sparse multi payload", vec![100, 200], || s.multi.take(7));
            once("empty sparse payload", Vec::<u16>::new(), || {
                s.multi.take(42)
            });
            assert!(s.multi.take(0).is_none());
            s.command.set(4u32);
            once("empty string", String::new(), || s.take.take());
            once("empty data", Vec::<u8>::new(), || s.data.take());
            barrier(s);
            assert!(s.take.take().is_none());
            assert!(s.empty.take().is_none());
            assert!(s.data.take().is_none());
            assert!(s.multi.take(7).is_none());
            assert!(s.multi.take(42).is_none());
        }
        "blocking-value" | "blocking-empty" | "blocking-data" | "blocking-multi" => {
            let command: u32 = match scenario {
                "blocking-value" => 10,
                "blocking-empty" => 11,
                "blocking-data" => 12,
                _ => 13,
            };
            s.command.set(command);
            phase(s, 1);
            // A negative assertion over a bounded interval proves the subsequent
            // send remains pending while the first payload is unconsumed.
            let until = Instant::now() + Duration::from_millis(150);
            while Instant::now() < until {
                assert_eq!(s.phase.get(), 1, "send completed before consumption");
                std::thread::sleep(Duration::from_millis(5));
            }
            match command {
                10 => assert_eq!(wait("first value", || s.take.take()), "first"),
                11 => {
                    wait("first unit", || s.empty.take());
                }
                12 => assert_eq!(wait("first data", || s.data.take()), vec![1, 2]),
                _ => assert_eq!(wait("first multi", || s.multi.take(73)), vec![100, 200]),
            }
            phase(s, 2);
            match command {
                10 => once("second value", String::new(), || s.take.take()),
                11 => once("second unit", (), || s.empty.take()),
                12 => once("second data", Vec::<u8>::new(), || s.data.take()),
                _ => once("second multi", Vec::<u16>::new(), || s.multi.take(73)),
            }
            barrier(s);
        }
        "cache" => {
            once("cached data", vec![8, 9], || s.cached.take());
            once("cached sparse data", vec![500, 600], || {
                s.cached_multi.take(91)
            });
            drop(connection);
            let (next, _connection) = connect(port);
            once("cached data for new client", vec![8, 9], || {
                next.integration.cached.take()
            });
            once("cached sparse data for new client", vec![500, 600], || {
                next.integration.cached_multi.take(91)
            });
        }
        _ => panic!("unknown scenario: {scenario}"),
    }
}
