pub(crate) mod arraypage;

pub trait Page {
    fn load(&self, x: i32, y: i32, z: i32) -> Option<usize>;

    fn store(&mut self, x: i32, y: i32, z: i32, state: usize) -> Result<(), String>;

    fn erase(&mut self, x: i32, y: i32, z: i32) -> Result<(), String>;

    fn nnz(&self) -> usize;

    fn deep_equals(&self, other: &dyn Page) -> bool {
        if self.nnz() != other.nnz() {
            return false;
        }
        // Note: This is a naive implementation and may not be efficient for large pages.
        for x in 0.. {
            for y in 0.. {
                for z in 0.. {
                    let self_value = self.load(x, y, z);
                    let other_value = other.load(x, y, z);
                    if self_value != other_value {
                        return false;
                    }
                }
            }
        }
        true
    }
}