use cgmath::{Vector3, Vector4};

pub fn color3<V>(color: V) -> Vector3<f32>
where
    V: Into<cgmath::Vector3<f32>>,
{
    let v: Vector3<f32> = color.into();
    Vector3::new(v.x * v.x, v.y * v.y, v.z * v.z)
}

pub fn color4<V>(color: V) -> Vector4<f32>
where
    V: Into<cgmath::Vector4<f32>>,
{
    let v: Vector4<f32> = color.into();
    Vector4::new(v.x * v.x, v.y * v.y, v.z * v.z, v.w)
}
