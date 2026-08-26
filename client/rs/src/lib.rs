#[cfg(feature = "client")]
mod presentation_controller;
#[cfg(feature = "client")]
#[allow(unused, dead_code)]
mod generated {
	mod mem_offsets;
}

#[cfg(all(feature = "client", feature = "server"))]
compile_error!("Compiling both client+server into the same binary is dumb and broken.");
