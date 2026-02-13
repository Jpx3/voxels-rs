use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Sub;
use std::rc::Rc;
use std::string::ToString;
use std::sync::OnceLock;

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

#[derive(Clone, Eq)]
pub struct BlockState {
    name: String,
    properties: Vec<(String, String)>,
    cached_hash: u64,
}

impl Hash for BlockState {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.cached_hash);
    }
}

impl PartialEq for BlockState {
    fn eq(&self, other: &Self) -> bool {
        self.cached_hash == other.cached_hash
            && self.name == other.name
            && self.properties == other.properties
    }
}

impl Debug for BlockState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.properties.is_empty() {
            write!(f, "{}", self.name)
        } else {
            let mut props: Vec<String> = self
                .properties
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            props.sort();
            write!(f, "{}[{}]", self.name, props.join(","))
        }
    }
}

impl Display for BlockState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.properties.is_empty() {
            write!(f, "{}", self.name)
        } else {
            let mut props: Vec<String> = self
                .properties
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            props.sort();
            write!(f, "{}[{}]", self.name, props.join(","))
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Block {
    pub position: BlockPosition,
    pub state: Rc<BlockState>,
}

impl Display for Block {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Block(state: {}, position: {})",
            self.state, self.position
        )
    }
}

impl Debug for Block {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Block")
            .field("state", &self.state)
            .field("position", &self.position)
            .finish()
    }
}

impl Display for BlockPosition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

pub trait Region {
    fn contains(&self, pos: &BlockPosition) -> bool;
    fn iter(&self, axis_order: AxisOrder) -> Box<dyn Iterator<Item = BlockPosition> + '_>;
}

impl AxisOrder {
    fn axis(&self) -> [Axis; 3] {
        match self {
            AxisOrder::XYZ => [Axis::X, Axis::Y, Axis::Z],
            AxisOrder::XZY => [Axis::X, Axis::Z, Axis::Y],
            AxisOrder::YXZ => [Axis::Y, Axis::X, Axis::Z],
            AxisOrder::YZX => [Axis::Y, Axis::Z, Axis::X],
            AxisOrder::ZXY => [Axis::Z, Axis::X, Axis::Y],
            AxisOrder::ZYX => [Axis::Z, Axis::Y, Axis::X],
        }
    }

    pub fn preferred() -> Self {
        AxisOrder::XYZ
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
}

impl Boundary {
    pub fn new(min_x: i32, min_y: i32, min_z: i32, d_x: i32, d_y: i32, d_z: i32) -> Self {
        Boundary {
            min_x,
            min_y,
            min_z,
            d_x,
            d_y,
            d_z,
        }
    }

    pub fn new_empty() -> Self {
        Boundary {
            min_x: 0,
            min_y: 0,
            min_z: 0,
            d_x: 0,
            d_y: 0,
            d_z: 0,
        }
    }

    pub fn new_from_min_max(
        min_x: i32,
        min_y: i32,
        min_z: i32,
        max_x: i32,
        max_y: i32,
        max_z: i32,
    ) -> Self {
        Boundary {
            min_x,
            min_y,
            min_z,
            d_x: max_x - min_x + 1,
            d_y: max_y - min_y + 1,
            d_z: max_z - min_z + 1,
        }
    }

    pub fn new_from_size(size_x: i32, size_y: i32, size_z: i32) -> Self {
        Boundary {
            min_x: 0,
            min_y: 0,
            min_z: 0,
            d_x: size_x,
            d_y: size_y,
            d_z: size_z,
        }
    }

    pub fn new_from_positions(min: &BlockPosition, max: &BlockPosition) -> Self {
        Boundary {
            min_x: min.x,
            min_y: min.y,
            min_z: min.z,
            d_x: max.x - min.x + 1,
            d_y: max.y - min.y + 1,
            d_z: max.z - min.z + 1,
        }
    }

