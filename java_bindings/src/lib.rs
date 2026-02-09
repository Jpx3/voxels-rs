mod jstreams;

use robusta_jni::bridge;
use robusta_jni::convert::{FromJavaValue, Signature, TryFromJavaValue, TryIntoJavaValue};
use robusta_jni::jni::errors::Result as JniResult;
use robusta_jni::jni::objects::{AutoLocal, JObject};
use robusta_jni::jni::objects::{GlobalRef, JFieldID};
use robusta_jni::jni::JNIEnv;
use std::collections::HashMap;
use std::sync::Arc;
use voxels_core::common::{Block, BlockPosition, BlockState, Boundary};
use voxels_core::stream::stream::{SchematicInputStream, SchematicOutputStream};

pub struct BlockInputStreamHandle {
    pub sis: Box<dyn SchematicInputStream>,
    pub jni_cache: JniCache,
}

pub struct BlockOutputStreamHandle {
    pub sos: Box<dyn SchematicOutputStream>,
    pub jni_cache: JniCache,
}

pub struct JniCache {
    // for Rust -> Java
    states: HashMap<BlockState, GlobalRef>,
    // for Java -> Rust
    reverse_states: Box<HashMap<i64, Arc<BlockState>>>,
    pub block_class: GlobalRef,
    pub block_pos_class: GlobalRef,
    pub block_pos_field: JFieldID<'static>,
    pub block_state_field: JFieldID<'static>,
    pub pos_x_field: JFieldID<'static>,
    pub pos_y_field: JFieldID<'static>,
    pub pos_z_field: JFieldID<'static>,
    pub __internal_id_field: JFieldID<'static>,
}

impl JniCache {
    pub fn init(env: &JNIEnv) -> JniResult<Self> {
        let b_class = env.find_class("de/richy/voxels/Block")?;
        let bp_class = env.find_class("de/richy/voxels/BlockPosition")?;
        let bs_class = env.find_class("de/richy/voxels/BlockState")?;

        let block_pos_field = env.get_field_id(b_class, "position", "Lde/richy/voxels/BlockPosition;")?;
        let block_state_field = env.get_field_id(b_class, "state", "Lde/richy/voxels/BlockState;")?;

        let pos_x_field = env.get_field_id(bp_class, "x", "I")?;
        let pos_y_field = env.get_field_id(bp_class, "y", "I")?;
        let pos_z_field = env.get_field_id(bp_class, "z", "I")?;

        let __internal_id_field = env.get_field_id(bs_class, "__internal_id", "J")?;

        Ok(JniCache {
            states: HashMap::new(),
            reverse_states: Box::new(HashMap::new()),
            block_class: env.new_global_ref(b_class)?,
            block_pos_class: env.new_global_ref(bp_class)?,
            block_pos_field: JFieldID::from(block_pos_field.into_inner()),
            block_state_field: JFieldID::from(block_state_field.into_inner()),
            pos_x_field: JFieldID::from(pos_x_field.into_inner()),
            pos_y_field: JFieldID::from(pos_y_field.into_inner()),
            pos_z_field: JFieldID::from(pos_z_field.into_inner()),
            __internal_id_field: JFieldID::from(__internal_id_field.into_inner()),
        })
    }

