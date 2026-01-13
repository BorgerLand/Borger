use crate::tick::TickInfo;
use crate::untracked::UntrackedState;
use glam::Vec3;
use rapier3d::prelude::*;
use std::fmt::{Debug, Error, Formatter};

//due to time constraints, the entire physics scene must be rebuilt
//at the start of every tick. all rigid bodies are discarded at the
//end of the tick. will revisit someday to make this less awful

pub struct Physics {
	//reset every tick
	pub islands: IslandManager,
	pub rigid_bodies: RigidBodySet,
	pub colliders: ColliderSet,
	level_col_handle: ColliderHandle,

	//stable
	integration_parameters: IntegrationParameters,
	ccd_solver: CCDSolver,
}

impl UntrackedState for Physics {
	fn reset_untracked(&mut self) {
		let level_col = self
			.colliders
			.remove(
				self.level_col_handle,
				&mut self.islands,
				&mut self.rigid_bodies,
				false,
			)
			.unwrap();

		self.islands = IslandManager::new();
		self.rigid_bodies = RigidBodySet::new();
		self.colliders = ColliderSet::new();

		self.level_col_handle = self.colliders.insert(level_col);
	}
}

impl Debug for Physics {
	fn fmt(&self, _: &mut Formatter) -> Result<(), Error> {
		Ok(())
	}
}

impl Physics {
	pub(crate) fn default() -> Self {
		let mut params = IntegrationParameters::default();
		params.dt = TickInfo::SIM_DT;

		let level_col = ColliderBuilder::cuboid(1000.0, 100.0, 1000.0)
			.translation(Vec3::new(0.0, -100.0, 0.0))
			.build();
		let mut colliders = ColliderSet::new();
		let level_col_handle = colliders.insert(level_col);

		Self {
			islands: IslandManager::new(),
			rigid_bodies: RigidBodySet::new(),
			colliders,

			integration_parameters: params,
			ccd_solver: CCDSolver::new(),

			level_col_handle,
		}
	}

	pub fn step(&mut self, gravity: Vec3) {
		PhysicsPipeline::new().step(
			gravity,
			&self.integration_parameters,
			&mut self.islands,
			&mut DefaultBroadPhase::new(),
			&mut NarrowPhase::new(),
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
}
