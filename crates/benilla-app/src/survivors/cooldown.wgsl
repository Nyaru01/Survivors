// Survivors cooldown: the clear sector advances clockwise from twelve o'clock.
#import bevy_ui::ui_vertex_output::UiVertexOutput
@group(1) @binding(0) var<uniform> sweep: f32;
const TAU: f32 = 6.283185307179586;
@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let remaining = clamp(sweep, 0.0, 1.0);
    if remaining <= 0.0 { return vec4<f32>(0.0); }
    if remaining >= 1.0 { return vec4<f32>(0.0, 0.0, 0.0, 0.55); }
    let p = in.uv - vec2<f32>(0.5);
    var angle = atan2(p.x, -p.y);
    angle = select(angle, angle + TAU, angle < 0.0);
    let boundary = (1.0 - remaining) * TAU;
    let alpha = smoothstep(-0.025, 0.025, angle - boundary) * 0.55;
    return vec4<f32>(0.0, 0.0, 0.0, alpha);
}