    pub fn block_state_rust_to_java<'env>(
        &mut self,
        env: &JNIEnv<'env>,
        state: &BlockState,
    ) -> JniResult<GlobalRef> {
        if let Some(global_ref) = self.states.get(state) {
            return Ok(global_ref.clone());
        }
        let jstate = state.to_jni(env)?;
        let global_ref = env.new_global_ref(jstate)?;
        self.states.insert(state.clone(), global_ref.clone());
        Ok(global_ref)
    }

    pub fn block_state_java_to_rust<'env>(
        &mut self, env: &JNIEnv<'env>,
        jstate: JObject,
    ) -> JniResult<Arc<BlockState>> {
        let internal_id = env.get_field_unchecked(
            jstate, self.__internal_id_field, "J".parse()?
        )?.j()? as i64;
        let state = self.reverse_states.entry(internal_id).or_insert_with(|| {
            Arc::new(BlockState::from_jni(env, jstate).unwrap())
        });
        Ok(state.clone())
    }

    pub fn block_position_java_to_rust(
        &mut self, env: &JNIEnv,
        jposition: JObject
    ) -> JniResult<BlockPosition> {
        let x = env.get_field_unchecked(
            jposition,
            self.pos_x_field,
            "I".parse()?
        )?.i()?;
        let y = env.get_field_unchecked(
            jposition,
            self.pos_y_field,
            "I".parse()?
        )?.i()?;
        let z = env.get_field_unchecked(
            jposition,
            self.pos_z_field,
            "I".parse()?)?.i()?;
        Ok(BlockPosition { x, y, z })
    }

    pub fn block_from_java(
        &mut self, env: &JNIEnv,
        java_block: JObject
    ) -> JniResult<Block> {
        let jposition = env.get_field_unchecked(
            java_block,
            self.block_pos_field,
            "Lde/richy/voxels/BlockPosition;".parse()?
        )?.l()?;
        let jstate = env.get_field_unchecked(
            java_block,
            self.block_state_field,
            "Lde/richy/voxels/BlockState;".parse()?
        )?.l()?;
        let position = self.block_position_java_to_rust(env, jposition)?;
        let state = self.block_state_java_to_rust(env, jstate)?;
        Ok(Block::new(state, position))
    }
}

fn override_block_position(
    env: &JNIEnv,
    jni_obj: JObject,
    block_pos: &BlockPosition,
    cache: &JniCache,
) -> JniResult<()> {
    env.set_field_unchecked(jni_obj, cache.pos_x_field, block_pos.x.into())?;
    env.set_field_unchecked(jni_obj, cache.pos_y_field, block_pos.y.into())?;
    env.set_field_unchecked(jni_obj, cache.pos_z_field, block_pos.z.into())?;
    Ok(())
}

#[bridge]
mod jni {

use std::io::{BufReader, BufWriter};use super::*;
    use crate::jstreams::{JavaInputStream, JavaOutputStream};
    use flate2::Compression;
    use robusta_jni::convert::Field;
    use robusta_jni::jni::sys::jlong;
    use voxels_core::common::{AxisOrder, Block};
    use voxels_core::stream::any_reader::AnySchematicInputStream;
    use voxels_core::stream::mojang_reader::MojangSchematicInputStream;
    use voxels_core::stream::mojang_writer::MojangSchematicOutputStream;
    use voxels_core::stream::sponge_reader::SpongeSchematicInputStream;
    use voxels_core::stream::sponge_writer::SpongeSchematicOutputStream;
    use voxels_core::stream::vxl_reader::VXLSchematicInputStream;
    use voxels_core::stream::vxl_writer::VXLSchematicOutputStream;

