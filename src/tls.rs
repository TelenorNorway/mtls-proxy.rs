use anyhow::{anyhow, Result};
use log::{debug, trace};
use rustls::{ClientConfig, RootCertStore};
use rustls_pemfile::{certs as read_certs, read_one, Item};
use rustls_pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use std::{fs::File, io::BufReader, sync::Arc};
use tokio::net::TcpStream;
use tokio_rustls::{client::TlsStream, TlsConnector};
use webpki_roots::TLS_SERVER_ROOTS;

fn load_certs(filename: &str) -> Result<Vec<CertificateDer<'static>>> {
	let certfile = File::open(filename)?;
	let mut reader = BufReader::new(certfile);
	let mut certs = Vec::new();
	for cert in read_certs(&mut reader) {
		certs.push(cert?);
	}
	debug!("Found {} {filename}", certs.len());
	Ok(certs)
}

fn load_key(filename: &str) -> Result<PrivateKeyDer<'static>> {
	let keyfile = File::open(filename)?;
	let mut reader = BufReader::new(keyfile);

	loop {
		return Ok(
			match read_one(&mut reader).map_err(|_| anyhow!("cannot parse private key file"))? {
				None => return Err(anyhow!("no keys found in {filename} file")),
				Some(tmp) => {
					debug!("Scanning for key in {filename}: {tmp:?}");
					match tmp {
						Item::Pkcs1Key(key) => key.into(),
						Item::Pkcs8Key(key) => key.into(),
						Item::Sec1Key(key) => key.into(),
						Item::X509Certificate(_) | Item::Crl(_) | _ => continue,
					}
				}
			},
		);
	}
}

pub fn create_config(certificate_filename: &str, key_filename: &str) -> Result<Arc<ClientConfig>> {
	Ok(Arc::new(
		ClientConfig::builder()
			.with_root_certificates({
				let mut root_cert_store = RootCertStore::empty();
				root_cert_store.extend(TLS_SERVER_ROOTS.iter().cloned());
				root_cert_store
			})
			.with_client_auth_cert(load_certs(certificate_filename)?, load_key(key_filename)?)?,
	))
}

pub async fn connect(
	config: Arc<ClientConfig>,
	host: impl Into<String>,
	sni: impl Into<String>,
) -> Result<TlsStream<TcpStream>> {
	let host = host.into();
	let sni = sni.into();
	trace!("Connecting to {host} with SNI {sni}");
	Ok(TlsConnector::from(config)
		.connect(ServerName::try_from(sni)?, TcpStream::connect(host).await?)
		.await?)
}
