use std::sync::Arc;

use anyhow::Result;
use rustls::ClientConfig;
use url::Url;

use crate::_util::{ClientAuthenticationMapping, RequestMapping};

pub struct Router {
	client_auth: Vec<ClientAuthenticationMapping>,
	request_mapping: Vec<RequestMapping>,
}

impl Router {
	pub fn new(client_auth: Vec<ClientAuthenticationMapping>, request_mapping: Vec<RequestMapping>) -> Self {
		Self {
			client_auth,
			request_mapping,
		}
	}

	fn destination_for(&self, url: &Url) -> Result<Option<Url>> {
		for mapping in &self.request_mapping {
			if mapping.test(url)? {
				return Ok(Some(mapping.destination(url)?));
			}
		}
		Ok(None)
	}

	fn client_auth_for_destination(&self, url: &Url) -> Result<Option<Arc<ClientConfig>>> {
		for client_auth in &self.client_auth {
			if client_auth.test(url)? {
				return Ok(Some(client_auth.config()));
			}
		}
		Ok(None)
	}

	pub fn get_destination(&self, url: &Url) -> Result<(Option<Arc<ClientConfig>>, Option<Url>)> {
		let destination = self.destination_for(url)?;
		if let Some(destination) = destination {
			Ok((self.client_auth_for_destination(&destination)?, Some(destination)))
		} else {
			Ok((None, None))
		}
	}
}
