use bevy::math::*;

pub struct LinePoly {
    pub vertices: Vec<Vec2>,
    pub indices: Vec<u32>,
}

impl LinePoly {
    fn vertical(dir: Vec2) -> Vec2 {
        Vec2::new(dir.y, -dir.x)
    }

    pub fn from_line(mut line: impl Iterator<Item = Vec2>, w: f32) -> Self {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let (line_len, _) = line.size_hint();
        let p0 = line.next();
        let p1 = line.next();
        if let (Some(p0), Some(p1)) = (p0, p1) {
            if line_len > 0 {
                vertices.reserve(line_len * 2);
                indices.reserve((line_len - 1) * 6);
            }
            let mut dir = (p1 - p0).normalize_or_zero();
            let mut flip = false;
            let n0 = Self::vertical(dir);
            vertices.push(p0 - n0 * w);
            vertices.push(p0 + n0 * w);
            let mut p_cur = p1;
            loop {
                let iv = vertices.len() as u32 - 2;
                if flip {
                    indices.extend([iv + 1, iv, iv + 2, iv + 3, iv + 2, iv]);
                } else {
                    indices.extend([iv, iv + 1, iv + 2, iv + 3, iv + 2, iv + 1]);
                };
                if let Some(p_next) = line.next() {
                    let v = (p_next - p_cur).normalize_or_zero();
                    flip = v.dot(dir) < 0.0;
                    let n = if flip {
                        Self::vertical((dir - v).normalize_or_zero())
                    } else {
                        Self::vertical((dir + v).normalize_or_zero())
                    };
                    dir = v;
                    let c = n.dot(dir);
                    let w1 = w / (1.0 - c * c).sqrt();
                    vertices.push(p_cur - n * w1);
                    vertices.push(p_cur + n * w1);
                    p_cur = p_next;
                } else {
                    let n = Self::vertical(dir);
                    vertices.push(p_cur - n * w);
                    vertices.push(p_cur + n * w);
                    break;
                }
            }
        }
        LinePoly { vertices, indices }
    }
}
