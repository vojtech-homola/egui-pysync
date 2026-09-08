fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let port = match args.next().as_deref() {
        None => 8092,
        Some("--port") => args
            .next()
            .ok_or("--port requires a number")?
            .parse::<u16>()?,
        _ => return Err("Usage: counter-server [--port PORT]".into()),
    };
    let demo = counter_server::setup_server()?;
    demo.server
        .start(port, Some(std::net::Ipv4Addr::LOCALHOST), None)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        // Register synchronously so Ctrl+C is handled as soon as readiness is printed.
        #[cfg(unix)]
        let mut interrupt =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;
        #[cfg(windows)]
        let mut interrupt = tokio::signal::windows::ctrl_c()?;

        println!("Counter server listening on {port}");
        interrupt.recv().await;
        Ok::<_, std::io::Error>(())
    })?;
    demo.server.stop();
    Ok(())
}
