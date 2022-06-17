# What this Is

The learn WGPU tutorial is nice, but I want to make something factored differently, to aim in better understanding.

# What this is Not

An engine.

## Goals

- Some "smart" way to generate pipelines based on need
- Pass a list of some kind of renderable/object/model to the scene, and then you can see them rendered
- create some kind of filter stack

## Milestones

- Step One
Refactor the final state of the wgpu tut11 app to be something I feel comfy about
	- some way to generate a pipeline based on model/camera/instance/lighting params
	- some clear state separation for camera lighting model etc

- Step Two
Introduce mipmaps
	- wgpu-mipmap had a fork which might be modern enough?

- Step Three
Make the scene render to texture and use a simple blitter pipeline to then display that texture

- Step Four
Make a post process shader proof of concept

- Step Five
Make a post processing "stack" which can ping pong between two intermediate textures