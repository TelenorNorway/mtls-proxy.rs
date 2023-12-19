use crate::{http_client::http_client_send, router::Router};
use anyhow::{anyhow, Result};
use hyper::{
	body::{Body, Incoming},
	server::conn::http1::Builder,
	service::Service,
	Request, Response,
};
use hyper_util::rt::TokioIo;
use log::{error, info, trace};
use std::{future::Future, marker::PhantomData, net::SocketAddr, sync::Arc};
use tokio::{net::TcpListener, task::spawn};
use url::Url;

pub async fn create_server(port: u16, router: Arc<Router>) -> Result<()> {
	let listener = TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], port))).await?;

	loop {
		let (stream, _) = listener.accept().await?;
		let io = TokioIo::new(stream);
		let port = port.clone();
		let router = Arc::clone(&router);

		spawn(async move {
			let service = handler_service(handle, port, router);
			if let Err(err) = Builder::new().serve_connection(io, service).await {
				println!("error when serving request {}", err);
			}
		});
	}
}

struct HandlerService<F, R> {
	port: u16,
	router: Arc<Router>,
	f: F,
	_phantom: PhantomData<R>,
}

fn handler_service<F, R, S>(hnd: F, port: u16, router: Arc<Router>) -> HandlerService<F, R>
where
	F: Fn(u16, Arc<Router>, Request<R>) -> S,
	S: Future,
{
	HandlerService {
		port,
		router,
		f: hnd,
		_phantom: PhantomData,
	}
}

impl<F, ReqBody, Ret, ResBody, E> Service<Request<ReqBody>> for HandlerService<F, ReqBody>
where
	F: Fn(u16, Arc<Router>, Request<ReqBody>) -> Ret,
	ReqBody: Body,
	Ret: Future<Output = Result<Response<ResBody>, E>>,
	E: Into<Box<dyn std::error::Error + Send + Sync>>,
	ResBody: Body,
{
	type Response = Response<ResBody>;
	type Error = E;
	type Future = Ret;

	fn call(&self, req: Request<ReqBody>) -> Self::Future {
		(self.f)(self.port, Arc::clone(&self.router), req)
	}
}

async fn handle(port: u16, router: Arc<Router>, request: Request<Incoming>) -> Result<Response<Incoming>> {
	trace!("Getting url");
	let url = Url::parse(format!("http://localhost:{port}{}", request.uri().to_string()).as_str())?;
	trace!("Getting method");
	let method = request.method().as_str();
	trace!("Getting pathname");
	let pathname = request.uri().path();
	trace!("Getting client_config and destination");
	let (client_config, destination) = router.get_destination(&url)?;
	let Some(client_config) = client_config else {
		error!("No client config found for request");
		return Err(anyhow!("No client config found for request"));
	};
	let Some(destination) = destination else {
		error!("No destination found for request");
		return Err(anyhow!("No destination found for request"));
	};

	// Used for debugging
	let incoming_at = format!("{method} :{port}{pathname}");

	trace!("Building outgoing request");
	let mut builder = Request::builder().uri(destination.to_string());

	trace!("Copying headers");
	for (key, value) in request.headers() {
		if key.to_string().to_lowercase().as_str() == "host" {
			continue;
		}
		trace!("Header {key} = {value:?}");
		builder = builder.header(key, value);
	}

	trace!("Setting host header = {:?}", destination.host_str());
	builder = builder.header(
		"host",
		destination.host_str().ok_or(anyhow!("Missing host in destination"))?,
	);
	trace!("Headers = {:#?}", builder.headers_ref());

	trace!("Sending and getting response");
	let response = match http_client_send(
		client_config,
		match builder.body(request.into_body()) {
			Err(err) => {
				trace!("Error building request: {err}");
				return Err(anyhow!("Could not build request: {err}"));
			}
			Ok(req) => req,
		},
	)
	.await
	{
		Err(err) => {
			trace!("Error sending or receiving request: {err}");
			return Err(anyhow!("Could not send request: {err}"));
		}
		Ok(res) => res,
	};

	info!("{incoming_at} -> {destination} = {}", response.status());

	Ok(response)
}
