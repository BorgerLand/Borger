use crate::interpolation::EntityKind;
use crate::js_bindings::{JSValueCache, bind_matrix};
use crate::networked_types::primitive::usize32;
use glam::Mat4;
use js_sys::{Function, Reflect};
use std::collections::HashMap;
use wasm_bindgen::{JsValue, throw_val};

//keeping this a separate trait for a future where other
//parts of the simulation state can be interpolated
pub trait Interpolate {
	fn interpolate(prv: &Self, cur: &Self, amount: f32) -> Self;
}

pub trait Entity: Interpolate + Default + Clone {
	const KIND: EntityKind;

	fn get_matrix_world(&self) -> Mat4;
}

pub struct EntityInstanceBindings<T: Entity> {
	pub js: EntityInstanceJSBindings,
	pub rs: EntityInstanceRSBindings<T>,
}

#[derive(Clone)]
pub struct EntityInstanceJSBindings {
	pub o3d: JsValue,          //THREE.Object3D
	pub matrix_world: JsValue, //THREE.Object3D.matrixWorld
}

pub struct EntityInstanceRSBindings<T: Entity> {
	pub slot_id: usize32, //the slot id this entity lives in on the simulation side
	pub interpolated: T,  //the interpolated presentation state
	pub mat: Mat4,        //THREE.Object3D.matrixWorld.elements, output of Entity.get_matrix_world
}

//derive macro not working...
impl<T: Entity> Default for EntityInstanceBindings<T> {
	fn default() -> Self {
		Self {
			js: EntityInstanceJSBindings {
				o3d: JsValue::default(),
				matrix_world: JsValue::default(),
			},
			rs: EntityInstanceRSBindings {
				slot_id: 0,
				interpolated: T::default(),
				mat: Mat4::IDENTITY,
			},
		}
	}
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
	out_entities: &mut Vec<EntityInstanceBindings<T>>,
	amount: f32,
	cache: &JSValueCache,
) {
	let allocated_len = prv_entities.len().max(cur_entities.len());
	let prv_data_ptr = out_entities.as_ptr();
	out_entities.resize_with(allocated_len, EntityInstanceBindings::default);
	let rebind_all = prv_data_ptr != out_entities.as_ptr(); //can only be true if rebind is true

	//when entity id's don't match for a given physical
	//index, store them here to sort out later. using
	//option to avoid unnecessary heap allocations -
	//usually the number of entities does not change
	//from tick to tick
	let mut mismatch_prv_entities: Option<HashMap<usize32, EntityInstanceJSBindings>> = None;
	let mut mismatch_cur_entities: Option<HashMap<usize32, (&T, usize)>> = None;

	//loop through all entity pairs in order to detect
	//new/removed entities, and interpolate transforms
	//of entities that existed in both the previous and
	//current frames
	for slot_index in 0..allocated_len {
		let prv_entity = prv_entities.get(slot_index);
		let cur_entity = cur_entities.get(slot_index);
		if let (Some(prv_entity), Some(cur_entity)) = (prv_entity, cur_entity)
			&& prv_entity.0 == cur_entity.0
		{
			//this branch (and rebind_all == false) should be what
			//executes most often: entity is still in the same slot
			//as the previous tick so just reinterpolate and let
			//three.js do the rest

			let bindings = &mut out_entities[slot_index];
			bindings.rs.interpolated = Interpolate::interpolate(&prv_entity.1, &cur_entity.1, amount);
			bindings.rs.mat = bindings.rs.interpolated.get_matrix_world();

			if rebind_all {
				//the matrices array was reallocated in order
				//to grow. all entities are now bound to a
				//dangling pointer
				unsafe {
					bind_matrix(&bindings.js.matrix_world, &bindings.rs.mat, cache);
				}
			}
		} else if received_new_tick {
			if let Some(prv_entity) = prv_entity {
				mismatch_prv_entities
					.get_or_insert_default()
					.insert(prv_entity.0, out_entities[slot_index].js.clone());
			}

			if let Some(cur_entity) = cur_entity {
				mismatch_cur_entities
					.get_or_insert_default()
					.insert(cur_entity.0, (&cur_entity.1, slot_index));
			}
		}
	}

	if received_new_tick {
		//allow removed entities to have their js data be gc'ed
		out_entities.truncate(cur_entities.len());

		//fix any mismatched id's
		if let Some(mismatch_cur_entities) = mismatch_cur_entities {
			for (slot_id, (cur_entity, slot_index)) in mismatch_cur_entities {
				let bindings = &mut out_entities[slot_index];
				*bindings = EntityInstanceBindings {
					js: if let Some(prv_entities) = &mut mismatch_prv_entities
						&& let Some(bindings_js) = prv_entities.remove(&slot_id)
					{
						//entity moved to a different physical slot index

						//note if an entity with slot id x was deleted, and a new entity
						//also with slot id x was created since the last tick, there is no
						//way to tell this apart from moving slot. the engine always
						//assumes move because this is more common. the effect is that
						//dispose/spawn will not be triggered, so the same Object3D will
						//be reused instead of recreated. because there's no way to tell
						//the difference, interpolating here would be risky in that you may
						//see a massive visual lerp across the screen from the old object
						//to the new object. so just use the current transform to be safe
						bindings_js
					} else {
						//new entity

						//call spawn_entity_cb
						let o3d = cache
							.spawn_entity_cb
							.call1(&JsValue::NULL, &JsValue::from(T::KIND))
							.unwrap_or_else(|err| throw_val(err));

						//call THREE.Scene.add(new_entity);
						cache.scene_add.call1(&JsValue::NULL, &o3d).unwrap();
						let matrix_world = Reflect::get(&o3d, &cache.matrix_world_str).unwrap();

						EntityInstanceJSBindings { o3d, matrix_world }
					},
					rs: EntityInstanceRSBindings {
						slot_id,
						interpolated: cur_entity.clone(),
						mat: cur_entity.get_matrix_world(),
					},
				};

				unsafe {
					bind_matrix(&bindings.js.matrix_world, &bindings.rs.mat, cache);
				}
			}
		}

		if let Some(mismatch_prv_entities) = mismatch_prv_entities {
			for (_, bindings_js) in mismatch_prv_entities {
				//deleted entity

				//call THREE.Object3D.removeFromParent();
				Function::from(Reflect::get(&bindings_js.o3d, &cache.remove_from_parent_str).unwrap())
					.call0(&bindings_js.o3d)
					.unwrap();

				//call dispose_entity_cb
				cache
					.dispose_entity_cb
					.call2(&JsValue::NULL, &JsValue::from(T::KIND), &bindings_js.o3d)
					.unwrap_or_else(|err| throw_val(err));
			}
		}
	}
}
