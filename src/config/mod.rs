use std::env;
use std::fmt;

pub struct SeriatimConfig {
	pub domain: String,
	pub twitter_key: String,
	pub twitter_secret: String,
	pub google_id: String,
	pub google_secret: String,
	pub google_api_key: String,
	pub fb_id: String,
	pub fb_secret: String,
	pub allowed_origin: String,
	pub session_domain: String,
	pub database_url: String,
}

impl SeriatimConfig {
	pub fn init() -> SeriatimConfig {
		SeriatimConfig {
			domain: env::var("SERIATIM_DOMAIN").unwrap(),
			twitter_key: env::var("SERIATIM_TWITTER_KEY").unwrap(),
			twitter_secret: env::var("SERIATIM_TWITTER_SECRET").unwrap(),
			google_id: env::var("SERIATIM_GOOGLE_ID").unwrap(),
			google_secret: env::var("SERIATIM_GOOGLE_SECRET").unwrap(),
			google_api_key: env::var("SERIATIM_GOOGLE_API_KEY").unwrap(),
			fb_id: env::var("SERIATIM_FB_ID").unwrap(),
			fb_secret: env::var("SERIATIM_FB_SECRET").unwrap(),
			allowed_origin: env::var("SERIATIM_ALLOWED_ORIGIN").unwrap(),
			session_domain: env::var("SERIATIM_SESSION_DOMAIN").unwrap(),
			database_url: env::var("DATABASE_URL").unwrap(),
		}
	}
}

impl fmt::Display for SeriatimConfig {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		writeln!(f, "    => seriatim_domain:         {}", self.domain)?;
		writeln!(f, "    => seriatim_twitter_key:    set")?;
		writeln!(f, "    => seriatim_twitter_secret: set")?;
		writeln!(f, "    => seriatim_google_id:      set")?;
		writeln!(f, "    => seriatim_google_secret:  set")?;
		writeln!(f, "    => seriatim_google_api_key: set")?;
		writeln!(f, "    => seriatim_fb_id:          set")?;
		writeln!(f, "    => seriatim_fb_secret:      set")?;
		writeln!(f, "    => seriatim_session_domain: {}", self.session_domain)?;
		writeln!(f, "    => seriatim_allowed_origin: {}", self.allowed_origin)?;
		write!(f, "    => database_url:            set")
	}
}
