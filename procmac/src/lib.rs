use proc_macro::TokenStream;
use quote::{format_ident, quote};
use std::collections::HashMap;
use std::fs;
use syn::{Data, DeriveInput, LitStr, parse_macro_input};

#[proc_macro_attribute]
pub fn server(_: TokenStream, item: TokenStream) -> TokenStream {
	let func = match syn::parse::<syn::ItemFn>(item) {
		Ok(f) => f,
		Err(_) => {
			return quote::quote! {
				compile_error!("#[server] macro can only be applied to functions");
			}
			.into();
		}
	};

	let sig = &func.sig;
	let vis = &func.vis;
	let attrs = &func.attrs;
	let block = &func.block;

	quote::quote! {
		//server impl: this is a no-op
		#[cfg(feature = "server_internal_only_dont_use")]
		#(#attrs)*
		#vis #sig #block

		//client impl: guts the function body but keeps the
		//declaration so that it can still be referenced
		#[cfg(feature = "client_internal_only_dont_use")]
		#[allow(dead_code, unused)]
		#(#attrs)*
		#vis #sig {
			unimplemented!()
		}
	}
	.into()
}

fn load_diff_ops() -> HashMap<String, HashMap<String, u8>> {
	let manifest_path = concat!(
		env!("CARGO_MANIFEST_DIR"),
		"/../engine/src/generated/diff_operation.json"
	);
	let contents =
		fs::read_to_string(manifest_path).unwrap_or_else(|e| panic!("Failed to read {manifest_path}: {e}"));

	serde_json::from_str(&contents).unwrap_or_else(|e| panic!("Invalid JSON in diff_operation.json: {e}"))
}

#[proc_macro_derive(EnumSerDes)]
pub fn derive_primitive_ser_des(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	let name = &input.ident;

	// Only enums.
	let Data::Enum(data) = &input.data else {
		return syn::Error::new_spanned(name, "EnumSerDes can only be derived on enums")
			.to_compile_error()
			.into();
	};

	// No generics on a simple enum.
	if !input.generics.params.is_empty() {
		return syn::Error::new_spanned(&input.generics, "EnumSerDes does not support generic enums")
			.to_compile_error()
			.into();
	}

	// Must be #[repr(u8)].
	let has_repr_u8 = input.attrs.iter().any(|attr| {
		attr.path().is_ident("repr")
			&& attr
				.parse_args_with(syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated)
				.map(|paths| paths.iter().any(|p| p.is_ident("u8")))
				.unwrap_or(false)
	});

	if !has_repr_u8 {
		return syn::Error::new_spanned(name, "EnumSerDes requires #[repr(u8)]")
			.to_compile_error()
			.into();
	}

	// Simple enums only: every variant must be a bare name, optionally with `= N`.
	// No tuple or struct variants.
	for variant in &data.variants {
		if !matches!(variant.fields, syn::Fields::Unit) {
			return syn::Error::new_spanned(
				variant,
				"EnumSerDes only supports simple enums (no tuple or struct variants)",
			)
			.to_compile_error()
			.into();
		}
	}

	// Compare against `Enum::Variant as u8` so the compiler resolves each
	// discriminant, whether explicit or implicit.
	let from_arms = data.variants.iter().map(|v| {
		let ident = &v.ident;
		quote! { x if x == #name::#ident as u8 => Ok(#name::#ident) }
	});

	let expanded = quote! {
		impl ::core::convert::From<#name> for u8 {
			fn from(v: #name) -> u8 {
				v as u8
			}
		}

		impl ::core::convert::TryFrom<u8> for #name {
			type Error = u8;

			fn try_from(byte: u8) -> ::core::result::Result<Self, Self::Error> {
				match byte {
					#(#from_arms,)*
					_ => Err(byte),
				}
			}
		}

		impl ::borger_plugin_sdk::primitive::PrimitiveSerDes for #name {
			fn ser_rollback(self, buffer: &mut Vec<u8>) {
				buffer.push(self.into());
			}

			fn des_rollback(
				buffer: &mut Vec<u8>,
			) -> Result<Self, ::borger_plugin_sdk::primitive::DeserializeOopsy> {
				buffer
					.pop()
					.ok_or(::borger_plugin_sdk::primitive::DeserializeOopsy)?
					.try_into()
					.map_err(|_| ::borger_plugin_sdk::primitive::DeserializeOopsy)
			}

			fn des_rx(
				buffer: &mut impl Iterator<Item = u8>,
			) -> Result<Self, ::borger_plugin_sdk::primitive::DeserializeOopsy> {
				buffer
					.next()
					.ok_or(::borger_plugin_sdk::primitive::DeserializeOopsy)?
					.try_into()
					.map_err(|_| ::borger_plugin_sdk::primitive::DeserializeOopsy)
			}
		}
	};

	expanded.into()
}

#[proc_macro]
pub fn diff_operation_enum(plugin_name: TokenStream) -> TokenStream {
	let plugin_name = parse_macro_input!(plugin_name as LitStr).value();

	let plugins = load_diff_ops();
	let diff_ops = plugins
		.get(&plugin_name)
		.unwrap_or_else(|| panic!("Plugin '{plugin_name}' not found in diff_operation.json"));

	let variants = diff_ops.iter().map(|(name, value)| {
		let ident = format_ident!("{}", name);
		quote! {
			#ident = #value
		}
	});

	let vis = if plugin_name == "base" {
		quote! { pub }
	} else {
		quote! {}
	};

	let expanded = quote! {
		#[derive(Debug, Clone, Copy, PartialEq, Eq, ::borger_procmac::EnumSerDes)]
		#[repr(u8)]
		#vis enum DiffOperation {
			#(#variants),*
		}
	};

	expanded.into()
}

#[proc_macro]
pub fn get_plugin_diff_op_range(_: TokenStream) -> TokenStream {
	let plugins = load_diff_ops();

	let (min, max) = plugins
		.iter()
		.filter(|(name, _)| name.as_str() != "base")
		.flat_map(|(_, ops)| ops.values())
		.fold(None, |acc: Option<(u8, u8)>, &v| match acc {
			None => Some((v, v)),
			Some((lo, hi)) => Some((lo.min(v), hi.max(v))),
		})
		.unwrap_or_else(|| panic!("No non-base plugins found in diff_operation.json"));

	quote! {
		#min..=#max
	}
	.into()
}
