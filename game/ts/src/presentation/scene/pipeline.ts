import * as Input from "@game/simulation/input.ts";
import * as Renderer from "@engine/client_ts/renderer.ts";
import type { EngineState } from "@engine/client_ts/presentation_controller.ts";

//purely client sided rendering pipeline. it should
//be able to able to render the game in any state,
//regardless of what the simulation is doing. remember
//that rollbacks/mispredicts can wipe out data that
//was already rendered in a previous frame. running
//this at 60hz is most common, but should always
//match the device's refresh rate
export function presentationTick(engine: EngineState) {
	//populate input state from poll
	Input.update();

	//ship the input off to the simulation + move/interpolate entities.
	//also internally calls the rust version of presentation_tick pipeline
	engine.rsController.presentation_tick(engine.dt, engine.rsInput);

	//this line of code might have something to do with rendering
	Renderer.render(engine.renderer);
}
