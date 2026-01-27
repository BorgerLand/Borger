use crate::entities::EntityKind;
use crate::js_bindings::{JSValueCache, bind_matrix};
use crate::networked_types::primitive::usize32;
use glam::{Mat4, Quat, Vec3A};
use js_sys::{Function, Reflect};
use std::collections::HashMap;
use std::fmt::Debug;
use std::ptr;
use wasm_bindgen::{JsValue, throw_val};

pub trait Entity: Debug {
	const KIND: EntityKind;

	fn get_pos(&self) -> Vec3A;
	fn get_rot(&self) -> Quat;
	fn get_scl(&self) -> Vec3A;
}

//the per-entity data exposed to js via array buffer views
pub(crate) struct JSData<T: Entity> {
	pub ptr: *const T,
	pub mat: Mat4,
}

impl<T: Entity> Default for JSData<T> {
	fn default() -> Self {
		Self {
			mat: Mat4::IDENTITY,
			ptr: ptr::null(),
		}
	}
}

//represents storage of all entities of a certain kind
//eg. "this one instance of InterpolatedEntityType owns
//all characters"
pub struct InterpolatedEntityType<T: Entity> {
	data: Vec<JSData<T>>,
	pub obj: HashMap<usize32, InterpolatedEntityInstance>,
}

impl<T: Entity> Default for InterpolatedEntityType<T> {
	fn default() -> Self {
		Self {
			data: Vec::default(),
			obj: HashMap::default(),
		}
	}
}

#[derive(Debug)]
pub struct InterpolatedEntityInstance {
	//ts side/bindings
	pub o3d: JsValue,      //THREE.Object3D
	matrix_world: JsValue, //THREE.Object3D.matrixWorld

	//rs side/transform
	pub pos: Vec3A,
	pub rot: Quat,
	pub scl: Vec3A,
	//would've been nice to store the glam mat in
	//here too, but i'm unable to find documentation
	//about hashbrown's pointer stability guarantees.
	//so instead there is a separate vec of glam mats
}

