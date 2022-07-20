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

- Milestone Four
Make the scene render to texture and use a simple blitter pipeline to then display that texture

- Milestone Five
Make a post process shader proof of concept

- Milestone Six
Make a post processing "stack" which can ping pong between two intermediate textures

## Presently

- gamma correction
	- color_attachment should probably be linear
	- all model shader code should be linear, and return linear
	- sRGB framebuffer handles delinearization automatically so we can output linear values.

- right now we pass an `output` to render() calls. Would be smarter to pass a tuple of texture views, e.g., (color: TextureView, depth: Option<TextureView>)
	- when drawing scene, we'd pass (color_attachment.view, Some(depth_attachment.view))
	- when drawing compositor, we'd pass output.texture.create_view()...

- we probably want an array of cameras, and to draw them in a loop, but this can wait
	- would be nice to have a "main" camera

- investigate migration to `tao` from winit.

THEN we can use "camera" with depth textures to render shadows


- THEN
	- shadows!?
	https://learnopengl.com/Advanced-Lighting/Shadows/Shadow-Mapping

Do I want a GUI?
	- https://github.com/emilk/egui#example
Things and stuff and people and places....

## Remember

uniforms require 16-byte (4 float field spacing)