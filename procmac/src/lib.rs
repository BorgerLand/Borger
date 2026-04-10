use proc_macro::TokenStream;

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
		#[cfg(feature = "server")]
		#(#attrs)*
		#vis #sig #block

		//client impl: guts the function body but keeps the
		//declaration so that it can still be referenced
		#[cfg(feature = "client")]
		#[allow(dead_code, unused)]
		#(#attrs)*
		#vis #sig {
			unimplemented!()
		}
	}
	.into()
}
