use crate::store::blockstore::BlockStore;
use std::string::ToString;

pub enum Axis {
    X,
    Y,
    Z,
}

pub enum AxisOrder {
    XYZ,
    XZY,
    YXZ,
    YZX,
    ZXY,
    ZYX,
}

pub struct Boundary {
    min_x: i32,
    min_y: i32,
    min_z: i32,
    d_x: i32,
    d_y: i32,
    d_z: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BlockPosition {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BlockState {
    name: String,
    properties: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Block {
    pub(crate) position: BlockPosition,
    pub(crate) state: BlockState,
}

pub struct Schematic {
    block_store: dyn BlockStore,
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

    fn index(&self, pos: &BlockPosition, boundary: &Boundary) -> i32 {
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
    pub(crate) fn new(min_x: i32, min_y: i32, min_z: i32, d_x: i32, d_y: i32, d_z: i32) -> Self {
        Boundary {
            min_x,
            min_y,
            min_z,
            d_x,
            d_y,
            d_z,
        }
    }

    fn new_from_min_max(min_x: i32, min_y: i32, min_z: i32, max_x: i32, max_y: i32, max_z: i32) -> Self {
        Boundary {
            min_x,
            min_y,
            min_z,
            d_x: max_x - min_x + 1,
            d_y: max_y - min_y + 1,
            d_z: max_z - min_z + 1,
        }
    }

    fn new_from_dict(dimensions: &std::collections::HashMap<String, i32>) -> Self {
        let has_max = dimensions.contains_key("max_x")
            && dimensions.contains_key("max_y")
            && dimensions.contains_key("max_z");
        if has_max {
            Boundary::new_from_min_max(
                *dimensions.get("min_x").unwrap(),
                *dimensions.get("min_y").unwrap(),
                *dimensions.get("min_z").unwrap(),
                *dimensions.get("max_x").unwrap(),
                *dimensions.get("max_y").unwrap(),
                *dimensions.get("max_z").unwrap(),
            )
        } else {
            Boundary::new(
                *dimensions.get("min_x").unwrap(),
                *dimensions.get("min_y").unwrap(),
                *dimensions.get("min_z").unwrap(),
                *dimensions.get("d_x").unwrap(),
                *dimensions.get("d_y").unwrap(),
                *dimensions.get("d_z").unwrap(),
            )
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

    fn max_x(&self) -> i32 {
        self.min_x + self.d_x - 1
    }

    fn max_y(&self) -> i32 {
        self.min_y + self.d_y - 1
    }

    fn max_z(&self) -> i32 {
        self.min_z + self.d_z - 1
    }

    pub(crate) fn d_x(&self) -> i32 {
        self.d_x
    }

    pub(crate) fn d_y(&self) -> i32 {
        self.d_y
    }

    pub(crate) fn d_z(&self) -> i32 {
        self.d_z
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
        let new_min_x = self.min_x().min(pos.x);
        let new_min_y = self.min_y().min(pos.y);
        let new_min_z = self.min_z().min(pos.z);
        let new_max_x = self.max_x().max(pos.x);
        let new_max_y = self.max_y().max(pos.y);
        let new_max_z = self.max_z().max(pos.z);
        Boundary::new_from_min_max(
            new_min_x,
            new_min_y,
            new_min_z,
            new_max_x,
            new_max_y,
            new_max_z,
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

impl BlockPosition {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        BlockPosition { x, y, z }
    }

    fn select(&self, axis: &Axis) -> i32 {
        match axis {
            Axis::X => self.x,
            Axis::Y => self.y,
            Axis::Z => self.z,
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

impl BlockState {
    fn from_name(name: String) -> Self {
        BlockState {
            name,
            properties: vec![],
        }
    }

    pub fn new(name: String, properties: Vec<(String, String)>) -> Self {
        BlockState { name, properties }
    }

    fn is_air(&self) -> bool {
        self.name == "minecraft:air"
            || self.name == "minecraft:cave_air"
            || self.name == "minecraft:void_air"
    }

    fn to_string(&self) -> String {
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

    pub(crate) fn from_str(input: String) -> BlockState {
        if !input.contains("[") {
            return BlockState::from_name(input.trim().to_string());
        }
        let split_index = input.find("[").unwrap();
        let type_name = &input[0..split_index];
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
        BlockState::new(type_name.trim().to_string(), property_map)
    }

    fn to_json(&self) -> String {
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
    fn new(state: BlockState, position: BlockPosition) -> Self {
        Block { state, position }
    }

    pub(crate) fn air() -> Self {
        Block {
            state: BlockState::new("minecraft:air".to_string(), vec![]),
            position: BlockPosition::zero(),
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

#[cfg(test)]
mod tests {
    #[test]
    fn test_block_state_parsing() {
        let state_str = "minecraft:stone [variant= granite , hardness=1.5]";
        let block_state = super::BlockState::from_str(state_str.to_string());
        assert_eq!(block_state.name, "minecraft:stone");
        assert_eq!(block_state.properties.len(), 2);
        assert_eq!(
            block_state.properties[0],
            ("variant".to_string(), "granite".to_string())
        );
        assert_eq!(
            block_state.properties[1],
            ("hardness".to_string(), "1.5".to_string())
        );
    }
}
