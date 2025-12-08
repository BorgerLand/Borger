///Data structures designed to be compatible with the deterministic
///rollback model
pub mod collections;

///Primitive number types. When receiving data, both server and client
///assume each other to be little endian. Otherwise block compilation
///(you'd get a broken build)
#[cfg(target_endian = "little")]
pub mod primitive;
