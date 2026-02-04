use std::collections::HashMap;
use std::string::ToString;
use std::sync::{Arc, OnceLock};

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Axis {
    X,
    Y,
    Z,
}

/// The order in which axes are iterated or indexed.
/// For example, `XYZ` means X is the outermost axis, then Y, then Z is the innermost axis.
/// So in `XYZ` order, X changes the slowest, and Z changes the fastest.

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum AxisOrder {
    XYZ,
    XZY,
    YXZ,
    YZX,
    ZXY,
    ZYX,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Boundary {
    pub min_x: i32,
    pub min_y: i32,
    pub min_z: i32,
    pub d_x: i32,
    pub d_y: i32,
    pub d_z: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BlockState {
    pub name: String,
    pub properties: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Block {
    pub state: Arc<BlockState>,
    pub position: BlockPosition,
}


pub trait Region {
    fn contains(&self, pos: &BlockPosition) -> bool;
    fn iter(&self, axis_order: AxisOrder) -> Box<dyn Iterator<Item = BlockPosition> + '_>;
}

impl Axis {
    fn to_string(&self) -> &str {
        match self {
            Axis::X => "X",
            Axis::Y => "Y",
            Axis::Z => "Z",
        }
    }
}

impl AxisOrder {
    fn axis(&self) -> Vec<Axis> {
        match self {
            AxisOrder::XYZ => vec![Axis::X, Axis::Y, Axis::Z],
            AxisOrder::XZY => vec![Axis::X, Axis::Z, Axis::Y],
            AxisOrder::YXZ => vec![Axis::Y, Axis::X, Axis::Z],
            AxisOrder::YZX => vec![Axis::Y, Axis::Z, Axis::X],
            AxisOrder::ZXY => vec![Axis::Z, Axis::X, Axis::Y],
            AxisOrder::ZYX => vec![Axis::Z, Axis::Y, Axis::X],
        }
    }

    pub fn index(&self, pos: &BlockPosition, boundary: &Boundary) -> i32 {
        let mut index = 0;
        for axis in self.axis() {
            let coord = match axis {
                Axis::X => pos.x - boundary.min_x,
                Axis::Y => pos.y - boundary.min_y,
                Axis::Z => pos.z - boundary.min_z,
            };
            let dim = match axis {
                Axis::X => boundary.d_x,
                Axis::Y => boundary.d_y,
                Axis::Z => boundary.d_z,
            };
            index = index * dim + coord;
        }
        index
    }

    fn compare(&self, a: &BlockPosition, b: &BlockPosition) -> std::cmp::Ordering {
        for axis in self.axis() {
            let a_value: i32 = a.select(&axis);
            let b_value: i32 = b.select(&axis);
            if a_value != b_value {
                return i32::cmp(&a_value, &b_value);
            }
        }
        std::cmp::Ordering::Equal
    }

    fn to_string(&self) -> &str {
        match self {
            AxisOrder::XYZ => "XYZ",
            AxisOrder::XZY => "XZY",
            AxisOrder::YXZ => "YXZ",
            AxisOrder::YZX => "YZX",
            AxisOrder::ZXY => "ZXY",
            AxisOrder::ZYX => "ZYX",
        }
    }
}

impl Boundary {
    pub fn new(min_x: i32, min_y: i32, min_z: i32, d_x: i32, d_y: i32, d_z: i32) -> Self {
        Boundary { min_x, min_y, min_z, d_x, d_y, d_z, }
    }

    pub fn new_empty() -> Self {
        Boundary {
            min_x: 0, min_y: 0, min_z: 0,
            d_x: 0, d_y: 0, d_z: 0,
        }
    }

    pub(crate) fn new_from_min_max(min_x: i32, min_y: i32, min_z: i32, max_x: i32, max_y: i32, max_z: i32) -> Self {
        Boundary {
            min_x, min_y, min_z,
            d_x: max_x - min_x + 1,
            d_y: max_y - min_y + 1,
            d_z: max_z - min_z + 1,
        }
    }

    fn new_from_positions(min: &BlockPosition, max: &BlockPosition) -> Self {
        Boundary {
            min_x: min.x,
            min_y: min.y,
            min_z: min.z,
            d_x: max.x - min.x + 1,
            d_y: max.y - min.y + 1,
            d_z: max.z - min.z + 1,
        }
    }

    fn volume(&self) -> i32 {
        self.d_x * self.d_y * self.d_z
    }

    fn min_x(&self) -> i32 {
        self.min_x
    }

    fn min_y(&self) -> i32 {
        self.min_y
    }

    fn min_z(&self) -> i32 {
        self.min_z
    }

    fn select_min(&self, axis: &Axis) -> i32 {
        match axis {
            Axis::X => self.min_x,
            Axis::Y => self.min_y,
            Axis::Z => self.min_z,
        }
    }

    pub(crate) fn max_x(&self) -> i32 {
        self.min_x + self.d_x - 1
    }

    pub(crate) fn max_y(&self) -> i32 {
        self.min_y + self.d_y - 1
    }

    pub(crate) fn max_z(&self) -> i32 {
        self.min_z + self.d_z - 1
    }

    fn select_max(&self, axis: &Axis) -> i32 {
        match axis {
            Axis::X => self.max_x(),
            Axis::Y => self.max_y(),
            Axis::Z => self.max_z(),
        }
    }

    pub fn d_x(&self) -> i32 {
        self.d_x
    }

    pub fn d_y(&self) -> i32 {
        self.d_y
    }

    pub fn d_z(&self) -> i32 {
        self.d_z
    }

    pub fn size_as_array(&self) -> [i32; 3] {
        [self.d_x, self.d_y, self.d_z]
    }

    pub fn size_as_i16_array(&self) -> [i16; 3] {
        [self.d_x as i16, self.d_y as i16, self.d_z as i16]
    }

    pub fn size_as_vector(&self) -> Vec<i32> {
        vec![self.d_x, self.d_y, self.d_z]
    }

    fn select_size(&self, axis: &Axis) -> i32 {
        match axis {
            Axis::X => self.d_x,
            Axis::Y => self.d_y,
            Axis::Z => self.d_z,
        }
    }

    pub fn contains(&self, pos: &BlockPosition) -> bool {
        pos.x >= self.min_x
            && pos.x < self.min_x + self.d_x
            && pos.y >= self.min_y
            && pos.y < self.min_y + self.d_y
            && pos.z >= self.min_z
            && pos.z < self.min_z + self.d_z
    }

    pub fn expand_to_include(&self, pos: &BlockPosition) -> Boundary {
        if self.contains(pos) {
            return *self;
        }
        let new_min_x = self.min_x().min(pos.x);
        let new_min_y = self.min_y().min(pos.y);
        let new_min_z = self.min_z().min(pos.z);
        let new_max_x = self.max_x().max(pos.x);
        let new_max_y = self.max_y().max(pos.y);
        let new_max_z = self.max_z().max(pos.z);
        Boundary::new_from_min_max(
            new_min_x, new_min_y, new_min_z,
            new_max_x, new_max_y, new_max_z,
        )
    }

    fn to_json(&self) -> String {
        format!(
            r#"{{"min_x": {}, "min_y": {}, "min_z": {}, "d_x": {}, "d_y": {}, "d_z": {}}}"#,
            self.min_x, self.min_y, self.min_z, self.d_x, self.d_y, self.d_z
        )
    }

    fn to_string(&self) -> String {
        format!(
            "Boundary(min: ({}, {}, {}), dimensions: ({}, {}, {}))",
            self.min_x, self.min_y, self.min_z, self.d_x, self.d_y, self.d_z
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BlockPosition {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockPosition {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        BlockPosition { x, y, z }
    }

    pub fn to_array(&self) -> [i32; 3] {
        [self.x, self.y, self.z]
    }

    fn select(&self, axis: &Axis) -> i32 {
        match axis {
            Axis::X => self.x,
            Axis::Y => self.y,
            Axis::Z => self.z,
        }
    }

    fn select_mut(&mut self, axis: &Axis) -> &mut i32 {
        match axis {
            Axis::X => &mut self.x,
            Axis::Y => &mut self.y,
            Axis::Z => &mut self.z,
        }
    }

    fn select_set(&mut self, axis: &Axis, value: i32) {
        match axis {
            Axis::X => self.x = value,
            Axis::Y => self.y = value,
            Axis::Z => self.z = value,
        }
    }

    const fn zero() -> Self {
        BlockPosition { x: 0, y: 0, z: 0 }
    }

    fn min(a: &BlockPosition, b: &BlockPosition) -> BlockPosition {
        BlockPosition {
            x: a.x.min(b.x),
            y: a.y.min(b.y),
            z: a.z.min(b.z),
        }
    }

    fn max(a: &BlockPosition, b: &BlockPosition) -> BlockPosition {
        BlockPosition {
            x: a.x.max(b.x),
            y: a.y.max(b.y),
            z: a.z.max(b.z),
        }
    }

    fn to_json(&self) -> String {
        format!(r#"{{"x": {}, "y": {}, "z": {}}}"#, self.x, self.y, self.z)
    }

    fn to_string(&self) -> String {
        format!("({}, {}, {})", self.x, self.y, self.z)
    }
}


static AIR: OnceLock<Arc<BlockState>> = OnceLock::new();

impl BlockState {
    pub fn from_name(name: String) -> Self {
        BlockState {
            name,
            properties: vec![],
        }
    }

    pub fn as_ref(&self) -> &BlockState {
        self
    }

    pub fn from_name_and_properties(name: &String, properties: &HashMap<String, String>) -> Self {
        let name = name.clone();
        let props_vec: Vec<(String, String)> = properties.into_iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        BlockState { name, properties: props_vec }
    }

    pub fn new(name: String, properties: Vec<(String, String)>) -> Self {
        BlockState { name, properties }
    }

    pub fn air_state_ref<'a>() -> &'a BlockState {
        AIR.get_or_init(|| Arc::new(BlockState::air())).as_ref()
    }

    pub fn air_arc() -> Arc<BlockState> {
        AIR.get_or_init(|| Arc::new(BlockState::air())).clone()
    }

    pub fn air() -> Self {
        BlockState {
            name: "minecraft:air".to_string(),
            properties: vec![],
        }
    }

    pub fn is_air(&self) -> bool {
        self.name == "minecraft:air"
            || self.name == "minecraft:cave_air"
            || self.name == "minecraft:void_air"
    }

    pub fn to_string(&self) -> String {
        if self.properties.is_empty() {
            self.name.clone() + "[]"
        } else {
            let props: Vec<String> = self
                .properties
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            format!("{}[{}]", self.name, props.join(","))
        }
    }

    pub fn update(&self, difference: &String) -> Result<Self, String> {
        let mut new_type_name = self.name.clone();
        let mut new_properties: HashMap<String, String> = self
            .properties
            .iter()
            .cloned()
            .collect();
        if difference.trim().is_empty() {
            return Ok(self.clone());
        }
        for part in difference.split("(?=[+-])") {
            let op = part.chars().next().unwrap_or('\0');
            if op == '+' {
                for pair in part[1..].split(",") {
                    let kv: Vec<&str> = pair.splitn(2, '=').collect();
                    if kv.len() == 2 {
                        new_properties.insert(kv[0].to_string(), kv[1].to_string());
                    }  else {
                        return Err(format!("Malformed property addition: '{}'", pair));
                    }
                }
            } else if op == '-' {
                for key in part[1..].split(",") {
                    new_properties.remove(key);
                }
            } else if !part.is_empty() {
                new_type_name = part.to_string();
            }
        }
        Ok(BlockState::from_name_and_properties(&new_type_name, &new_properties))
    }
    pub fn difference(&self, other: &BlockState) -> String {
        let mut sb = String::with_capacity(64);
        if self.name != other.name {
            sb.push_str(&other.name);
        }
        let mut first_update = true;
        for (k, v) in &other.properties {
            let existing = self.properties.iter().find(|(sk, _)| sk == k);
            if existing.map_or(true, |(_, sv)| sv != v) {
                if first_update {
                    sb.push('+');
                    first_update = false;
                } else {
                    sb.push(',');
                }
                sb.push_str(k);
                sb.push('=');
                sb.push_str(v);
            }
        }
        let mut first_removal = true;
        for (k, _) in &self.properties {
            // If key is not in 'other', it was removed
            if !other.properties.iter().any(|(ok, _)| ok == k) {
                if first_removal {
                    sb.push('-');
                    first_removal = false;
                } else {
                    sb.push(',');
                }
                sb.push_str(k);
            }
        }
        sb
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn properties(&self) -> Option<HashMap<String, String>> {
        if self.properties.is_empty() {
            return None;
        }
        let mut map = HashMap::new();
        for (k, v) in &self.properties {
            map.insert(k.clone(), v.clone());
        }
        Some(map)
    }

    pub fn clone(&self) -> Self {
        BlockState {
            name: self.name.clone(),
            properties: self.properties.clone(),
        }
    }

    pub fn from_str(input: String) -> Result<BlockState, String> {
        if !input.contains("[") {
            if input.contains("]") {
                return Err("Malformed BlockState string: missing '['".to_string());
            }
            if input.trim().is_empty() {
                return Err("Malformed BlockState string: empty input".to_string());
            }
            return Ok(BlockState::from_name(input.trim().to_string()));
        }
        let split_index = input.find("[").unwrap();
        let type_name = &input[0..split_index];
        let raw_type_name = if let Some(stripped) = type_name.strip_prefix("minecraft:") {
            stripped.trim()
        } else {
            return Err(format!(
                "Malformed BlockState string: '{}' must start with 'minecraft:'",
                type_name
            ));
        };
        if !raw_type_name.chars().all(|c| matches!(c, 'a'..='z' | '0'..='9' | '_' | '/' | ':')) {
            return Err(format!(
                "Malformed BlockState string: illegal character in path '{}'",
                raw_type_name
            ));
        }
        let type_properties_string = &input[split_index + 1..];
        let property_map = if type_properties_string == "]" {
            vec![]
        } else {
            type_properties_string[0..type_properties_string.len() - 1]
                .split(",")
                .map(|kv| {
                    if kv.is_empty() || !kv.contains("=") {
                        return ("".to_string(), "".to_string());
                    }
                    let mut kv_iter = kv.split("=");
                    let key = kv_iter.next().unwrap().trim().to_string();
                    let value = kv_iter.next().unwrap().trim().to_string();
                    (key, value)
                })
                .collect()
        };
        for (k, v) in &property_map {
            if k.is_empty() || v.is_empty() {
                return Err("Malformed BlockState string: empty property key or value".to_string());
            }
            if !k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '+' || c == '-') {
                return Err(format!(
                    "Malformed BlockState string: illegal character in property key '{}'",
                    k
                ));
            }
            if !v.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '+' || c == '-') {
                return Err(format!(
                    "Malformed BlockState string: illegal character in property value '{}'",
                    v
                ));
            }
        }
        Ok(BlockState::new(type_name.trim().to_string(), property_map))
    }

    pub fn to_json(&self) -> String {
        let props: Vec<String> = self
            .properties
            .iter()
            .map(|(k, v)| format!(r#""{}": "{}""#, k, v))
            .collect();
        format!(
            r#"{{"name": "{}", "properties": {{{}}}}}"#,
            self.name,
            props.join(", ")
        )
    }
}

impl Block {
    pub fn new(state: Arc<BlockState>, position: BlockPosition) -> Self {
        Block {
            state,
            position
        }
    }

    pub fn new_at_zero(state: Arc<BlockState>) -> Self {
        Block {
            state, position: BlockPosition::zero(),
        }
    }

    fn to_json(&self) -> String {
        format!(
            r#"{{"state": {}, "position": {}}}"#,
            self.state.to_json(),
            self.position.to_json()
        )
    }

    fn to_string(&self) -> String {
        format!(
            "Block(state: {}, position: {})",
            self.state.to_string(),
            self.position.to_string()
        )
    }
}


impl Region for Boundary {
    fn contains(&self, pos: &BlockPosition) -> bool {
        self.contains(pos)
    }

    fn iter(&self, axis_order: AxisOrder) -> Box<dyn Iterator<Item = BlockPosition> + '_> {
        Box::new(BoundaryIterator {
            boundary: self,
            axis_order,
            current: BlockPosition::new(self.min_x, self.min_y, self.min_z),
            done: false,
        })
    }
}

struct BoundaryIterator<'a> {
    boundary: &'a Boundary,
    axis_order: AxisOrder,
    current: BlockPosition,
    done: bool,
}


impl Iterator for BoundaryIterator<'_> {
    type Item = BlockPosition;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let result = self.current;
        let axis_vectors = self.axis_order.axis();
        // last since we will reverse the literal axis order
        let last_axis = axis_vectors.first().unwrap();
        //                          ----------vvvv------- here!
        for axis in axis_vectors.iter().rev() {
            let next = self.current.select(axis) + 1;
            let limit = self.boundary.select_max(axis);
            if next > limit {
                if axis == last_axis {
                    self.done = true;
                    break;
                }
                self.current.select_set(axis, self.boundary.select_min(axis));
            } else {
                self.current.select_set(axis, next);
                break;
            }
        }
        Some(result)
    }
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let axes = self.axis_order.axis();
        let dims = axes.len();
        let mut lengths = vec![0usize; dims];
        let mut mins = vec![0i32; dims];
        for i in 0..dims {
            mins[i] = self.boundary.select_min(&axes[i]);
            lengths[i] = (self.boundary.select_max(&axes[i]) - mins[i] + 1) as usize;
        }
        let mut strides = vec![1usize; dims];
        for i in (0..dims - 1).rev() {
            strides[i] = strides[i + 1] * lengths[i + 1];
        }
        let total_size = strides[0] * lengths[0];
        let mut current_index = 0usize;
        for i in 0..dims {
            let val = (self.current.select(&axes[i]) - mins[i]) as usize;
            current_index += val * strides[i];
        }
        let target_index = current_index + n;
        if target_index >= total_size {
            self.done = true;
            return None;
        }
        let reconstruct = |idx: usize| -> BlockPosition {
            let mut pos = self.current;
            let mut running_idx = idx;
            for i in 0..dims {
                let coord = running_idx / strides[i];
                pos.select_set(&axes[i], (coord as i32) + mins[i]);
                running_idx %= strides[i];
            }
            pos
        };
        let result = reconstruct(target_index);
        if target_index + 1 >= total_size {
            self.done = true;
        } else {
            self.current = reconstruct(target_index + 1);
        }
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use crate::common::Region;

    #[test]
    fn test_block_state_parsing() {
        let state_str = "minecraft:stone [variant= granite , hardness=1]";
        let block_state = super::BlockState::from_str(state_str.to_string()).unwrap();
        assert_eq!(block_state.name, "minecraft:stone");
        assert_eq!(block_state.properties.len(), 2);
        assert_eq!(
            block_state.properties[0],
            ("variant".to_string(), "granite".to_string())
        );
        assert_eq!(
            block_state.properties[1],
            ("hardness".to_string(), "1".to_string())
        );
    }

    #[test]
    fn test_illegal_block_state_parsing() {
        let state_str = "minecraft:stone variant=granite]";
        let result = super::BlockState::from_str(state_str.to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_indexing() {
        let boundary = super::Boundary::new(0, 0, 0, 4, 4, 4);
        let pos = super::BlockPosition::new(2, 1, 3);
        let index_xyz = super::AxisOrder::XYZ.index(&pos, &boundary);
        let index_yzx = super::AxisOrder::YZX.index(&pos, &boundary);
        assert_eq!(index_xyz, 2 * 16 + 1 * 4 + 3); // 2*16 + 1*4 + 3 = 35
        assert_eq!(index_yzx, 1 * 16 + 3 * 4 + 2); // 1*16 + 3*4 + 2 = 30
    }

    #[test]
    fn test_boundary_iterator() {
        let boundary = super::Boundary::new(0, 0, 0, 2, 2, 2);
        let iter = boundary.iter(super::AxisOrder::XYZ);
        let mut iter2 = iter.skip(4);
        let mut positions = vec![];
        while let Some(pos) = iter2.next() {
            positions.push((pos.x, pos.y, pos.z));
        }
        let expected_positions = vec![
            // (0, 0, 0), (0, 0, 1),
            // (0, 1, 0), (0, 1, 1),
            (1, 0, 0), (1, 0, 1),
            (1, 1, 0), (1, 1, 1),
        ];
        assert_eq!(positions, expected_positions);
    }
}
