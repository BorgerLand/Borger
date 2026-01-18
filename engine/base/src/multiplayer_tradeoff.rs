///Although all game logic code is meant to be interpreted as server authoritative and
///from the server's top-down perspective controlling everything, you as the game
///developer can choose how each of your game mechanics feel, on a spectrum between
///"responsive+potentially incorrect" to "laggy+definitely correct". Game state is always
///eventually consistent, but this macro lets you decide when and where to execute logic
///in order to fine-tune game feel. This is possible because the engine executes each
///simulation tick multiple times across the server and all connected clients, requiring
///game logic to be **deterministic** depending on the chosen tradeoff.
///
///multiplayer_tradeoff!() blocks can be nested inside each other, but only in order of
///increasing latency:
///
///`Immediate` (outer) → `WaitForServer` → `WaitForConsensus` (inner)
///
///The implementation of the macro itself is very simple:
///- Immediate is the default because it functionally does nothing other than declare
///intent to the developer. Both the server and client call the simulation_tick, so they
///will naturally both run the same code.
///- WaitForServer removes the block of code from the client build, causing it to only run
///on the server.
///- WaitForConsensus also removes the code from the client, and additionally will prevent
///the code from running during resimulation of the same tick until all client inputs have
///arrived for that particular tick.
///- The actual "magic" part, eventual consistency, is enforced as a consequence of the
///engine's design.
///
///|                            |`Immediate` (default)                                                                                                                                                                                                                                                                                                        |`WaitForServer`                                                                                                                                                                                                                                              |`WaitForConsensus`                                                                                                                                                                 |
///|----------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
///|**Where**                   |Code runs on both client and server. It is worth mentioning that use of Immediate mode does NOT give clients any cheating ability or authority over what everyone else sees - it's simply a local prediction.                                                                                                                |Code runs only on the server                                                                                                                                                                                                                                 |Code runs only on the server                                                                                                                                                       |
///|**Latency/Responsiveness**  |Instantaneous. Press a button on the client, see result on screen immediately without waiting for server reply.                                                                                                                                                                                                              |Slower if used for processing client inputs/interactions (their individual RTT), otherwise feels smooth. The server executes this regardless of whether it has received inputs from all clients, and clients will render the result whenever they receive it.|Slowest. The server waits to receive all inputs from all clients (up to 3 seconds before timing out) before executing. Clients won’t see the result until RTT of the slowest client.|
///|**Correctness**             |May be wrong (“mispredicts”) due to not having access to all relevant state. The server may produce different results than the client.                                                                                                                                                                                       |If it is possible for a client's input to change the results (eg. someone stepping in front of an NPC), there is a risk of mispredicts.                                                                                                                      |Always correct. No mispredicts.                                                                                                                                                    |
///|**State Visibility**        |Can only access client-visible state. Accessing private state causes a compile error; out-of-scope access causes a runtime error.                                                                                                                                                                                            |Has full access to all game state, including private/hidden data.                                                                                                                                                                                            |Has full access to all game state, including private/hidden data.                                                                                                                  |
///|**Determinism Requirements**|Must be deterministic across all devices running this block of code. Minor floating-point variations across CPU architectures are *usually* acceptable. Note that comparisons between nearly identical floating-point numbers may produce conflicting boolean results across devices, which could cause a jarring mispredict.|Must be locally deterministic (consistent results on the same device across multiple runs, but may vary between devices)                                                                                                                                     |Determinism is not required because this code will only ever run once. Perfect for making backend calls, eg. a leaderboard update                                                  |
///|**Example Use Cases**       |Processing client inputs (movement, shooting, etc.) should happen here in Immediate as much as possible for best game feel. Controls are by far the most latency-sensitive aspect of gameplay. Any physics interactions should also be immediate whenever possible: damage-on-collision mechanics, moving platforms, etc.    |Logic that affects entities seen by clients but are unable to be predicted by clients due to having some private state (NPC)                                                                                                                                 |Large/important game state change events that would look horrible if rolled back/mispredicted (game over, level change)                                                            |
///
///## Parameters
///- `tradeoff` - The tradeoff level: `Immediate`, `WaitForServer`, or `WaitForConsensus`
///- `ctx` - The variable to rebind with the new context. Type can be either `&mut
///GameContext` or `&mut DiffSerializer`. You can either pass just a variable name (no &
///or .) or declare a new variable (eg. literally write out `let diff = &mut ctx.diff`)`
///- `tick` - Expression that evaluates to a `&TickInfo` (required for `WaitForConsensus`
///only)
///- `code` - The code block to execute within the specified tradeoff context. Can be
///an expression or a bracketed block
///
///## Game logic example:
///```rust
///pub fn simulation_tick(ctx: &mut GameContext<Immediate>)
///{
///	for character in ctx.state.characters.values_mut()
///	{
///		match character.input_owner
///		{
///			//client-controlled - we're already in the immediate block, so client side
///			//prediction is active
///			InputOwner::Client =>
///			{
///				//this hypothetical get_input function only *optionally* returns an input:
///				//clients can only access their own inputs. when the server also runs
///				//this block of code, it can access all inputs
///				if let Some(input) = get_input(character)
///				{
///					process_input(character, input, &mut ctx.diff);
///				}
///			},
///
///			//npc-controlled - in this example, npc input/decision-making state is
///			//defined as private/server only (state schema shown below). Without WaitForServer,
///			//the client build would not compile because the private struct fields are removed
///			InputOwner::NPC =>
///			{
///				//safe to unwrap/assert here on get_input. server always has access to all state.
///				//also take note of this nesting example - the inner (WaitForServer) block can
///				//still access variables declared in the outer (Immediate) block
///				multiplayer_tradeoff!(WaitForServer, let diff = &mut ctx.diff,
///						process_input(character, get_input(character).unwrap(), diff));
///			},
///		}
///	}
///
///	multiplayer_tradeoff!(WaitForConsensus, ctx, ctx.tick,
///	{
///		if collision_check_finish_line(state)
///		{
///			game_over(ctx);
///
///			//use WaitForConsensus here to avoid mispredicts/rollbacks,
///			//at the cost of standing around waiting at the finish line
///			//(should be 3 seconds absolute worst case). we want to avoid
///			//someone seeing "you win!" then a second later their screen
///			//changes to "you lose!" at all costs
///		}
///	});
///}
///
/////process_input is called from both an Immediate block and a WaitForServer block
///pub fn process_input(character: &mut Character, input: &InputState, diff: &mut DiffSerializer<impl ImmediateOrWaitForServer>)
///{
///	let mut pos = character.get_pos();
///	pos.x += input.omnidir.x * TickInfo::SIM_DT;
///	character.set_pos(pos, diff);
///	//                     ^ diff serializer records all state mutations, the key to engine's rollback implementation
///}
///```
///
///State schema example for the above game logic:
///
///(Note this has been cut down to illustrate only the shape of
///the SimulationState object and how usage of netVisibility
///affects the required multiplayer_tradeoff)
///```typescript
///import type { SimulationState } from "@engine/code_generator/StateSchema.ts";
///
///export default {
///	//represents a real person connected to a server
///	clients: {
///		netVisibility: "Public",
///		type: "SlotMap",
///		content: {
///			input: {
///				netVisibility: "Owner",
///				type: "struct",
///				content: {
///					omnidir: { netVisibility: "Owner", type: "Vec2" },
///				},
///			},
///			character_id: { netVisibility: "Public", type: "usize32" },
///		},
///	},
///	//represents the internal thought process of a non-player character
///	npcs: {
///		netVisibility: "Private",
///		type: "SlotMap",
///		content: {
///			character_id: { netVisibility: "Private", type: "usize32" },
///			will_drop_rare_item: { netVisibility: "Private", type: "bool" },
///			//more secret stuff that clients shouldn't know about...
///		},
///	},
///	//represents an entity being rendered, who may be controlled by either a client or npc
///	characters: {
///		netVisibility: "Public",
///		type: "SlotMap",
///		content: {
///			pos: { netVisibility: "Public", type: "Vec3" },
///			input_owner: { netVisibility: "Public", type: "enum", content: ["Client", "NPC"] },
///		},
///	},
///} satisfies SimulationState;
///```
#[macro_export]
macro_rules! multiplayer_tradeoff
{
	//Immediate - no-op
	(Immediate, $ctx:ident, $code:stmt) =>
	{
		{
			let $ctx = unsafe { $ctx._to_immediate_unchecked() };
			$code
		}
	};
	(Immediate, let $rebind:ident = $ctx:expr, $code:stmt) =>
	{
		{
			let $rebind = $ctx;
			let $rebind = unsafe { $rebind._to_immediate_unchecked() };
			$code
		}
	};
	(Immediate, $ctx:ident, { $($code:tt)* }) =>
	{
		{
			let $ctx = unsafe { $ctx._to_immediate_unchecked() };
			$($code)*
		}
	};
	(Immediate, let $rebind:ident = $ctx:expr, { $($code:tt)* }) =>
	{
		{
			let $rebind = $ctx;
			let $rebind = unsafe { $rebind._to_immediate_unchecked() };
			$($code)*
		}
	};

	//WaitForServer - adds server feature flag
	(WaitForServer, $ctx:ident, $code:stmt) =>
	{
		#[cfg(feature = "server")]
		{
			let $ctx = unsafe { $ctx._to_server_unchecked() };
			$code
		}
	};
	(WaitForServer, let $rebind:ident = $ctx:expr, $code:stmt) =>
	{
		#[cfg(feature = "server")]
		{
			let $rebind = $ctx; //evaluate safely first
			let $rebind = unsafe { $rebind._to_server_unchecked() };
			$code
		}
	};
	(WaitForServer, $ctx:ident, { $($code:tt)* }) =>
	{
		#[cfg(feature = "server")]
		{
			let $ctx = unsafe { $ctx._to_server_unchecked() };
			$($code)*
		}
	};
	(WaitForServer, let $rebind:ident = $ctx:expr, { $($code:tt)* }) =>
	{
		#[cfg(feature = "server")]
		{
			let $rebind = $ctx; //evaluate safely first
			let $rebind = unsafe { $rebind._to_server_unchecked() };
			$($code)*
		}
	};

	//WaitForConsensus - adds server feature flag+wraps in has_consensus() if statement
	(WaitForConsensus, $ctx:ident, $tick:expr, $code:stmt) =>
	{
		#[cfg(feature = "server")]
		{
			let _tick: &base::tick::TickInfo = $tick;
			if _tick.has_consensus()
			{
				let $ctx = unsafe { $ctx._to_consensus_unchecked() };
				$code
			}
		}
	};
	(WaitForConsensus, let $rebind:ident = $ctx:expr, $tick:expr, $code:stmt) =>
	{
		#[cfg(feature = "server")]
		{
			let _tick: &base::tick::TickInfo = $tick;
			if _tick.has_consensus()
			{
				let $rebind = $ctx; //evaluate safely first
				let $rebind = unsafe { $rebind._to_consensus_unchecked() };
				$code
			}
		}
	};
	(WaitForConsensus, $ctx:ident, $tick:expr, { $($code:tt)* }) =>
	{
		#[cfg(feature = "server")]
		{
			let _tick: &base::tick::TickInfo = $tick;
			if _tick.has_consensus()
			{
				let $ctx = unsafe { $ctx._to_consensus_unchecked() };
				$($code)*
			}
		}
	};
	(WaitForConsensus, let $rebind:ident = $ctx:expr, $tick:expr, { $($code:tt)* }) =>
	{
		#[cfg(feature = "server")]
		{
			let _tick: &base::tick::TickInfo = $tick;
			if _tick.has_consensus()
			{
				let $rebind = $ctx; //evaluate safely first
				let $rebind = unsafe { $rebind._to_consensus_unchecked() };
				$($code)*
			}
		}
	};
}
