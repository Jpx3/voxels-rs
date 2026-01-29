mod javastreams;

use std::collections::HashMap;
use robusta_jni::jni::objects::{GlobalRef, JFieldID};
use robusta_jni::bridge;
use robusta_jni::convert::{FromJavaValue, Signature, TryFromJavaValue, TryIntoJavaValue};
use robusta_jni::jni::errors::Error as JniError;
use robusta_jni::jni::errors::Result as JniResult;
use robusta_jni::jni::objects::{AutoLocal, JObject};
use robusta_jni::jni::JNIEnv;
use voxels_core::common::{Block, BlockPosition, BlockState};
use voxels_core::stream::SchematicInputStream;
use crate::jni::BlockInputStream;

trait JNITranslation {
    fn to_jni<'env>(&self, env: &JNIEnv<'env>) -> JniResult<JObject<'env>>;

    fn from_jni<'env>(env: &JNIEnv<'env>, obj: JObject<'env>) -> JniResult<Self>
    where
        Self: Sized;
}

pub struct BlockInputStreamHandle {
    pub sis: Box<dyn SchematicInputStream>,
    pub jni_cache: JniCache,
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
        Err(JniError::NullPtr("BlockState from_jni not implemented".into()))
    }
}

/*

public class Block {
  private BlockPosition position;
  private BlockState state;

  public Block(BlockPosition position, BlockState state) {
    this.position = position;
    this.state = state;
  }

  public BlockPosition position() {
    return position;
  }

  public BlockState state() {
    return state;
  }
}
 */

impl JNITranslation for Block<'_> {
    fn to_jni<'env>(&self, env: &JNIEnv<'env>) -> JniResult<JObject<'env>> {
        let class = env.find_class("de/richy/voxels/Block")?;
        let jposition = self.position.to_jni(env)?;
        let jstate = self.state.to_jni(env)?;
        let obj = env.new_object(
            class,
            "(Lde/richy/voxels/BlockPosition;Lde/richy/voxels/BlockState;)V",
            &[jposition.into(), jstate.into()],
        )?;
        Ok(obj)
    }

    fn from_jni<'env>(env: &JNIEnv<'env>, obj: JObject<'env>) -> JniResult<Self> {
        Err(JniError::NullPtr("Block from_jni not implemented".into()))
    }
}


pub struct JniCache {
    // Maps a Rust BlockState to a Java BlockState object
    states: HashMap<BlockState, GlobalRef>,
    pub block_class: GlobalRef,
    pub block_pos_class: GlobalRef,
    pub block_pos_field: JFieldID<'static>,
    pub block_state_field: JFieldID<'static>,
    pub pos_x_field: JFieldID<'static>,
    pub pos_y_field: JFieldID<'static>,
    pub pos_z_field: JFieldID<'static>,
}

impl JniCache {
    pub fn init(env: &JNIEnv) -> JniResult<Self> {
        let b_class = env.find_class("de/richy/voxels/Block")?;
        let bp_class = env.find_class("de/richy/voxels/BlockPosition")?;

        let block_pos_field = env.get_field_id(b_class, "position", "Lde/richy/voxels/BlockPosition;")?;
        let block_state_field = env.get_field_id(b_class, "state", "Lde/richy/voxels/BlockState;")?;

        let pos_x_field = env.get_field_id(bp_class, "x", "I")?;
        let pos_y_field = env.get_field_id(bp_class, "y", "I")?;
        let pos_z_field = env.get_field_id(bp_class, "z", "I")?;

        Ok(JniCache {
            states: HashMap::new(),
            block_class: env.new_global_ref(b_class)?,
            block_pos_class: env.new_global_ref(bp_class)?,
            block_pos_field: JFieldID::from(block_pos_field.into_inner()),
            block_state_field: JFieldID::from(block_state_field.into_inner()),
            pos_x_field: JFieldID::from(pos_x_field.into_inner()),
            pos_y_field: JFieldID::from(pos_y_field.into_inner()),
            pos_z_field: JFieldID::from(pos_z_field.into_inner()),
        })
    }

    pub fn get_or_insert_block_state<'env>(
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
    use robusta_jni::convert::Field;
    use robusta_jni::jni::sys::jlong;
    use voxels_core::common::Block;
    use voxels_core::stream::mojang::MojangSchematicInputStream;
    use crate::javastreams::JavaInputStream;
    use super::*;

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
            let sis = match schematic_type_str.as_str() {
                "MOJANG" => {
                    MojangSchematicInputStream::new(GzDecoder::new(stream))
                },
                _ => {
                    env.throw_new("java/lang/IllegalArgumentException", "Unknown schematic type")?;
                    return Ok(JObject::null());
                }
            };
            let boxedHandle = Box::new(
                BlockInputStreamHandle {
                    sis: Box::new(sis),
                    jni_cache: JniCache::init(env)?
                }
            );
            let ptr = Box::into_raw(boxedHandle) as jlong;
            let obj = env.new_object("de/richy/voxels/BlockInputStream", "()V", &[])?;
            env.set_field(obj.clone(), "ptr", "J", ptr.into(), )?;
            Ok(obj)
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
            // println!("BlockInputStream read called, ptr value: {}", ptr_value);
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
                            let jstate = handle.jni_cache.get_or_insert_block_state(env, &block.state)?;
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
                            let jstate = handle.jni_cache.get_or_insert_block_state(env, &block.state)?;
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
                    println!("End of stream reached");
                    Ok(-1)
                }
                Err(e) => {
                    println!("Error reading blocks: {}", e);
                    env.throw_new("java/io/IOException", format!("Error reading blocks: {}", e))?;
                    Ok(-1)
                }
            }
        }

        pub extern "jni" fn close(mut self) -> JniResult<()> {
            let ptr_value = self.ptr.get()?;
            println!("BlockInputStream close called, ptr value: {}", ptr_value);
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

    #[cfg(test)]
    mod tests {
        use std::fs;
        use std::process::{Command, Stdio};

        #[test]
        fn test_java() {
            compile_java_test_class();

            let separator = if cfg!(target_os = "windows") {
                ";"
            } else {
                ":"
            };

            let java_classpath = &format!(
                "../target/voxels.jar{}../target/java_test_classes",
                separator
            );

            let status = Command::new("java")
                // inherit stdout and stderr
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .args([
                    "-cp",
                    java_classpath,
                    "-Djava.library.path=../target/release",
                    "TestAll",
                ])
                .status()
                .expect("Failed to run Java test class");

            if !status.success() {
                panic!("Java test class failed!");
            }
        }

        fn compile_java_test_class() {
            let java_src_dir = "java_test";
            let out_dir = "../target/java_test_classes";
            fs::create_dir_all(out_dir).unwrap();
            let status = Command::new("javac")
                .args([
                    "-d",
                    out_dir,
                    "-cp",
                    "../target/voxels.jar",
                    &format!("{}/TestAll.java", java_src_dir),
                ])
                .status()
                .expect("Failed to run javac");

            if !status.success() {
                panic!("Java test class compilation failed: {:?}", status);
            }
        }
    }
}
