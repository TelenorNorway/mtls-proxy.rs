use std::sync::Arc;

use anyhow::{anyhow, Result};
use hyper::{body::Incoming, client::conn::http1::Builder, Request, Response};
use hyper_util::rt::TokioIo;
use log::{error, trace};
use rustls::ClientConfig;
use tokio::{net::TcpStream, task::spawn};
use tokio_rustls::client::TlsStream;

use crate::tls::connect;

pub async fn http_client_send(tls_config: Arc<ClientConfig>, request: Request<Incoming>) -> Result<Response<Incoming>> {
	let host = request.uri().host().ok_or(anyhow!("Missing host in URI"))?;
	let port = request.uri().port_u16().unwrap_or(443);

	trace!("Connecting to {host}:{port}");

	let (mut sender, conn) = Builder::new()
		.handshake::<TokioIo<TlsStream<TcpStream>>, Incoming>(TokioIo::new(
			connect(tls_config, format!("{host}:{port}"), host).await?,
		))
		.await?;

	spawn(async move {
		if let Err(err) = conn.await {
			error!("connection error: {err}");
		}
	});

	Ok(sender.send_request(request).await?)
}
