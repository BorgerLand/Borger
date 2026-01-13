use crate::tick::TickInfo;
use glam::Vec3;
use rapier3d::prelude::*;

//due to time constraints, the entire physics scene must be rebuilt
//at the start of every tick. all rigid bodies are discarded at the
//end of the tick. will revisit someday to make this less awful

pub struct Physics {
	//reset every tick
	pub islands: IslandManager,
	pub rigid_bodies: RigidBodySet,
	pub colliders: ColliderSet,

	//stable
	integration_parameters: IntegrationParameters,
	physics_pipeline: PhysicsPipeline,
	broad_phase: DefaultBroadPhase,
	narrow_phase: NarrowPhase,
	ccd_solver: CCDSolver,

	level_collider: Option<Collider>,
	level_collider_handle: Option<ColliderHandle>,
}

impl Physics {
	pub(crate) fn new() -> Self {
		let mut params = IntegrationParameters::default();
		params.dt = TickInfo::SIM_DT;

		Self {
			islands: IslandManager::new(),
			rigid_bodies: RigidBodySet::new(),
			colliders: ColliderSet::new(),

			integration_parameters: params,
			physics_pipeline: PhysicsPipeline::new(),
			broad_phase: DefaultBroadPhase::new(),
			narrow_phase: NarrowPhase::new(),
			ccd_solver: CCDSolver::new(),

			level_collider: Some(
				ColliderBuilder::cuboid(1000.0, 100.0, 1000.0)
					.translation(Vec3::new(0.0, -100.0, 0.0))
					.build(),
			),
			level_collider_handle: None,
		}
	}

	pub fn step(&mut self, gravity: Vec3) {
		self.physics_pipeline.step(
			gravity,
			&self.integration_parameters,
			&mut self.islands,
			&mut self.broad_phase,
			&mut self.narrow_phase,
			&mut self.rigid_bodies,
			&mut self.colliders,
			&mut ImpulseJointSet::new(),
			&mut MultibodyJointSet::new(),
			&mut self.ccd_solver,
			&(),
			&(),
		);
	}

	pub fn get_rigid_body(&self, handle: RigidBodyHandle) -> Option<&RigidBody> {
		self.rigid_bodies.get(handle)
	}

	pub(crate) fn start_tick(&mut self) {
		let handle = self.colliders.insert(self.level_collider.take().unwrap());
		self.level_collider_handle = Some(handle);
	}

	pub(crate) fn end_tick(&mut self) {
		let handle = self.level_collider_handle.take().unwrap();
		self.level_collider = Some(
			self.colliders
				.remove(handle, &mut self.islands, &mut self.rigid_bodies, false)
				.unwrap(),
		);

		self.islands = IslandManager::new();
		self.rigid_bodies = RigidBodySet::new();
		self.colliders = ColliderSet::new();
	}
}
