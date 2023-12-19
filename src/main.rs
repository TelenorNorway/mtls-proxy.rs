use std::sync::Arc;

use anyhow::Result;
use http_mtls_proxy::{create_server, Cli};
use log::{error, info, trace};
use tokio::task::spawn;

#[tokio::main]
async fn main() -> Result<()> {
	env_logger::init();
	trace!("Building router and ports");
	let (router, ports) = Cli::router_and_ports()?;

	let mut handles = Vec::new();

	for port in &ports {
		let router = Arc::clone(&router);
		let port = port.clone();
		trace!("Spawning server on port {port}");
		handles.push(spawn(async move {
			if let Err(err) = create_server(port, router).await {
				error!("Error on port {port}: {err}");
			}
		}))
	}

	info!(
		"Listening on ports: {}",
		ports.iter().map(|port| port.to_string()).collect::<Vec<_>>().join(", ")
	);

	for handle in handles {
		handle.await?;
	}

	Ok(())
}
