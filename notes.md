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

- Milestone Three: DONE
Use multipass rendering to support N lights.
	- Should be possible with just extra pipelines in a single render pass since RenderPipelineDescriptor supports blend funcs

- Milestone Four : DONE
Make the scene render to texture and use a simple blitter pipeline to then display that texture

- Milestone Five:
	- Environment map and sky

- Milestone Six:
	- Shadows
	https://learnopengl.com/Advanced-Lighting/Shadows/Shadow-Mapping

## Presently

Cube maps

	TODO:
	- DONE: create a method on texture to load
	- DONE: pass to scene, and make it an env_map field
	- DONE: render in compositor shader, using depth testing?
	- render in model shader as ambient term, and perhaps also as shiny lookup?

Material
	Material is at present pretty bare bones and needs fleshing out
	- shininess but as a power term, e.g., 1 -> 64 or whatever
	- roughness which I guess maps to skybox mip?
	- emission




## Remember

Do I want a GUI?
	- https://github.com/emilk/egui#example

uniforms require 16-byte (4 float field spacing)

- investigate migration to `tao` from winit.
	- this is on branch `replace-winit-with-tao`. it almost works, but has awful flicker.

## Ideas

We probably want an array of cameras, and to draw them in a loop, but this can wait
	- would be nice to have a "main" camera

Instead of a global ambient term, apparently having a point light centered on the camera can produce lovely results
	- doesn't need shadows
	- ref: https://www.vfxwizard.com/tutorials/gamma-correction-for-linear-workflow.html
