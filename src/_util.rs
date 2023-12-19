use crate::{_url_template::UrlTemplate, tls::create_config};
use anyhow::Result;
use regex::Regex;
use rustls::ClientConfig;
use std::{collections::HashMap, sync::Arc};
use url::Url;
use urlpattern::{UrlPattern, UrlPatternInit, UrlPatternMatchInput};

pub struct ClientAuthenticationMapping {
	pub name: String,
	patterns: Vec<UrlPattern>,
	tls_config: Arc<ClientConfig>,
}

impl ClientAuthenticationMapping {
	pub fn new(
		name: String,
		patterns: Vec<UrlPattern>,
		certificate_filename: String,
		key_filename: String,
	) -> Result<Self> {
		Ok(Self {
			name,
			patterns,
			tls_config: create_config(certificate_filename.as_str(), key_filename.as_str())?,
		})
	}

	pub fn config(&self) -> Arc<ClientConfig> {
		Arc::clone(&self.tls_config)
	}

	pub fn test(&self, input: &Url) -> Result<bool> {
		for pattern in &self.patterns {
			if pattern.test(urlpattern::UrlPatternMatchInput::Url(input.clone()))? {
				return Ok(true);
			}
		}
		Ok(false)
	}
}

pub struct RequestMapping {
	pattern: UrlPattern,
	destination_template: UrlTemplate,
}

impl RequestMapping {
	pub fn new(pattern: UrlPattern, destination_template: UrlTemplate) -> Self {
		Self {
			pattern,
			destination_template,
		}
	}

	pub fn test(&self, input: &Url) -> Result<bool> {
		Ok(self.pattern.test(UrlPatternMatchInput::Url(input.clone()))?)
	}

	pub fn destination(&self, input: &Url) -> Result<Url> {
		let captures = self
			.pattern
			.exec(UrlPatternMatchInput::Url(input.clone()))?
			.ok_or_else(|| anyhow::anyhow!("Could not extract variables from input url {} using pattern", input))?;

		let mut variables: HashMap<String, String> = HashMap::new();

		for (key, value) in captures.pathname.groups {
			variables.insert(key, value);
		}

		for (key, value) in captures.search.groups {
			variables.insert(key, value);
		}

		Ok(self.destination_template.fill(variables)?)
	}
}

pub fn url_pattern(input: String) -> Result<UrlPattern> {
	Ok(UrlPattern::<Regex>::parse(UrlPatternInit::parse_constructor_string::<
		Regex,
	>(input.as_str(), None)?)?)
}
