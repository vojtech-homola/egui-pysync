fn main() {
    // Also bounds the executable when invoked directly by Cargo integration tests.
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_secs(25));
        eprintln!("client-probe exceeded its hard timeout");
        std::process::exit(124);
    });
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 3, "Usage: client-probe PORT SCENARIO");
    egui_states_test_support::scenarios::run(args[1].parse().unwrap(), &args[2]);
    println!("scenario {} passed", args[2]);
}
