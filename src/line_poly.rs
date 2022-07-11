use bevy::math::*;

pub struct LinePoly {
    pub vertices: Vec<Vec2>,
    pub indices: Vec<u32>,
}

impl LinePoly {
    fn vertical(dir: Vec2) -> Vec2 {
        Vec2::new(dir.y, -dir.x)
    }

    pub fn from_line(line: Vec<Vec2>, w: f32) -> Self {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        if line.len() > 1 {
            vertices.reserve(line.len() * 2);
            indices.reserve((line.len() - 1) * 6);
            let mut dir = (line[1] - line[0]).normalize_or_zero();
            let mut flip = false;
            let n0 = Self::vertical(dir);
            vertices.push(line[0] - n0 * w);
            vertices.push(line[0] + n0 * w);
            for i in 1..line.len() {
                if flip {
                    let v = vertices.len() as u32 - 2;
                    indices.extend([v + 1, v, v + 2, v + 3, v + 2, v]);
                } else {
                    let v = vertices.len() as u32 - 2;
                    indices.extend([v, v + 1, v + 2, v + 3, v + 2, v + 1]);
                };
                let n;
                if i + 1 < line.len() {
                    let v = (line[i + 1] - line[i]).normalize_or_zero();
                    flip = v.dot(dir) < 0.0;
                    if flip {
                        n = Self::vertical((dir - v).normalize_or_zero());
                    } else {
                        n = Self::vertical((dir + v).normalize_or_zero());
                    };
                    dir = v;
                } else {
                    n = Self::vertical(dir);
                }
                let c = n.dot(dir);
                let w1 = w / (1.0 - c * c).sqrt();
                vertices.push(line[i] - n * w1);
                vertices.push(line[i] + n * w1);
            }
        }
        LinePoly { vertices, indices }
    }
}
