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
		#[cfg(feature = "server")]
		#(#attrs)*
		#vis #sig #block

		#[cfg(not(feature = "server"))]
		#[allow(dead_code, unused)]
		#(#attrs)*
		#vis #sig {
			unimplemented!()
		}
	}
	.into()
}