    #[derive(Signature, TryIntoJavaValue, TryFromJavaValue, FromJavaValue)]
    #[package(de.richy.voxels)]
    pub struct Voxels<'env: 'borrow, 'borrow> {
        #[instance]
        raw: AutoLocal<'env, 'borrow>,
    }

    impl<'env: 'borrow, 'borrow> Voxels<'env, 'borrow> {
        pub extern "jni" fn blocksFromBytes(
            env: &JNIEnv<'env>,
            input_stream: JObject<'env>,
            schematic_type: JObject<'env>,
        ) -> JniResult<JObject<'env>> {
            if input_stream.is_null() {
                env.throw_new("java/lang/NullPointerException", "Input stream is null")?;
                return Ok(JObject::null());
            }
            if schematic_type.is_null() {
                env.throw_new("java/lang/NullPointerException", "Schematic type is null")?;
                return Ok(JObject::null());
            }
            let schematic_type_str_obj = env.call_method(schematic_type, "name", "()Ljava/lang/String;", &[])?.l()?;
            let schematic_type_str: String = env.get_string(schematic_type_str_obj.into())?.into();
            let stream = JavaInputStream::new(
                env, input_stream,
            )?;
            use flate2::read::GzDecoder;
            let sis: Box<dyn SchematicInputStream> = match schematic_type_str.as_str() {
                "MOJANG" => {
                    Box::new(MojangSchematicInputStream::new(
                        BufReader::new(GzDecoder::new(stream))
                    ))
                },
                "VXL" => {
                    Box::new(VXLSchematicInputStream::new(
                        BufReader::new(GzDecoder::new(stream))
                    ))
                },
                "SPONGE" => {
                    Box::new(SpongeSchematicInputStream::new(
                        BufReader::new(GzDecoder::new(stream))
                    ))
                }
                _ => {
                    Box::new(AnySchematicInputStream::new_from_known(
                        BufReader::new(GzDecoder::new(stream))
                    ))
                }
            };
            let boxedHandle = Box::new(
                BlockInputStreamHandle {
                    sis, jni_cache: JniCache::init(env)?
                }
            );
            let ptr = Box::into_raw(boxedHandle) as jlong;
            let obj = env.new_object("de/richy/voxels/BlockInputStream", "()V", &[])?;
            env.set_field(obj.clone(), "ptr", "J", ptr.into(), )?;
            Ok(obj)
        }

        pub extern "jni" fn blocksToBytes(
            env: &JNIEnv<'env>,
            output_stream: JObject<'env>,
            schematic_type: JObject<'env>,
            boundary: JObject<'env>
        ) -> JniResult<JObject<'env>> {
            if output_stream.is_null() {
                env.throw_new("java/lang/NullPointerException", "Output stream is null")?;
                return Ok(JObject::null());
            }
            if schematic_type.is_null() {
                env.throw_new("java/lang/NullPointerException", "Schematic type is null")?;
                return Ok(JObject::null());
            }
            let schematic_type_str_obj = env.call_method(schematic_type, "name", "()Ljava/lang/String;", &[])?.l()?;
            let schematic_type_str: String = env.get_string(schematic_type_str_obj.into())?.into();
            let stream = JavaOutputStream::new(env, output_stream)?;
            let boundary_r = if !boundary.is_null() {
                Some(JNITranslation::from_jni(env, boundary)?)
            } else {
                None
            };
            use flate2::write::GzEncoder;
            let sis: Box<dyn SchematicOutputStream> = match schematic_type_str.as_str() {
                "MOJANG" => {
                    Box::new(MojangSchematicOutputStream::new(GzEncoder::new(stream, Compression::default())))
                },
                "VXL" => {
                    if boundary_r.is_none() {
                        env.throw_new("java/lang/IllegalArgumentException", "Boundary must be provided for VXL schematic type")?;
                        return Ok(JObject::null());
                    }
                    Box::new(VXLSchematicOutputStream::new(
                        BufWriter::new(
                            GzEncoder::new(stream, Compression::default())
                            // stream
                        ),
                        AxisOrder::XYZ,
                        boundary_r.unwrap()
                    ))
                },
                "SPONGE" => {
                    if boundary_r.is_none() {
                        env.throw_new("java/lang/IllegalArgumentException", "Boundary must be provided for SPONGE schematic type")?;
                        return Ok(JObject::null());
                    }
                    Box::new(SpongeSchematicOutputStream::new(
                        GzEncoder::new(stream, Compression::default()),
                        boundary_r.unwrap()
                    ))
                }
                _ => {
                    env.throw_new("java/lang/IllegalArgumentException", "Unknown schematic type")?;
                    return Ok(JObject::null());
                }
            };
            let boxedHandle = Box::new(
                BlockOutputStreamHandle {
                    sos: sis,
                    jni_cache: JniCache::init(env)?
                }
            );
            let ptr = Box::into_raw(boxedHandle) as jlong;
            let obj = env.new_object("de/richy/voxels/BlockOutputStream", "()V", &[])?;
            env.set_field(obj.clone(), "ptr", "J", ptr.into(), )?;
            Ok(obj)
        }

        pub extern "jni" fn init0(
            _env: &JNIEnv<'env>
        ) {
            // let subscriber = FmtSubscriber::builder()
            //     .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
            //     .with_max_level(Level::TRACE)
            //     .finish();
            // tracing::subscriber::set_global_default(subscriber)
            //     .expect("setting default subscriber failed");
            // info!("Voxels JNI initialized with tracing subscriber");
        }
    }


    #[derive(Signature, TryIntoJavaValue, TryFromJavaValue, FromJavaValue)]
    #[package(de.richy.voxels)]
    pub struct BlockInputStream<'env: 'borrow, 'borrow> {
        #[instance]
        raw: AutoLocal<'env, 'borrow>,

        #[field]
        ptr: Field<'env, 'borrow, jlong>,
    }

    impl<'env: 'borrow, 'borrow> BlockInputStream<'env, 'borrow> {
        #[constructor]
        pub extern "java" fn new(env: &'borrow JNIEnv<'env>) -> JniResult<Self> {}

        pub extern "jni" fn read(
            self,
            env: &JNIEnv,
            block_array: JObject<'env>,
            offset: i32, length: i32,
        ) -> JniResult<i32> {
            let ptr_value = self.ptr.get()?;
            if ptr_value == 0 {
                env.throw_new("java/io/IOException", "Stream is closed")?;
                return Ok(-1);
            }
            if block_array.is_null() {
                env.throw_new("java/lang/NullPointerException", "Block array is null")?;
                return Ok(-1);
            }
            if !env.is_instance_of(block_array, "[Lde/richy/voxels/Block;")? {
                env.throw_new("java/lang/IllegalArgumentException", "block_array is not of type Block[]")?;
                return Ok(-1);
            }
            let ptr = ptr_value as *mut BlockInputStreamHandle;
            let handle = unsafe { &mut *ptr };
            let mut blocks: Vec<Block> = Vec::with_capacity(length as usize);

            let read_result = handle.sis.read(&mut blocks, 0, length as usize);

            match read_result {
                Ok(Some(read_blocks)) => {
                    for i in 0..read_blocks {
                        let block = &blocks[i];
                        let array_index = offset + i as i32;
                        let java_block = env.get_object_array_element(
                            (*block_array).into(),
                            array_index,
                        )?;
                        if java_block.is_null() {
                            let jposition = block.position.to_jni(env)?;
                            let jstate = handle.jni_cache.block_state_rust_to_java(env, &block.state)?;
                            let jni_block = env.new_object(
                                env.find_class("de/richy/voxels/Block")?,
                                "(Lde/richy/voxels/BlockPosition;Lde/richy/voxels/BlockState;)V",
                                &[jposition.into(), jstate.as_obj().into()],
                            )?;
                            env.set_object_array_element(
                                (*block_array).into(),
                                array_index,
                                jni_block,
                            )?;
                        } else {
                            let block_position = env.get_field_unchecked(
                                java_block,
                                handle.jni_cache.block_pos_field,
                                "Lde/richy/voxels/BlockPosition;".parse()?
                            )?.l()?;
                            override_block_position(env, block_position, &block.position, &handle.jni_cache)?;
                            let jstate = handle.jni_cache.block_state_rust_to_java(env, &block.state)?;
                            env.set_field_unchecked(
                                java_block,
                                handle.jni_cache.block_state_field,
                                jstate.as_obj().into(),
                            )?;
                        }
                    }
                    Ok(read_blocks as i32)
                }
                Ok(None) => {
                    Ok(-1)
                }
                Err(e) => {
                    env.throw_new("java/io/IOException", format!("Error reading blocks: {}", e))?;
                    Ok(-1)
                }
            }
        }

        pub extern "jni" fn boundary(
            self, env: &JNIEnv<'env>,
        ) -> JniResult<JObject<'env>> {
            let ptr_value = self.ptr.get()?;
            if ptr_value == 0 {
                env.throw_new("java/io/IOException", "Stream is closed")?;
                return Ok(JObject::null());
            }
            let ptr = ptr_value as *mut BlockInputStreamHandle;
            let handle = unsafe { &mut *ptr };
            match handle.sis.boundary() {
                Ok(Some(boundary)) => {
                    Ok(boundary.to_jni(env)?)
                }
                Ok(None) => {
                    Ok(JObject::null())
                }
                Err(e) => {
                    env.throw_new("java/io/IOException", format!("Error getting boundary: {}", e))?;
                    Ok(JObject::null())
                }
            }
        }

        pub extern "jni" fn close(mut self) -> JniResult<()> {
            let ptr_value = self.ptr.get()?;
            // println!("BlockInputStream close called, ptr value: {}", ptr_value);
            if ptr_value == 0 {
                return Ok(());
            }
            self.ptr.set(0)?;
            let ptr = ptr_value as *mut BlockInputStreamHandle;
            unsafe {
                let _ = Box::from_raw(ptr);
            }
            Ok(())
        }
    }

    #[derive(Signature, TryIntoJavaValue, TryFromJavaValue, FromJavaValue)]
    #[package(de.richy.voxels)]
    pub struct BlockOutputStream<'env: 'borrow, 'borrow> {
        #[instance]
        raw: AutoLocal<'env, 'borrow>,
        #[field]
        ptr: Field<'env, 'borrow, jlong>,
    }

    impl<'env: 'borrow, 'borrow> BlockOutputStream<'env, 'borrow> {
        #[constructor]
        pub extern "java" fn new(env: &'borrow JNIEnv<'env>) -> JniResult<Self> {}

        pub extern "jni" fn write(
            self, env: &JNIEnv,
            block_array: JObject<'env>,
            offset: i32, length: i32,
        ) -> JniResult<()> {
            let ptr_value = self.ptr.get()?;
            if ptr_value == 0 {
                env.throw_new("java/io/IOException", "Stream is closed")?;
                return Ok(());
            }
            if block_array.is_null() {
                env.throw_new("java/lang/NullPointerException", "Block array is null")?;
                return Ok(());
            }
            if !env.is_instance_of(block_array, "[Lde/richy/voxels/Block;")? {
                env.throw_new("java/lang/IllegalArgumentException", "block_array is not of type Block[]")?;
                return Ok(());
            }
            let ptr = ptr_value as *mut BlockOutputStreamHandle;
            let handle = unsafe { &mut *ptr };
            let mut blocks: Vec<Block> = Vec::with_capacity(length as usize);
            for i in 0..length {
                let array_index = offset + i;
                let java_block = env.get_object_array_element(
                    (*block_array).into(),
                    array_index,
                )?;
                if java_block.is_null() {
                    env.throw_new("java/lang/NullPointerException", format!("Block at index {} is null", array_index))?;
                    return Ok(());
                }
                let block = {
                    let state_ref = handle.jni_cache.block_from_java(env, java_block)?;
                    state_ref.clone()
                };
                blocks.push(block);
            }

            // let my_span = span!(Level::INFO, "BlockOutputStream.write", length = length);
            // let _enter = my_span.enter();
            match handle.sos.write(&*blocks) {
                Ok(_) => Ok(()),
                Err(e) => {
                    env.throw_new("java/io/IOException", format!("Error writing blocks: {}", e))?;
                    Ok(())
                }
            }
        }

        pub extern "jni" fn close(
            mut self, env: &JNIEnv
        ) -> JniResult<()> {
            let ptr_value = self.ptr.get()?;
            // println!("BlockOutputStream close called, ptr value: {}", ptr_value);
            if ptr_value == 0 {
                return Ok(());
            }
            self.ptr.set(0)?;
            let ptr = ptr_value as *mut BlockOutputStreamHandle;
            unsafe {
                let mut raw = Box::from_raw(ptr);
                let write_result = raw.sos.complete();
                if let Err(e) = write_result {
                    env.throw_new("java/io/IOException", format!("Error completing output stream: {}", e))?;
                }
            }
            Ok(())
        }
    }

    #[derive(Signature, TryIntoJavaValue, TryFromJavaValue, FromJavaValue)]
    #[package(de.richy.voxels)]
    #[allow(dead_code)]
    pub struct Boundary<'env: 'borrow, 'borrow> {
        #[instance]
        raw: AutoLocal<'env, 'borrow>,
        #[field]
        minX: Field<'env, 'borrow, i32>,
        #[field]
        minY: Field<'env, 'borrow, i32>,
        #[field]
        minZ: Field<'env, 'borrow, i32>,
        #[field]
        dX: Field<'env, 'borrow, i32>,
        #[field]
        dY: Field<'env, 'borrow, i32>,
        #[field]
        dZ: Field<'env, 'borrow, i32>,
    }

     impl <'env: 'borrow, 'borrow> Boundary<'env, 'borrow> {
         #[constructor]
         pub extern "java" fn new(
             env: &'borrow JNIEnv<'env>,
             min_x: i32,
             min_y: i32,
             min_z: i32,
             d_x: i32,
             d_y: i32,
             d_z: i32,
         ) -> JniResult<Self> {}
     }

    // #[cfg(test)]
    // mod tests {
    //     use std::fs;
    //     use std::process::{Command, Stdio};
    //
    //     #[test]
    //     fn test_java() {
    //         compile_java_test_class();
    //
    //         let separator = if cfg!(target_os = "windows") {
    //             ";"
    //         } else {
    //             ":"
    //         };
    //
    //         let java_classpath = &format!(
    //             "../target/voxels.jar{}../target/java_test_classes",
    //             separator
    //         );
    //
    //         let status = Command::new("java")
    //             // inherit stdout and stderr
    //             .stdout(Stdio::inherit())
    //             .stderr(Stdio::inherit())
    //             .args([
    //                 "-cp",
    //                 java_classpath,
    //                 "-Djava.library.path=../target/release",
    //                 "TestAll",
    //             ])
    //             .status()
    //             .expect("Failed to run Java test class");
    //
    //         if !status.success() {
    //             panic!("Java test class failed!");
    //         }
    //     }
    //
    //     fn compile_java_test_class() {
    //         let java_src_dir = "java_test";
    //         let out_dir = "../target/java_test_classes";
    //         fs::create_dir_all(out_dir).unwrap();
    //         let status = Command::new("javac")
    //             .args([
    //                 "-d",
    //                 out_dir,
    //                 "-cp",
    //                 "../target/voxels.jar",
    //                 &format!("{}/TestAll.java", java_src_dir),
    //             ])
    //             .status()
    //             .expect("Failed to run javac");
    //
    //         if !status.success() {
    //             panic!("Java test class compilation failed: {:?}", status);
    //         }
    //     }
    // }
}