//interpolate all instances of one entity type. this
//has been optimized to avoid js<->wasm ffi overhead
//as much as possible. calling js should only happen
//when prv_entities and cur_entities contain different
//sets of entities and received_new_tick is true
pub fn interpolate_type<T: Entity>(
	received_new_tick: bool,
	prv_entities: &[(usize32, T)],
	cur_entities: &[(usize32, T)],
	out_entities: &mut InterpolatedEntityType<T>,
	amount: f32,
	cache: &JSValueCache,
) {
	let cur_count = cur_entities.len();
	out_entities
		.obj
		.reserve(cur_count.saturating_sub(out_entities.obj.len()));

	let prv_data_ptr = out_entities.data.as_ptr();
	out_entities.data.resize_with(cur_count, JSData::default);
	let rebind_all = prv_data_ptr != out_entities.data.as_ptr(); //can only be true if rebind is true

	//when entity id's don't match for a given physical
	//index, store them here to sort out later. using
	//option to avoid unnecessary heap allocations -
	//usually the number of entities does not change
	//from tick to tick
	let mut mismatch_prv_entities: Option<HashMap<usize32, &T>> = None;
	let mut mismatch_cur_entities: Option<HashMap<usize32, (usize, &T)>> = None;

	//loop through all entity pairs in order to detect
	//new/removed entities, and interpolate transforms
	//of entities that existed in both the previous and
	//current frames
	for physical_index in 0..prv_entities.len().max(cur_count) {
		let prv_entity = prv_entities.get(physical_index);
		let cur_entity = cur_entities.get(physical_index);

		if prv_entity.is_none() {
			//number of entities has increased since previous frame
			let cur_entity = cur_entity.unwrap();
			mismatch_cur_entities
				.get_or_insert_default()
				.insert(cur_entity.0, (physical_index, &cur_entity.1));
		} else if cur_entity.is_none() {
			//number of entities has decreased since previous frame
			let prv_entity = prv_entity.unwrap();
			mismatch_prv_entities
				.get_or_insert_default()
				.insert(prv_entity.0, &prv_entity.1);
		} else
		//both are some
		{
			let prv_entity = prv_entity.unwrap();
			let cur_entity = cur_entity.unwrap();

			if prv_entity.0 == cur_entity.0 {
				//this branch (and rebind_all == false) should be what
				//executes most often: entity is still in the same slot
				//as the previous tick so just reinterpolate and let
				//three.js do the rest

				let js_obj = out_entities.obj.get_mut(&cur_entity.0).unwrap();
				js_obj.pos = prv_entity.1.get_pos().lerp(cur_entity.1.get_pos(), amount);
				js_obj.rot = prv_entity.1.get_rot().slerp(cur_entity.1.get_rot(), amount);
				js_obj.scl = prv_entity.1.get_scl().lerp(cur_entity.1.get_scl(), amount);
				out_entities.data[physical_index] = JSData {
					mat: Mat4::from_scale_rotation_translation(
						js_obj.scl.into(),
						js_obj.rot,
						js_obj.pos.into(),
					),
					ptr: &cur_entity.1 as *const T,
				};

				if rebind_all {
					//the matrices array was reallocated in order
					//to grow. all entities are now bound to a
					//dangling pointer
					unsafe {
						bind_matrix(
							&out_entities.obj.get_mut(&cur_entity.0).unwrap().matrix_world,
							&out_entities.data[physical_index].mat,
							cache,
						);
					}
				}
			} else {
				mismatch_prv_entities
					.get_or_insert_default()
					.insert(prv_entity.0, &prv_entity.1);
				mismatch_cur_entities
					.get_or_insert_default()
					.insert(cur_entity.0, (physical_index, &cur_entity.1));
			}
		}

		//none for both cur+prv is logically impossible
	}

	//fix any mismatched id's
	//note if an entity with id x was deleted, and a
	//new entity also with id x was created within the
	//same tick, there is no way to tell this apart
	//from moving slot. the engine always assumes move
	//because this is more common. the effect is that
	//dispose/spawn will not be triggered, so the same
	//Object3D will be reused instead of recreated
	if let Some(mismatch_cur_entities) = mismatch_cur_entities {
		for (id, (physical_index, cur_entity)) in mismatch_cur_entities {
			let rebind_matrix = if let Some(prv_entities) = &mut mismatch_prv_entities
				&& let Some(prv_entity) = prv_entities.remove(&id)
			{
				//entity moved slots since last tick (or this is
				//a new entity with the same id, but its Object3D
				//will be reused)

				let js_obj = out_entities.obj.get_mut(&id).unwrap();
				js_obj.pos = prv_entity.get_pos().lerp(cur_entity.get_pos(), amount);
				js_obj.rot = prv_entity.get_rot().slerp(cur_entity.get_rot(), amount);
				js_obj.scl = prv_entity.get_scl().lerp(cur_entity.get_scl(), amount);
				out_entities.data[physical_index] = JSData {
					mat: Mat4::from_scale_rotation_translation(
						js_obj.scl.into(),
						js_obj.rot,
						js_obj.pos.into(),
					),
					ptr: cur_entity as *const T,
				};

				if received_new_tick {
					Some(&js_obj.matrix_world)
				} else {
					None
				}
			} else if received_new_tick {
				//new entity

				//call spawn_entity_cb
				let object_3d = cache
					.spawn_entity_cb
					.call1(&JsValue::NULL, &JsValue::from(T::KIND))
					.unwrap_or_else(|err| throw_val(err));

				//call THREE.Scene.add(new_entity);
				cache.scene_add.call1(&JsValue::NULL, &object_3d).unwrap();

				let matrix_world = Reflect::get(&object_3d, &cache.matrix_world_str).unwrap();
				out_entities.data[physical_index] = JSData {
					mat: Mat4::from_scale_rotation_translation(
						cur_entity.get_scl().into(),
						cur_entity.get_rot(),
						cur_entity.get_pos().into(),
					),
					ptr: cur_entity as *const T,
				};

				Some(
					&out_entities
						.obj
						.entry(id)
						.or_insert(InterpolatedEntityInstance {
							o3d: object_3d,
							matrix_world,
							pos: cur_entity.get_pos(),
							rot: cur_entity.get_rot(),
							scl: cur_entity.get_scl(),
						})
						.matrix_world,
				)
			} else {
				None
			};

			if let Some(three_matrix) = rebind_matrix {
				unsafe {
					bind_matrix(three_matrix, &out_entities.data[physical_index].mat, cache);
				}
			}
		}
	}

	if received_new_tick && let Some(mismatch_prv_entities) = mismatch_prv_entities {
		for (id, _) in mismatch_prv_entities {
			//deleted entity

			let object_3d = out_entities.obj.remove(&id).unwrap().o3d;

			//call THREE.Object3D.removeFromParent();
			Function::from(Reflect::get(&object_3d, &cache.remove_from_parent_str).unwrap())
				.call0(&object_3d)
				.unwrap();

			//call dispose_entity_cb
			cache
				.dispose_entity_cb
				.call2(&JsValue::NULL, &JsValue::from(T::KIND), &object_3d)
				.unwrap_or_else(|err| throw_val(err));
		}
	}
}
