use std::{
	collections::{HashMap, HashSet},
	sync::Arc,
};

use anyhow::{anyhow, Result};
use clap::Parser;
use urlpattern::UrlPattern;

use crate::{
	_url_template::UrlTemplate,
	_util::{url_pattern, ClientAuthenticationMapping, RequestMapping},
	router::Router,
};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None, )]
pub struct Cli {
	/// Create a client for outbound requests. Ex. --client foo=*.example.com
	#[arg(long)]
	pub client: Vec<String>,
	/// Define the certificate a client should use for mTLS. Ex. --cert foo=/path/to/example.com.pem
	#[arg(long)]
	pub cert: Vec<String>,
	/// Define the key a client should use for mTLS. Ex. --key foo=/path/to/example.com.key
	#[arg(long)]
	pub key: Vec<String>,

	/// Define the request mappings. Ex ':9000/foo/:path*=https://example.com/{path}'
	pub mapping1: String,
	pub mapping: Vec<String>,
}

fn kv(field: &'static str, index: usize, value: &String) -> Result<(String, String)> {
	let Some(eq_index) = value.find('=') else {
		return Err(anyhow!(
			"Invalid key value pair '{value}' when reading {field}#{}",
			index + 1
		));
	};
	Ok((value[..eq_index].to_string(), value[eq_index + 1..].to_string()))
}

impl Cli {
	pub fn router_and_ports() -> Result<(Arc<Router>, Vec<u16>)> {
		let cli = Cli::parse();
		let mut ports = HashSet::<u16>::new();

		let client_authentication_mappings: Vec<ClientAuthenticationMapping> = {
			let mut builder = ClientAuthenticationMappingBuilders::new();

			// Find all client authentication mappings
			for index in 0..cli.client.len() {
				let (name, pattern) = kv("client", index, &cli.client[index])?;
				builder.add_pattern(name, pattern, index)?;
			}

			// Find all client authentication mappings
			for index in 0..cli.cert.len() {
				let (name, filename) = kv("cert", index, &cli.cert[index])?;
				builder.add_certificate(name, filename, index)?;
			}

			// Find all client key mappings
			for index in 0..cli.key.len() {
				let (name, filename) = kv("key", index, &cli.key[index])?;
				builder.add_key(name, filename, index)?;
			}

			builder.build()?
		};

		let mappings: Vec<RequestMapping> = {
			let mut tmp = Vec::new();
			let values = [vec![cli.mapping1], cli.mapping].concat();
			for index in 0..values.len() {
				let (key, value) = kv("mapping", index, &values[index])?;

				let (port, path) = {
					let (port, path) = if let Some(slash_index) = key.find('/') {
						(&key[1..slash_index], &key[slash_index + 1..])
					} else {
						(&key[1..], "*")
					};
					(port.parse::<u16>()?, path)
				};

				ports.insert(port);

				let pattern = match url_pattern(format!("*://*:{port}/{path}")) {
					Err(err) => return Err(anyhow!("Could not parse url pattern on mapping#{}: {err}", index + 1)),
					Ok(pattern) => pattern,
				};

				tmp.push(RequestMapping::new(pattern, UrlTemplate::parse(value.clone())?))
			}
			tmp
		};

		Ok((
			Arc::new(Router::new(client_authentication_mappings, mappings)),
			ports.into_iter().collect(),
		))
	}
}

struct ClientAuthenticationMappingBuilder {
	name: String,
	patterns: Vec<UrlPattern>,
	certificate_filename: Option<String>,
	key_filename: Option<String>,
}

impl ClientAuthenticationMappingBuilder {
	fn new(name: String, pattern: UrlPattern) -> Self {
		Self {
			name,
			patterns: vec![pattern],
			certificate_filename: None,
			key_filename: None,
		}
	}

	fn build(self) -> Result<ClientAuthenticationMapping> {
		let Some(certificate_filename) = self.certificate_filename else {
			return Err(anyhow!(
				"No certificate file defined for client authentication mapping '{}'",
				self.name
			));
		};

		let Some(key_filename) = self.key_filename else {
			return Err(anyhow!(
				"No key file defined for client authentication mapping '{}'",
				self.name
			));
		};

		match ClientAuthenticationMapping::new(self.name.clone(), self.patterns, certificate_filename, key_filename) {
			Err(err) => Err(anyhow!(
				"Could not build client authentication mapping '{}': {err}",
				self.name
			)),
			Ok(mapping) => Ok(mapping),
		}
	}

	fn add_pattern(&mut self, pattern: UrlPattern) {
		self.patterns.push(pattern);
	}

	fn add_certificate(&mut self, filename: String) -> Result<()> {
		if self.certificate_filename.is_some() {
			return Err(anyhow!(
				"Client authentication mapping '{}' already has a certificate defined",
				self.name
			));
		}
		self.certificate_filename = Some(filename);
		Ok(())
	}

	fn add_key(&mut self, filename: String) -> Result<()> {
		if self.key_filename.is_some() {
			return Err(anyhow!(
				"Client authentication mapping '{}' already has a key defined",
				self.name
			));
		}
		self.key_filename = Some(filename);
		Ok(())
	}
}

struct ClientAuthenticationMappingBuilders(HashMap<String, ClientAuthenticationMappingBuilder>);

impl ClientAuthenticationMappingBuilders {
	fn new() -> Self {
		Self(HashMap::new())
	}

	pub fn build(self) -> Result<Vec<ClientAuthenticationMapping>> {
		let mut tmp = Vec::new();
		for (_, value) in self.0 {
			tmp.push(value.build()?)
		}
		Ok(tmp)
	}

	pub fn add_pattern(&mut self, name: String, pattern: String, index: usize) -> Result<()> {
		match url_pattern(pattern) {
			Err(err) => Err(anyhow!("Could not parse url pattern on client#{} {err}", index + 1)),
			Ok(pattern) => Ok({
				if let Some(builder) = self.0.get_mut(&name) {
					builder.add_pattern(pattern);
				} else {
					self.0
						.insert(name.clone(), ClientAuthenticationMappingBuilder::new(name, pattern));
				}
			}),
		}
	}

	pub fn add_certificate(&mut self, name: String, filename: String, _index: usize) -> Result<()> {
		let Some(builder) = self.0.get_mut(&name) else {
			return Err(anyhow!(
				"Could not find client authentication mapping '{name}' when adding certificate '{filename}'",
			));
		};
		builder.add_certificate(filename)?;
		Ok(())
	}

	pub fn add_key(&mut self, name: String, filename: String, _index: usize) -> Result<()> {
		let Some(builder) = self.0.get_mut(&name) else {
			return Err(anyhow!(
				"Could not find client authentication mapping '{name}' when adding key '{filename}''",
			));
		};
		builder.add_key(filename)?;
		Ok(())
	}
}