trait JNITranslation {
    fn to_jni<'env>(&self, env: &JNIEnv<'env>) -> JniResult<JObject<'env>>;

    fn from_jni<'env>(env: &JNIEnv<'env>, obj: JObject<'env>) -> JniResult<Self>
    where
        Self: Sized;
}

impl JNITranslation for Boundary {
    fn to_jni<'env>(&self, env: &JNIEnv<'env>) -> JniResult<JObject<'env>> {
        let class = env.find_class("de/richy/voxels/Boundary")?;
        let obj = env.new_object(
            class,
            "(IIIIII)V",
            &[
                self.min_x.into(),
                self.min_y.into(),
                self.min_z.into(),
                self.d_x.into(),
                self.d_y.into(),
                self.d_z.into(),
            ],
        )?;
        Ok(obj)
    }

    fn from_jni<'env>(env: &JNIEnv<'env>, obj: JObject<'env>) -> JniResult<Self> {
        let min_x = env.get_field(obj, "minX", "I")?.i()?;
        let min_y = env.get_field(obj, "minY", "I")?.i()?;
        let min_z = env.get_field(obj, "minZ", "I")?.i()?;
        let d_x = env.get_field(obj, "dX", "I")?.i()?;
        let d_y = env.get_field(obj, "dY", "I")?.i()?;
        let d_z = env.get_field(obj, "dZ", "I")?.i()?;
        Ok(Boundary {
            min_x, min_y, min_z,
            d_x, d_y, d_z,
        })
    }
}

