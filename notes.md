# What this Is

The learn WGPU tutorial is nice, but I want to make something factored differently, to aim in better understanding.

# What this is Not

An engine.

## Goals

- Some "smart" way to generate pipelines based on need
- Pass a list of some kind of renderable/object/model to the scene, and then you can see them rendered
- create some kind of filter stack

## End State

I would like this to be some kind of silly demo - like a vaporwave scene - with multiple materials and post processing shaders.

- obj model statue with diffuse/normal/specular and a volumetric marble shader
- some kind of environment, like a floor w/ boxes etc
- some kind of procedural camera-aligned sky quad
- post-process ray-marched atmospherics
- post process tonemapping


## Milestones

- Milestone One: DONE
Refactor the final state of the wgpu tut11 app to be something I feel comfy about
	- some way to generate a pipeline based on model/camera/instance/lighting params
	- some clear state separation for camera lighting model etc

- Milestone Two: DONE
Introduce mipmaps
	- wgpu-mipmap had a fork which might be modern enough?
	- alternatively, the `image` crate supports resizing, can use that to upload appropriate mips

- Milestone Three
Use multipass rendering to support N lights.
	- Should be possible with just extra pipelines in a single render pass since RenderPipelineDescriptor supports blend funcs
	- Should Scene just draw solid pass stuff and have a second thing (like a dedicated filter stack render pass) which draws filter passes?

- Milestone Four
Make the scene render to texture and use a simple blitter pipeline to then display that texture

- Milestone Five
Make a post process shader proof of concept

- Milestone Six
Make a post processing "stack" which can ping pong between two intermediate textures

## Presently

wgpu 0.13 is OUT
	- https://sotrh.github.io/learn-wgpu/news/0.13/
	- https://github.com/gfx-rs/wgpu/blob/master/CHANGELOG.md#wgpu-013-2022-06-30

Do I want a GUI?
	- https://github.com/emilk/egui#example

Milestone Three
- multipass lighting works
- TODO:
	- multiple lights
	- light types - directional, spot, point, can use a switch in fragment shader?
		- point light works
		- directional light works

	- camera, model, light could use a is_dirty() fn to reduce calls to update()
		- light: done
		- camera: done
		- model: done

	- the scene constructor could probably have a callback to mutate state
		- would be dispatched via update()

- THEN:
	- update to wgpu 0.13
	Note: None of wgpu 0.13 demos run. Seems to be related to debug groups, timing, etc. Perhaps vulkan extensions my GPU doesn't support.

- THEN
	- camera could use a look_at function
	- shadows!?
	https://learnopengl.com/Advanced-Lighting/Shadows/Shadow-Mapping