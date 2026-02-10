use crate::tick::TickInfo;
use crate::untracked::UntrackedState;
use glam::Vec3;
use rapier3d::prelude::*;
use std::fmt::{Debug, Error, Formatter};

///Wrapper around all types required to step the Rapier simulation.
///Due to time constraints, the entire physics scene must be rebuilt
///at the start of every tick. All rigid bodies are discarded at the
///end of the tick. Will revisit someday to make this less awful.
pub struct Physics {
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
	pub ccd_solver: CCDSolver,

	level_col_handle: ColliderHandle,
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

		self.integration_parameters = IntegrationParameters::default();
		self.integration_parameters.dt = TickInfo::SIM_DT;

		self.islands = IslandManager::new();
		self.broad_phase = BroadPhaseBvh::new();
		self.narrow_phase = NarrowPhase::new();
		self.rigid_bodies = RigidBodySet::new();
		self.colliders = ColliderSet::new();
		self.impulse_joints = ImpulseJointSet::new();
		self.multibody_joints = MultibodyJointSet::new();
		self.ccd_solver = CCDSolver::new();

		self.level_col_handle = self.colliders.insert(level_col);
	}
}

impl Debug for Physics {
	fn fmt(&self, _: &mut Formatter) -> Result<(), Error> {
		Ok(())
	}
}

impl Physics {
	#[allow(unused)]
	pub(crate) fn default() -> Self {
		let level_col = ColliderBuilder::cuboid(100.0, 25.0, 100.0)
			.translation(Vec3::new(0.0, -25.0, 0.0))
			.build();
		let mut colliders = ColliderSet::new();
		let level_col_handle = colliders.insert(level_col);

		Self {
			integration_parameters: IntegrationParameters::default(),
			islands: IslandManager::new(),
			broad_phase: BroadPhaseBvh::new(),
			narrow_phase: NarrowPhase::new(),
			rigid_bodies: RigidBodySet::new(),
			colliders,
			impulse_joints: ImpulseJointSet::new(),
			multibody_joints: MultibodyJointSet::new(),
			ccd_solver: CCDSolver::new(),

			level_col_handle,
		}
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
