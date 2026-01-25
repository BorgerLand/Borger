# <img src="game/assets/favicon.webp" height="30"> BORGER <img src="game/assets/favicon.webp" height="30">

### _⚠️ This is an ongoing R&D project. Many essential features don't exist yet: ⚠️_

- _Documentation_
- _Triggering of VFX/SFX events_
- _Access game state from React_
- _WebSocket fallback_

[Follow my unhinged gamedev journey for updates!](https://www.youtube.com/@borgerland)

<div style="display: flex; gap: 10px;">
	<img src="readme/flintlockwood1.webp" alt="Browser-Oriented Rust Game Engine with Rancid tech stack" style="width: 49%;">
	<img src="readme/flintlockwood2.webp" alt="Browser-Oriented Rust Game Engine with Rancid tech stack" style="width: 49%;">
</div>
<br />

**Borger** is an open source, multiplayer-first game engine built from the ground up to take full advantage of the web ecosystem.

- 🕸️ Click to play instantly. No downloads, no app stores, no waiting. <2MB base bundle size
- 🛆 Three.js (3D rendering) featuring 0 overhead, 0 copy Rust bindings
- ⚛️ React, Vite, and Tailwind (Standard UI stack) featuring instant hot reload
- 🦀 Rust and WebAssembly (multithreaded game logic) featuring ~10 second recompilation time/iteration speed
- 🤖 LLM-friendly: composed of declarative frameworks that AI assistance excels at

Borger's bodacious gambit is a beginner-friendly mental model that empowers "multiplayer on rails". Write netcode-free game logic, annotate it with either "snappy" or "correct" using the magic [`multiplayer_tradeoff!()` macro](https://github.com/BorgerLand/Borger/blob/main/engine/base/src/multiplayer_tradeoff.rs), and get multiplayer for free: server authority, client prediction, rollback, and reconciliation. Relaxed determinism requirements allow for private server-only state and regular old IEEE 754 floating point. Just as Rust is known for memory safety, Borger aims to introduce multiplayer safety.

The framework:

- Applies "make impossible states unrepresentable" to networking, enforced by Rust's type system
- Prevents several classes of multiplayer cheats, desyncs, bugs, and other vulnerabilities at compile time
- Replaces brittle, willy-nilly, architecture-aware code with explicit, composable annotations declaring "when and where"
- Automatically generates both OS-native server + WASM client binaries from a unified codebase
- You couldn't write netcode spaghetti even if you tried!

Start by defining game state in a rich JSON format:

```Typescript
export default {
	items: {
		netVisibility: "Public",
		entity: true,
		type: "SlotMap",
		content: {
			//PUBLIC: all clients can see the item
			pos: { netVisibility: "Public", type: "Vec3A" },

			//PRIVATE: for server eyes only
			is_booby_trapped: { netVisibility: "Private", type: "bool" },
		},
	},
} satisfies SimulationState;
```

This auto-generates rollback machinery in the background, letting you focus on just the game logic:

```Rust
pub fn simulation_tick(ctx: &mut GameContext<Immediate>) {
	//player walks onto a jump pad - must feel responsive
	multiplayer_tradeoff!(Immediate, ctx, {
		if player_touched_jump_pad(ctx) {
			//instant boing, ideal for platformer game feel
			launch_player_upward(ctx);
		}
	});

	//player picks up an item - must validate on the server
	multiplayer_tradeoff!(WaitForServer, ctx, {
		if player_touched_item(ctx) {
			if item_is_booby_trapped(ctx) {
				//booby trap state is private and server-only, preventing
				//clients from accessing secrets. hence WaitForServer!
				kill_player(ctx);
			} else {
				//the tradeoff now is that there will be a short delay before
				//the player sees the item picking up. Waiting For Server!
				give_player_item(ctx);
			}
		}
	});

	//different multiplayer_tradeoffs can co-exist in the same function,
	//or be nested inside each other. always cheat-proof and fully composable
}
```

And the visuals are just plain ol' three.js:

```Typescript
import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";
import { DirectionalLight, type Object3D } from "three";

scene.add(new DirectionalLight(0xffffff, 1));

const loader = new GLTFLoader();
const itemModel = await loader.loadAsync("item.glb");

//this callback happens automatically when you call
//ctx.state.items.add(diff);
//in rust
export function spawnItem() {
	return itemModel.scene.clone();
}
```

### lol why

Practically speaking, I built Borger to power [Borger Land](https://borger.land), a for-profit web portal of absurdist comedy video games satirizing food culture. Borger Land will also double as a showcase for what its engine is capable of. Personally, I believe that tools for creative expression should be free, accessible, and make my résumé look good.

Technically speaking, I am fully aware the stack is ~~truly rancid~~ unconventional and requires some beast mode polyglotting. Yeah, she's not like the other girls. But in her defense:

- Needing to have familiarity with multiple languages is the norm in the gamedev world, especially for indie. Take Unreal for example: Blueprints for game logic, C++ for engine tweaks, HLSL for shaders, maybe a Typescript+Postgres backend service. Borger defines its split where it makes sense for web: simulation (Rust) and presentation (Typescript).
- On the client side, resimulating each tick multiple times due to rollback × (game logic + binary diff serialization) + in the same thread as three.js + written in javascript = [really stankin' slow](https://gamecreatorsclub.com/blog/deterministic-lockstep#:~:text=serializing%20to%20a%20byte%20array%20%28too%20much%20of%20a%20perf%20hit%29). It matters on the server side, too, because hosting costs YOU money every month, and Bun-based game servers require [significantly more RAM](https://en.wikipedia.org/wiki/2024%E2%80%932026_global_memory_supply_shortage).
- Rust gamedev can only mature if it is willing to accept that Rust isn't the perfect solution to every problem. React+Vite [conquered](https://trends.builtwith.com/javascript/React) the UI world for a reason. Rust efficiently tackles the concern of game logic using only basic C-like getters and setters syntax. No lifetimes, traits, ECS queries, etc. required. Keep it simple; let generalists win. We've got games to make.

```Rust
let old_pos = character.get_pos();
let new_pos = old_pos + input.omnidir * SPEED * TickInfo::SIM_DT;
character.set_pos(new_pos, diff);
```

- Most importantly: science isn't about why; it's about why not.

### Getting started:

- Required technomologies
    - [Git](https://git-scm.com/install/)
    - [Rustup](https://rustup.rs/)
    - [Bun](https://bun.com/)
    - [Something capable of running Bash scripts](https://xubuntu.org/download/) (Windows victims use [WSL](https://learn.microsoft.com/en-us/windows/wsl/install))
    - [IDE](https://code.visualstudio.com/Download) (though even a text editor will do!)
- Recommended
    - VS Code extensions:
        - [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer) (this uses a ton of RAM - recommend having at least 12 GB)
        - [ESLint](https://marketplace.visualstudio.com/items?itemName=dbaeumer.vscode-eslint)
        - [Prettier](https://marketplace.visualstudio.com/items?itemName=esbenp.prettier-vscode)
        - To automatically format code each time you save, after running `setup.sh` add to `.vscode/settings.json`:
            ```JSON
            "editor.formatOnSave": true,
            "editor.defaultFormatter": "esbenp.prettier-vscode",
            "[rust]": {
            	"editor.defaultFormatter": "rust-lang.rust-analyzer"
            },
            ```
    - Debugging Rust code in browser devtools:
        - [Chromium](https://chromewebstore.google.com/detail/cc++-devtools-support-dwa/pdcpmagijalfljmkmjngeonclgbbannb)
        - [Firefox (unpleasant but supposedly doable)](https://github.com/jdmichaud/dwarf-2-sourcemap)
        - Safari (lol)

### Make 'em move hunny

Fork this repo first, in order to use it as a blank template. Then:

```Bash
git clone https://github.com/Username/MyGame.git
cd MyGame
./setup.sh
./dev.sh
#wait a few seconds for it to stop spamming the console
```

Now visit http://localhost:5173 for a good meal

![4 Clients](readme/itworks.webp)

Files of interest:

- `game/ts/src/Index.ts` - Client entry point
- `game/State.ts` - Defines the data structure representing the entire networked scene/world
- `game/rs/src/simulation/pipeline.rs` - Entry point for the game logic simulation loop (30Hz fixed)
- `game/ts/src/presentation/Pipeline.ts` - Entry point for the rendering loop (VSync)

### Acknowledgements & Inspirations

- [Fast-Paced Multiplayer](https://www.gabrielgambetta.com/client-server-game-architecture.html) - Gabriel Gambetta
- ['Overwatch' Gameplay Architecture and Netcode](https://www.gdcvault.com/play/1024001/-Overwatch-Gameplay-Architecture-and) - Timothy Ford
- [Photon Quantum](https://doc.photonengine.com/quantum/current/quantum-intro) - Exit Games
- [Quake 3 Network Protocol](https://www.jfedor.org/quake3/) - John Carmack, Jacek Fedoryński
- [Dealing with Latency](https://docs.unity3d.com/Packages/com.unity.netcode.gameobjects@2.5/manual/learn/dealing-with-latency.html) - Unity 3D
- [Source Multiplayer Networking](http://developer.valvesoftware.com/wiki/Source_Multiplayer_Networking) - Valve
- [Tribes/Torque network model](https://www.gamedevs.org/uploads/tribes-networking-model.pdf) - Mark Frohnmayer, Tim Gift
- http://dek.engineer/ - Insights from a colleague of mine
