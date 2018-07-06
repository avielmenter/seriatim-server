use std::env;
use std::fmt;

pub struct SeriatimConfig {
	pub domain: String,
	pub twitter_key: String,
	pub twitter_secret: String,
	pub allowed_origin: String,
	pub database_url: String,
}

impl SeriatimConfig {
	pub fn init() -> SeriatimConfig {
		SeriatimConfig {
			domain: env::var("seriatim_domain").unwrap(),
			twitter_key: env::var("seriatim_twitter_key").unwrap(),
			twitter_secret: env::var("seriatim_twitter_secret").unwrap(),
			allowed_origin: env::var("seriatim_allowed_origin").unwrap(),
			database_url: env::var("database_url").unwrap(),
		}
	}
}

impl fmt::Display for SeriatimConfig {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		writeln!(f, "    => seriatim_domain:         {}", self.domain)?;
		writeln!(f, "    => seriatim_twitter_key:    set")?;
		writeln!(f, "    => seriatim_twitter_secret: set")?;
		writeln!(f, "    => seriatim_allowed_origin: {}", self.allowed_origin)?;
		write!(f, "    => database_url:            set")
	}
}
