use std::collections::HashMap;

use anyhow::Result;
use url::Url;

struct Variable {
	name: String,
	default_value: Option<String>,
	is_optional: bool,
	leading_slash: bool,
}

impl Variable {
	fn parse(mut value: String) -> Variable {
		let leading_slash = {
			if value.starts_with('/') {
				value = value[1..].to_string();
				true
			} else {
				false
			}
		};

		let is_optional = {
			if value.ends_with('?') {
				value = value[..value.len() - 1].to_string();
				true
			} else {
				false
			}
		};

		let default_value = {
			if let Some(default_value_start) = value.find(':') {
				let default_value = value[default_value_start + 1..].to_string();
				value = value[..default_value_start].to_string();
				Some(default_value)
			} else {
				None
			}
		};

		Self {
			name: value,
			default_value,
			is_optional,
			leading_slash,
		}
	}
}

enum Component {
	Literal(String),
	Variable(Variable),
}

pub struct UrlTemplate(Vec<Component>);

impl UrlTemplate {
	pub fn fill(&self, variables: HashMap<String, String>) -> Result<Url> {
		let mut buf = String::new();
		for component in self.0.iter() {
			match component {
				Component::Literal(literal) => buf.push_str(literal),
				Component::Variable(variable) => {
					let Some(value) = variables.get(&variable.name) else {
						if variable.is_optional {
							if let Some(default_value) = &variable.default_value {
								if variable.leading_slash {
									buf.push('/');
								}
								buf.push_str(default_value.as_str());
							}
							continue;
						} else {
							return Err(anyhow::anyhow!("Missing required variable {}", variable.name));
						}
					};
					if variable.leading_slash {
						buf.push('/');
					}
					buf.push_str(value.as_str());
				}
			}
		}
		Ok(Url::parse(buf.as_str())?)
	}

	pub fn parse(mut template: String) -> Result<Self> {
		let original_template = template.clone();
		let mut components: Vec<Component> = Vec::new();
		while !template.is_empty() {
			if let Some(variable_start) = template.find('{') {
				if variable_start > 0 {
					components.push(Component::Literal(template[..variable_start].to_string()));
				}
				if let Some(variable_end) = template.find('}') {
					let variable_content = template[variable_start + 1..variable_end].to_string();
					template = template[variable_end + 1..].to_string();
					components.push(Component::Variable(Variable::parse(variable_content)));
				} else {
					return Err(anyhow::anyhow!(
						"unexpected end of variable in template '{}'",
						original_template
					));
				}
			} else {
				components.push(Component::Literal(template));
				template = String::new();
			}
		}
		Ok(Self(components))
	}
}
