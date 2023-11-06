#version 330 core

layout(location = 0) in vec2 position;
layout(location = 1) in vec2 uv;
layout(location = 2) in vec4 color;

out vec4 out_color;
out vec2 out_uv;

void main()
{
	gl_Position = vec4(position, 0.0, 1.0);
    out_color = color;
    out_uv = uv;
}