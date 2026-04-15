///Collection data structures designed to be compatible with the
///deterministic rollback model
pub mod collections;

///Primitive number types
//When receiving data, both server and client
//assume each other to be little endian. Otherwise block compilation
//(you'd get a broken build)
#[cfg(target_endian = "little")]
pub mod primitive;

///Event dispatcher for non-critical, unrollbackable game feel
///events: camera/shakes, footsteps, particle effects, etc. Do
///not rely on this to work 100% of the time. For example, a
///rollback can undo the event before it ever hits presentation.
pub mod event_dispatcher;
