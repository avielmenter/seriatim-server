#![recursion_limit = "256"]
extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

fn impl_tagged_id(ast: &syn::DeriveInput) -> quote::Tokens {
	let name = &ast.ident;

	quote! {
		use rocket;
		use rocket::outcome::IntoOutcome;
		use std;
		use serde;

		impl #name {
			pub fn cookie_name() -> &'static str {
				stringify!(#name)
			}

			pub fn from_uuid(uuid: uuid::Uuid) -> Self {
				#name(uuid)
			}

			#[allow(dead_code)]
			pub fn to_cookie(&self) -> rocket::http::Cookie<'static> {
				rocket::http::Cookie::new(Self::cookie_name(), self.0.hyphenated().to_string())
			}

			#[allow(dead_code)]
			pub fn json_str(&self) -> String {
				self.0.hyphenated().to_string()
			}
		}

		impl<'a, 'r> rocket::request::FromRequest<'a, 'r> for #name {
			type Error = ();

			fn from_request(
				request: &'a rocket::request::Request<'r>,
			) -> rocket::request::Outcome<Self, Self::Error> {
				request
					.cookies()
					.get_private(Self::cookie_name())
					.and_then(|c_id| uuid::Uuid::parse_str(c_id.value()).ok())
					.and_then(|uuid| Some(Self::from_uuid(uuid)))
					.or_forward(())
			}
		}

		impl<'a> rocket::request::FromParam<'a> for #name {
			type Error = ();

			fn from_param(param: &'a rocket::http::RawStr) -> Result<Self, Self::Error> {
				let param_str = match param.url_decode() {
					Ok(s) => Ok(s),
					Err(_) => Err(()),
				}?;

				let uuid = match uuid::Uuid::parse_str(&param_str) {
					Ok(s) => Ok(s),
					Err(_) => Err(()),
				}?;

				Ok(Self::from_uuid(uuid))
			}
		}

		impl std::ops::Deref for #name {
			type Target = uuid::Uuid;

			fn deref(&self) -> &uuid::Uuid {
				&self.0
			}
		}

		impl std::fmt::Display for #name {
			fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
				write!(f, "{} ( {} )", stringify!(#name), self.0.hyphenated())
			}
		}

		impl std::fmt::Debug for #name {
			fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
				write!(f, "{} ( {} )", stringify!(#name), self.0.hyphenated())
			}
		}

		impl std::str::FromStr for #name {
			type Err = uuid::ParseError;

			fn from_str(s: &str) -> Result<Self, Self::Err> {
				let uuid = uuid::Uuid::parse_str(&s)?;
				Ok(Self::from_uuid(uuid))
			}
		}

		impl std::cmp::PartialEq for #name {
			fn eq(&self, other: &Self) -> bool {
				self.0 == other.0
			}
		}

		impl std::cmp::Eq for #name {}

		impl std::hash::Hash for #name {
			fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
				stringify!(#name).hash(state);
				self.0.hash(state);
			}
		}

		impl serde::ser::Serialize for #name {
			fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
			where
				S: serde::ser::Serializer,
			{
				serializer.serialize_str(&self.0.hyphenated().to_string())
			}
		}
	}
}

#[proc_macro_derive(TaggedID)]
pub fn tagged_id(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let s = input.to_string();
	let ast = syn::parse_derive_input(&s).unwrap();

	let gen = impl_tagged_id(&ast);

	gen.parse().unwrap()
}
