pub use rapier3d;

use borger_plugin_sdk::traits::UntrackedState;
use glam::Vec3;
use rapier3d::prelude::*;
use std::fmt::{Debug, Error, Formatter};

///Wrapper around all types required to step the Rapier simulation.
///Due to time constraints, the entire physics scene must be rebuilt
///at the start of every tick. All rigid bodies are discarded at the
///end of the tick. Will revisit someday to make this less awful.
pub struct Rapier {
	///Resets every tick
	pub integration_parameters: IntegrationParameters,
	///Resets every tick
	pub islands: IslandManager,
	///Resets every tick
	pub broad_phase: BroadPhaseBvh,
	///Resets every tick
	pub narrow_phase: NarrowPhase,
	///Resets every tick
	pub rigid_bodies: RigidBodySet,
	///Resets every tick
	pub colliders: ColliderSet,
	///Resets every tick
	pub impulse_joints: ImpulseJointSet,
	///Resets every tick
	pub multibody_joints: MultibodyJointSet,
	///Resets every tick
	pub soft_bodies: SoftBodySet,
	///Resets every tick
	pub ccd_solver: CCDSolver,

	level_col_handle: Option<ColliderHandle>,
}

impl Default for Rapier {
	fn default() -> Self {
		Self {
			integration_parameters: IntegrationParameters::new(),
			islands: IslandManager::new(),
			broad_phase: BroadPhaseBvh::new(),
			narrow_phase: NarrowPhase::new(),
			rigid_bodies: RigidBodySet::new(),
			colliders: ColliderSet::new(),
			impulse_joints: ImpulseJointSet::new(),
			multibody_joints: MultibodyJointSet::new(),
			soft_bodies: SoftBodySet::new(),
			ccd_solver: CCDSolver::new(),
			level_col_handle: None,
		}
	}
}

impl UntrackedState for Rapier {
	fn reset_untracked(&mut self) {
		let mut colliders = ColliderSet::new();
		let level_col_handle = self
			.level_col_handle
			.map(|level_col_handle| {
				self.colliders.remove(
					level_col_handle,
					&mut self.islands,
					&mut self.rigid_bodies,
					&mut self.soft_bodies,
					false,
				)
			})
			.flatten()
			.map(|level_col| colliders.insert(level_col));

		*self = Rapier {
			colliders,
			level_col_handle,
			..Rapier::default()
		};
	}
}

impl Debug for Rapier {
	fn fmt(&self, _: &mut Formatter) -> Result<(), Error> {
		Ok(())
	}
}

impl Rapier {
	pub fn init_static_level_geom(&mut self, level_col: Collider) {
		self.level_col_handle = Some(self.colliders.insert(level_col));
	}

	pub fn step(&mut self, gravity: Vec3, hooks: &dyn PhysicsHooks, events: &dyn EventHandler) {
		PhysicsPipeline::new().step(
			gravity,
			&self.integration_parameters,
			&mut self.islands,
			&mut self.broad_phase,
			&mut self.narrow_phase,
			&mut self.rigid_bodies,
			&mut self.colliders,
			&mut self.impulse_joints,
			&mut self.multibody_joints,
			&mut self.soft_bodies,
			&mut self.ccd_solver,
			hooks,
			events,
		);
	}

	pub fn query<'a>(&'a self, filter: QueryFilter<'a>) -> QueryPipeline<'a> {
		self.broad_phase.as_query_pipeline(
			self.narrow_phase.query_dispatcher(),
			&self.rigid_bodies,
			&self.colliders,
			filter,
		)
	}
}
