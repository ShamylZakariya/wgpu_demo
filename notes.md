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

- Milestone Two
Introduce mipmaps
	- wgpu-mipmap had a fork which might be modern enough?
	- alternatively, the `image` crate supports resizing, can use that to upload appropriate mips

- Milestone Three
Make the scene render to texture and use a simple blitter pipeline to then display that texture

- Milestone Four
Make a post process shader proof of concept

- Milestone Five
Make a post processing "stack" which can ping pong between two intermediate textures

## Presently

Cleanups?

	- veirfy correct behavior if mipmaps are disabled
	- clippy says we have functions with too many arguments. Make a params struct with ..Default() spread?
	- ???