    pub fn volume(&self) -> usize {
        (self.d_x * self.d_y * self.d_z) as usize
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

    pub fn max_x(&self) -> i32 {
        self.min_x + self.d_x - 1
    }

    pub fn max_y(&self) -> i32 {
        self.min_y + self.d_y - 1
    }

    pub fn max_z(&self) -> i32 {
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
            new_min_x, new_min_y, new_min_z, new_max_x, new_max_y, new_max_z,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BlockPosition {
    x: i32,
    y: i32,
    z: i32,
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
        BlockPosition::new(
            a.x().max(b.x()),
            a.y().max(b.y()),
            a.z().max(b.z()),
        )
    }

    pub fn x(&self) -> i32 {
        self.x
    }

    pub fn y(&self) -> i32 {
        self.y
    }

    pub fn z(&self) -> i32 {
        self.z
    }

    fn to_json(&self) -> String {
        format!(r#"{{"x": {}, "y": {}, "z": {}}}"#, self.x, self.y, self.z)
    }

    fn to_string(&self) -> String {
        format!("({}, {}, {})", self.x, self.y, self.z)
    }
}

impl BlockState {
    pub fn new(name: String, properties: Vec<(String, String)>) -> Self {
        let hash = BlockState::hash(&name, &properties);
        BlockState {
            name,
            properties,
            cached_hash: hash,
        }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn name_ref(&self) -> &String {
        &self.name
    }

    pub fn properties(&self) -> &Vec<(String, String)> {
        &self.properties
    }

    pub fn properties_map(&self) -> Option<HashMap<String, String>> {
        if self.properties.is_empty() {
            return None;
        }
        let mut map = HashMap::new();
        for (k, v) in &self.properties {
            map.insert(k.clone(), v.clone());
        }
        Some(map)
    }

    fn hash(name: &String, properties: &Vec<(String, String)>) -> u64 {
        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        for (k, v) in properties {
            k.hash(&mut hasher);
            v.hash(&mut hasher);
        }
        hasher.finish()
    }

    pub fn from_name(name: String) -> Self {
        BlockState::new(name, vec![])
    }

    pub fn as_ref(&self) -> &BlockState {
        self
    }

    pub fn from_name_and_properties(name: &String, properties: &HashMap<String, String>) -> Self {
        let name = name.clone();
        let props_vec: Vec<(String, String)> = properties
            .into_iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        BlockState::new(name, props_vec)
    }

    pub fn air_state_ref() -> &'static BlockState {
        static AIR: OnceLock<&'static BlockState> = OnceLock::new();

        *AIR.get_or_init(|| {
            let state = BlockState::air();
            Box::leak(Box::new(state))
        })
    }

    thread_local! {
      // Each thread initializes this once on its first access
      static AIR_RC: Rc<BlockState> = Rc::new(BlockState::air());
    }

    pub fn air_rc() -> Rc<BlockState> {
        Self::AIR_RC.with(|rc| rc.clone())
    }

    pub fn air() -> Self {
        BlockState::from_name("minecraft:air".to_string())
    }

    pub fn is_air(&self) -> bool {
        self.name == "minecraft:air"
            || self.name == "minecraft:cave_air"
            || self.name == "minecraft:void_air"
    }

    pub fn to_string(&self) -> String {
        if self.properties.is_empty() {
            self.name.clone()
        } else {
            let mut props: Vec<String> = self
                .properties
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                // sort by key alphabetically for consistent output
                .collect();
            props.sort();
            format!("{}[{}]", self.name, props.join(","))
        }
    }

    // difference string format: "new_type+prop1=val1,prop2=val2-prop3,prop4"
    // new_type is optional, if not present, type is not changed. + indicates properties to add or update, - indicates properties to remove.
    pub fn update(&self, difference: String) -> Result<BlockState, String> {
        if difference.trim().is_empty() {
            return Ok(self.clone());
        }
        if difference.len() > 4096 {
            return Err(format!(
                "Malformed difference string: length {} exceeds maximum of 4096",
                difference.len()
            ));
        }
        let mut new_name = self.name.clone();
        let difference: String = difference.chars().filter(|c| !c.is_whitespace()).collect();
        if !difference
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '+' | '-' | '=' | ':' | ','))
        {
            return Err(format!(
                "Malformed difference string: illegal character in '{}'",
                difference
            ));
        }
        let first_sign = difference.find(['+', '-']).unwrap_or(difference.len());
        let name_part = &difference[..first_sign];
        if !name_part.is_empty() {
            if name_part.matches(':').count() > 1 {
                return Err(format!(
                    "Malformed difference string: '{}' must contain at most one ':' separating namespace and type",
                    name_part
                ));
            }
            // ":abc" is valid and means "<oldnamespace>:abc"
            if name_part.starts_with(':') {
                new_name = format!("{}{}", self.name.split(':').next().unwrap_or(""), name_part);
            } else {
                new_name = name_part.to_string();
            }

            // "minecraft:" is valid and means "minecraft:<oldtype>"
            if new_name.ends_with(':') {
                new_name = format!("{}{}", new_name, self.name.split(':').last().unwrap_or(""));
            }
        }
        if new_name.len() > 64 {
            return Err("Malformed difference string: new type name too long".to_string());
        }
        let mut to_add = Vec::new();
        let mut to_remove = Vec::new();
        let mut remaining = &difference[first_sign..];
        while !remaining.is_empty() {
            let sign = &remaining[0..1];
            let next_sign = remaining[1..]
                .find(['+', '-'])
                .map(|i| i + 1)
                .unwrap_or(remaining.len());
            let segment = &remaining[1..next_sign];
            if sign == "+" {
                for pair in segment.split(',') {
                    if let Some((k, v)) = pair.split_once('=') {
                        to_add.push((k.to_string(), v.to_string()));
                        if to_add.len() > 256 {
                            return Err("Malformed difference string: too many properties to add"
                                .to_string());
                        }
                    }
                }
            } else if sign == "-" {
                for prop in segment.split(',') {
                    to_remove.push(prop.to_string());
                    if to_remove.len() > 256 {
                        return Err("Malformed difference string: too many properties to remove"
                            .to_string());
                    }
                }
            }
            remaining = &remaining[next_sign..];
        }
        let mut new_properties: Vec<(String, String)> = self
            .properties
            .iter()
            .filter(|(k, _)| !to_remove.contains(k) && !to_add.iter().any(|(add_k, _)| add_k == k))
            .cloned()
            .collect();

        new_properties.extend(to_add);
        Ok(BlockState::new(new_name, new_properties))
    }

    pub fn difference(&self, other: &BlockState) -> String {
        let mut sb = String::with_capacity(64);
        if self.name != other.name {
            let other_namespace = if let Some(idx) = other.name.find(':') {
                &other.name[..idx]
            } else {
                ""
            };
            let other_type = if let Some(idx) = other.name.find(':') {
                &other.name[idx + 1..]
            } else {
                &other.name
            };
            let self_namespace = if let Some(idx) = self.name.find(':') {
                &self.name[..idx]
            } else {
                ""
            };
            let self_type = if let Some(idx) = self.name.find(':') {
                &self.name[idx + 1..]
            } else {
                &self.name
            };

            if other_namespace != self_namespace {
                sb.push_str(other_namespace);
            }
            if (other_namespace != self_namespace) || (other_type != self_type) {
                sb.push(':');
            }
            if other_type != self_type {
                sb.push_str(other_type);
            }
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

    pub fn clone(&self) -> Self {
        BlockState::new(self.name.clone(), self.properties.clone())
    }

    pub fn from_str(input: &str) -> Result<BlockState, String> {
        BlockState::from_string(input.to_string())
    }

    pub fn from_string(input: String) -> Result<BlockState, String> {
        if !input.contains("[") {
            if input.contains("]") {
                return Err("Malformed BlockState string: missing '['".to_string());
            }
            if input.trim().is_empty() {
                return Err("Malformed BlockState string: empty input".to_string());
            }
            return Ok(BlockState::from_name(input.trim().to_string()));
        }
        if input.matches(':').count() != 1 {
            return Err(format!(
                "Malformed BlockState string: '{}' must contain exactly one ':' separating namespace and type",
                input
            ));
        }

        let split_index = input.find("[").unwrap();
        let resource_location = &input[0..split_index];

        if !resource_location
            .chars()
            .all(|c| matches!(c, 'a'..='z' | '0'..='9' | '_' | '/' | ':'))
        {
            return Err(format!(
                "Malformed BlockState string: illegal character in '{}'",
                resource_location
            ));
        }

        let namespace = resource_location.split(':').next().unwrap_or("").trim();
        let type_name = resource_location.split(':').last().unwrap_or("").trim();

        if namespace.is_empty() {
            return Err(format!(
                "Malformed BlockState string: missing namespace in '{}'",
                resource_location
            ));
        }

        if namespace != "minecraft" {
            return Err(format!(
                "Malformed BlockState string: unsupported namespace '{}', only 'minecraft' is allowed",
                namespace
            ));
        }

        if type_name.is_empty() {
            return Err(format!(
                "Malformed BlockState string: missing type name in '{}'",
                resource_location
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
            if !k
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '+' || c == '-')
            {
                return Err(format!(
                    "Malformed BlockState string: illegal character in property key '{}'",
                    k
                ));
            }
            if !v
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '+' || c == '-')
            {
                return Err(format!(
                    "Malformed BlockState string: illegal character in property value '{}'",
                    v
                ));
            }
        }
        Ok(BlockState::new(
            resource_location.trim().to_string(),
            property_map,
        ))
    }
}

impl Sub for BlockState {
    type Output = String;

    fn sub(self, rhs: Self) -> Self::Output {
        self.difference(&rhs)
    }
}

impl Block {
    pub fn new(state: Rc<BlockState>, position: BlockPosition) -> Self {
        Block { state, position }
    }

    pub fn new_at_zero(state: Rc<BlockState>) -> Self {
        Block {
            state,
            position: BlockPosition::zero(),
        }
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
        let innermost_axis = *axis_vectors.last().unwrap();
        let next_val = self.current.select(&innermost_axis) + 1;
        let limit = self.boundary.select_max(&innermost_axis);
        if next_val <= limit {
            self.current.select_set(&innermost_axis, next_val);
            return Some(result);
        }
        self.current
            .select_set(&innermost_axis, self.boundary.select_min(&innermost_axis));
        let last_axis = axis_vectors.first().unwrap();
        for axis in axis_vectors.iter().rev().skip(1) {
            let next = self.current.select(axis) + 1;
            let limit = self.boundary.select_max(axis);

            if next > limit {
                if axis == last_axis {
                    self.done = true;
                    break;
                }
                self.current
                    .select_set(axis, self.boundary.select_min(axis));
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
        let state_str = "minecraft:stone [variant= granite ,hardness   =1]";
        let block_state = super::BlockState::from_string(state_str.to_string()).unwrap();
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
    fn test_block_difference() {
        let state1 = super::BlockState::from_string(
            "minecraft:stone[variant=granite,hardness=1]".to_string(),
        )
        .unwrap();
        let state2 = super::BlockState::from_string(
            "minecraft:stone[variant=diorite,hardness=1]".to_string(),
        )
        .unwrap();
        let difference = state1 - state2;
        assert_eq!(difference, "+variant=diorite");
    }

    #[test]
    fn test_block_update() {
        let state1 = super::BlockState::from_str(
            "minecraft:stone[variant=granite,hardness=1]",
        )
        .unwrap();
        let updated_state = state1
            .update("minecraft:cobblestone  + variant = diorite+richy=nice-  hardness".to_string())
            .unwrap();
        assert_eq!(updated_state.name, "minecraft:cobblestone");
        assert_eq!(updated_state.properties.len(), 2);
        assert_eq!(
            updated_state.properties[0],
            ("variant".to_string(), "diorite".to_string())
        );
        assert_eq!(
            updated_state.properties[1],
            ("richy".to_string(), "nice".to_string())
        );
        let other_updated_state = state1
            .update("-variant+fish=false,muffin=true".to_string())
            .unwrap();
        assert_eq!(other_updated_state.name, "minecraft:stone");
        assert_eq!(other_updated_state.properties.len(), 3);
        assert!(other_updated_state
            .properties
            .contains(&("hardness".to_string(), "1".to_string())));
        assert!(other_updated_state
            .properties
            .contains(&("fish".to_string(), "false".to_string())));
        assert!(other_updated_state
            .properties
            .contains(&("muffin".to_string(), "true".to_string())));
    }

    #[test]
    fn test_illegal_block_state_parsing() {
        let state_str = "minecraft:stone variant=granite]";
        let result = super::BlockState::from_str(state_str);
        assert!(result.is_err());
    }

    #[test]
    fn test_namespace_difference_parsing() {
        let state1 = super::BlockState::from_str(
            "minecraft:stone[variant=granite,hardness=1]",
        )
        .unwrap();
        let updated_state = state1
            .update(":cobblestone  + variant = diorite".to_string())
            .unwrap();
        assert_eq!(updated_state.name, "minecraft:cobblestone");

        // test legacy namespace handling
        let updated_state2 = state1
            .update("minecraft:cobblestone  + variant = diorite".to_string())
            .unwrap();
        assert_eq!(updated_state2.name, "minecraft:cobblestone");

        // just update namespace
        let updated_state3 = state1
            .update("minecraft:  + variant = diorite".to_string())
            .unwrap();
        assert_eq!(updated_state3.name, "minecraft:stone");
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
            (1, 0, 0),
            (1, 0, 1),
            (1, 1, 0),
            (1, 1, 1),
        ];
        assert_eq!(positions, expected_positions);
    }
}