impl JNITranslation for BlockPosition {
    fn to_jni<'env>(&self, env: &JNIEnv<'env>) -> JniResult<JObject<'env>> {
        let class = env.find_class("de/richy/voxels/BlockPosition")?;
        let obj = env.new_object(
            class,
            "(III)V",
            &[
                self.x.into(),
                self.y.into(),
                self.z.into(),
            ],
        )?;
        Ok(obj)
    }

    fn from_jni<'env>(env: &JNIEnv<'env>, obj: JObject<'env>) -> JniResult<Self> {
        let x = env.get_field(obj, "x", "I")?.i()?;
        let y = env.get_field(obj, "y", "I")?.i()?;
        let z = env.get_field(obj, "z", "I")?.i()?;
        Ok(BlockPosition { x, y, z })
    }
}


impl JNITranslation for BlockState {
    fn to_jni<'env>(&self, env: &JNIEnv<'env>) -> JniResult<JObject<'env>> {
        let class = env.find_class("de/richy/voxels/BlockState")?;
        let jtype_name = env.new_string(&self.name)?;
        let jproperties_class = env.find_class("java/util/HashMap")?;
        let jproperties = env.new_object(jproperties_class, "()V", &[])?;

        for (key, value) in &self.properties {
            let jkey = env.new_string(key)?;
            let jvalue = env.new_string(value)?;
            env.call_method(
                jproperties,
                "put",
                "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
                &[jkey.into(), jvalue.into()],
            )?;
        }
        let obj = env.new_object(
            class,
            "(Ljava/lang/String;Ljava/util/Map;)V",
            &[jtype_name.into(), jproperties.into()],
        )?;
        Ok(obj)
    }

    fn from_jni<'env>(env: &JNIEnv<'env>, obj: JObject<'env>) -> JniResult<Self> {
        let jtype_name = env.get_field(obj, "typeName", "Ljava/lang/String;")?.l()?;
        let name: String = env.get_string(jtype_name.into())?.into();

        let jproperties = env.get_field(obj, "properties", "Ljava/util/Map;")?.l()?;
        let jentry_set = env.call_method(jproperties, "entrySet", "()Ljava/util/Set;", &[])?.l()?;
        let jiterator = env.call_method(jentry_set, "iterator", "()Ljava/util/Iterator;", &[])?.l()?;

        let mut properties = Vec::new();
        while env.call_method(jiterator, "hasNext", "()Z", &[])?.z()? {
            let jentry = env.call_method(jiterator, "next", "()Ljava/lang/Object;", &[])?.l()?;
            let jkey = env.call_method(jentry, "getKey", "()Ljava/lang/Object;", &[])?.l()?;
            let jvalue = env.call_method(jentry, "getValue", "()Ljava/lang/Object;", &[])?.l()?;
            let key: String = env.get_string(jkey.into())?.into();
            let value: String = env.get_string(jvalue.into())?.into();
            properties.push((key, value));
        }
        Ok(BlockState {
            name, properties,
        })
    }
}