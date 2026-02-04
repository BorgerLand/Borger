use crate::interpolation::EntityBindings;
use crate::networked_types::primitive::usize32;
use glam::Mat4;
use js_sys::{Float32Array, Function, JsString, Reflect, SharedArrayBuffer, Uint32Array, WebAssembly};
use std::str::FromStr;
use wasm_bindgen::JsValue;
use web_sys::WritableStreamDefaultWriter;

pub struct JSBindings {
	pub camera: CameraBindings,
	pub entities: EntityBindings,

	pub cache: JSValueCache,
}

//you can't prevent illegal camera movement cheating using server authoritative code.
//the client can always ignore the server and render whatever they want. net visibility
//scopes are the intended solution for this: hide data that clients shouldn't be able to
//render in the first place.
//when the presentation tick has to wait on the simulation for a new camera transform,
//there is a small but noticeable delay. flicking the mouse to rotate quickly has the
//lowest tolerance for delay of any input, especially in twitchy shooters
#[derive(Default)]
pub struct CameraBindings {
	pub mat: Mat4,
	pub mat_inv: Mat4,
}

//assortment of frequently used js values because wasm<->js calls are slow
pub struct JSValueCache {
	pub elements_str: JsString,           //THREE.Matrix4.elements
	pub matrix_world_str: JsString,       //THREE.Object3D.matrixWorld
	pub scene_add: Function,              //THREE.Scene.prototype.add
	pub remove_from_parent_str: JsString, //THREE.Object3D.prototype.removeFromParent
	pub input_stream: WritableStreamDefaultWriter,
	pub spawn_entity_cb: Function, //(type: EntityType, id: number) => THREE.Object3D
	pub dispose_entity_cb: Function, //(type: EntityType, entity: THREE.Object3D, id: number) => void
}

impl JSBindings {
	pub fn new(
		input_stream: WritableStreamDefaultWriter,
		scene: &JsValue,             //THREE.Scene
		spawn_entity_cb: Function,   //(type: EntityType, id: number) => THREE.Object3D
		dispose_entity_cb: Function, //(type: EntityType, entity: THREE.Object3D, id: number) => void
	) -> Self {
		let scene_add =
			Function::from(Reflect::get(scene, &JsString::from_str("add").unwrap()).unwrap()).bind(scene);

		Self {
			camera: CameraBindings::default(),
			entities: EntityBindings::default(),

			cache: JSValueCache {
				elements_str: JsString::from_str("elements").unwrap(),
				matrix_world_str: JsString::from_str("matrixWorld").unwrap(),
				scene_add,
				remove_from_parent_str: JsString::from_str("removeFromParent").unwrap(),
				input_stream,
				spawn_entity_cb,
				dispose_entity_cb,
			},
		}
	}
}

//safety for any of the binding functions:
//- if the rust-owned memory is moved or dropped, the current binding becomes a dangling pointer.
//need to then either rebind or remove the object from the scene
//- for mysterious reasons, when the SharedArrayBuffer is replaced due to memory growth,
//changes in the new buffer are visible in the old buffer. this may be an implementation quirk
//where both SharedArrayBuffers point to the same os-level block of memory and i shouldn't be
//relying on this, but boy oh boy it sure is nice to enjoy it while it lasts. otherwise, growing
//the memory module would require rebinding every single matrix
//happening in chrome+firefox on ubuntu
//^ UPDATE ^ above is likely because of this
//https://spidermonkey.dev/blog/2025/01/15/is-memory64-actually-worth-using.html#how-is-memory-really-implemented

pub unsafe fn bind_matrix(
	three_matrix: &JsValue, /*THREE.Matrix4*/
	glam_matrix: &Mat4,
	cache: &JSValueCache,
) {
	Reflect::set(
		three_matrix,
		&cache.elements_str,
		&Float32Array::new_with_byte_offset_and_length(
			&SharedArrayBuffer::from(WebAssembly::Memory::from(wasm_bindgen::memory()).buffer()),
			glam_matrix as *const Mat4 as usize32,
			16,
		),
	)
	.unwrap();
}

pub unsafe fn bind_f32_array(block: &[f32]) -> Float32Array {
	Float32Array::new_with_byte_offset_and_length(
		&SharedArrayBuffer::from(WebAssembly::Memory::from(wasm_bindgen::memory()).buffer()),
		block.as_ptr() as usize32,
		block.len() as usize32,
	)
}

pub unsafe fn bind_u32_array(block: &[u32]) -> Uint32Array {
	Uint32Array::new_with_byte_offset_and_length(
		&SharedArrayBuffer::from(WebAssembly::Memory::from(wasm_bindgen::memory()).buffer()),
		block.as_ptr() as usize32,
		block.len() as usize32,
	)
}

//safety: do not move the bindings after calling this.
//to accomplish this, call bind_camera from jsland
//because it has permanently moved to the heap at this
//point
pub unsafe fn bind_camera(
	cam_rs: &CameraBindings,
	cam_ts: &JsValue, /*THREE.Camera*/
	cache: &JSValueCache,
) {
	unsafe {
		bind_matrix(
			&Reflect::get(cam_ts, &cache.matrix_world_str).unwrap(),
			&cam_rs.mat,
			cache,
		);
		bind_matrix(
			&Reflect::get(cam_ts, &JsString::from_str("matrixWorldInverse").unwrap()).unwrap(),
			&cam_rs.mat_inv,
			cache,
		);
	}
}
