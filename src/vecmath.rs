use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct V3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct V3i {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Copy, Clone)]
pub struct Ray {
    pub origin: V3,
    pub direction: V3 //should be normalized
}

pub struct IntersectionData {
    pub _ray: Ray,
    pub _voxel_data: u32,
}

#[inline(always)]
pub fn vec_add(v1: &V3, v2: &V3) -> V3 {
    V3 {
        x: v1.x + v2.x,
        y: v1.y + v2.y,
        z: v1.z + v2.z,
    }
}

#[inline(always)]
pub fn vec_sub(v1: &V3, v2: &V3) -> V3 {
    V3 {
        x: v1.x - v2.x,
        y: v1.y - v2.y,
        z: v1.z - v2.z,
    }
}

#[inline(always)]
pub fn vec_div(v1: &V3, v2: &V3) -> V3 {
    V3 {
        x: v1.x / v2.x,
        y: v1.y / v2.y,
        z: v1.z / v2.z,
    }
}

#[inline(always)]
pub fn vec_div_scal(v: &V3, n: f32) -> V3 {
    V3 {
        x: v.x / n,
        y: v.y / n,
        z: v.z / n,
    }   
}

#[inline(always)]
pub fn vec_mult_scal(v: &V3, n: f32) -> V3 {
    V3{
        x: v.x * n,
        y: v.y * n,
        z: v.z * n,
    }
}

#[inline]
pub fn vec_entry_plane(v: &V3) -> u32 {
    if v.x > v.y && v.x > v.z {
        0 //YZ plane
    } else if v.y > v.x && v.y > v.z {
        1 //XZ plane
    } else {
        2 //XY plane
    }
}

#[inline(always)]
pub fn vec_exit_plane(v: &V3) -> u32 {
    if v.x < v.y && v.x < v.z {
        0 //YZ
    } else if v.y < v.x && v.y < v.z {
        1 //XZ
    } else {
        2 //XY
    }
}

#[inline]
pub fn vec_normalize (v: &V3) -> V3 {
    let len = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt();

    if len == 0.0 {
        return V3 { x: 0.0, y: 0.0, z: 0.0 };
    }

    V3 {
        x: v.x / len, 
        y: v.y / len, 
        z: v.z / len
    }
}

#[inline(always)]
pub fn vec_crossp(v1: &V3, v2: &V3) -> V3 {
    V3 {
        x: v1.y * v2.z - v1.z * v2.y,
        y: v1.z * v2.x - v1.x * v2.z,
        z: v1.x * v2.y - v1.y * v2.x
    }
}

#[inline]
pub fn vec_floor_to_v3i(v: &V3) -> V3i {
    V3i {
        x: v.x.floor() as i32,
        y: v.y.floor() as i32,
        z: v.z.floor() as i32,
    }
}

#[inline]
pub fn vec_inv_dir_dda(v: &V3) -> V3 {
    V3 {
        x: if v.x.abs() > 1e-8 { 1.0 / v.x } else { f32::INFINITY },
        y: if v.y.abs() > 1e-8 { 1.0 / v.y } else { f32::INFINITY },
        z: if v.z.abs() > 1e-8 { 1.0 / v.z } else { f32::INFINITY },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec_math() {
        let v1 = V3 { x: 10.0, y: 20.0, z: 30.0 };
        let v2 = V3 { x: 2.0, y: 4.0, z: 5.0 };

        let added = vec_add(&v1, &v2);
        assert_eq!((added.x, added.y, added.z), (12.0, 24.0, 35.0));

        let subbed = vec_sub(&v1, &v2);
        assert_eq!((subbed.x, subbed.y, subbed.z), (8.0, 16.0, 25.0));

        let divided = vec_div(&v1, &v2);
        assert_eq!((divided.x, divided.y, divided.z), (5.0, 5.0, 6.0));

        let scaled = vec_mult_scal(&v1, 0.5);
        assert_eq!((scaled.x, scaled.y, scaled.z), (5.0, 10.0, 15.0));
    }

    #[test]
    fn test_entry_exit_planes() {
        let entry_x_max = V3 { x: 10.0, y: 5.0, z: 2.0 };
        assert_eq!(vec_entry_plane(&entry_x_max), 0); // YZ plane

        let exit_z_min = V3 { x: 20.0, y: 15.0, z: 5.0 };
        assert_eq!(vec_exit_plane(&exit_z_min), 2); // XY plane
    }
}
